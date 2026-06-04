#!/usr/bin/env python3
"""Score EnterpriseRAG-Bench answers with DeepSeek instead of GPT judges."""

from __future__ import annotations

import argparse
import concurrent.futures
import http.client
import json
import re
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def append_jsonl(path: Path, row: dict[str, Any], lock: threading.Lock) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with lock:
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def by_question_id(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {str(row.get("question_id")): row for row in rows if row.get("question_id")}


def document_metrics(question: dict[str, Any], answer: dict[str, Any]) -> tuple[float | None, int | None]:
    expected = {str(item) for item in question.get("expected_doc_ids", [])}
    retrieved = [str(item) for item in answer.get("document_ids", [])]
    if not expected:
        return None, None
    recall = len(expected & set(retrieved)) / len(expected) * 100.0
    invalid_extra = sum(1 for doc_id in retrieved if doc_id not in expected)
    return round(recall, 2), invalid_extra


def build_prompt(question: dict[str, Any], answer: dict[str, Any]) -> str:
    facts = "\n".join(f"- {fact}" for fact in question.get("answer_facts", []))
    return f"""You are judging an EnterpriseRAG-Bench answer.

Use the gold answer and required facts as the source of truth. Ignore whether
the retrieved documents were good or bad; score only the candidate answer.

Return only strict JSON with this shape:
{{"answer_correct": true|false, "completeness_pct": 0-100, "correctness_reasoning": "short reason"}}

Scoring rules:
- answer_correct is true only when the candidate answers the question without a material contradiction.
- completeness_pct estimates how much of the required facts are present.
- If the candidate says insufficient information while the gold answer contains facts, mark it incorrect.
- If the gold answer says the information is unavailable and the candidate also says that, mark it correct.
- Penalize wrong numbers, dates, file paths, headers, names, IDs, regions, and versions.
- Keep correctness_reasoning under 40 words.

Question:
{question.get("question", "")}

Gold answer:
{question.get("gold_answer", "")}

Required facts:
{facts}

Candidate answer:
{answer.get("answer", "")}
"""


def parse_judge_json(text: str) -> dict[str, Any]:
    cleaned = text.strip()
    if cleaned.startswith("```"):
        cleaned = re.sub(r"^```(?:json)?", "", cleaned).strip()
        cleaned = re.sub(r"```$", "", cleaned).strip()
    match = re.search(r"\{.*\}", cleaned, flags=re.S)
    if match:
        cleaned = match.group(0)
    payload = json.loads(cleaned)
    return {
        "answer_correct": bool(payload.get("answer_correct")),
        "completeness_pct": max(0.0, min(100.0, float(payload.get("completeness_pct", 0.0)))),
        "correctness_reasoning": str(payload.get("correctness_reasoning", "")).strip()[:500],
    }


def chat_json(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    timeout: float,
    retries: int,
) -> tuple[dict[str, Any], dict[str, Any], int]:
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0,
        "max_tokens": 220,
        "thinking": {"type": "disabled"},
    }
    data = json.dumps(payload).encode("utf-8")
    url = base_url.rstrip("/") + "/chat/completions"
    for attempt in range(retries + 1):
        request = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": "Bearer " + api_key,
                "Content-Type": "application/json",
            },
        )
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            content = body["choices"][0]["message"].get("content", "")
            return parse_judge_json(str(content)), body.get("usage", {}), elapsed_ms
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"deepseek judge HTTP {error.code}: {detail}") from error
            time.sleep(min(30, 2**attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"deepseek judge request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def question_type_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        grouped.setdefault(str(row.get("question_type") or "unknown"), []).append(row)
    return {name: aggregate_stats(items, include_total=False) | {"count": len(items)} for name, items in grouped.items()}


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def aggregate_stats(rows: list[dict[str, Any]], *, include_total: bool = True) -> dict[str, Any]:
    recall_values = [float(row["document_recall_pct"]) for row in rows if row.get("document_recall_pct") is not None]
    invalid_values = [float(row["invalid_extra_docs"]) for row in rows if row.get("invalid_extra_docs") is not None]
    correct_pct = sum(1 for row in rows if row.get("answer_correct")) / len(rows) * 100.0 if rows else 0.0
    completeness = mean([float(row.get("completeness_pct") or 0.0) for row in rows])
    stats: dict[str, Any] = {
        "average_correctness_pct": round(correct_pct, 2),
        "average_completeness_pct": completeness,
        "combined_correctness_completeness_score": round(correct_pct * completeness / 100.0, 2),
        "average_recall_pct": mean(recall_values),
        "average_invalid_extra_docs": mean(invalid_values),
    }
    if include_total:
        stats.update(
            {
                "total_questions": len(rows),
                "completed_questions": len(rows),
                "skipped_rows": 0,
                "num_corrected_questions": 0,
            }
        )
    return stats


def run(args: argparse.Namespace) -> dict[str, Any]:
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    questions = by_question_id(read_jsonl(args.questions_file))
    answers = by_question_id(read_jsonl(args.answers_file))
    qids = list(questions.keys())
    if args.limit is not None:
        qids = qids[: args.limit]
    existing = by_question_id(read_jsonl(args.judgments_file))
    output_lock = threading.Lock()
    usage_lock = threading.Lock()
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    pending = [qid for qid in qids if qid not in existing]
    if not pending and args.results_file.exists():
        return read_json(args.results_file)

    def judge(qid: str) -> dict[str, Any]:
        question = questions[qid]
        answer = answers.get(qid, {"question_id": qid, "answer": "", "document_ids": []})
        recall, invalid_extra = document_metrics(question, answer)
        judged, usage, elapsed_ms = chat_json(
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            prompt=build_prompt(question, answer),
            timeout=args.timeout_seconds,
            retries=args.retries,
        )
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        return {
            "question_id": qid,
            "question_type": question.get("question_type"),
            "answer_correct": judged["answer_correct"],
            "completeness_pct": judged["completeness_pct"],
            "correctness_reasoning": judged["correctness_reasoning"],
            "document_recall_pct": recall,
            "invalid_extra_docs": invalid_extra,
            "corrected": False,
            "elapsed_ms": elapsed_ms,
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(judge, qid) for qid in pending]
        for future in concurrent.futures.as_completed(futures):
            row = future.result()
            existing[row["question_id"]] = row
            append_jsonl(args.judgments_file, row, output_lock)
            if args.progress_every and len(existing) % args.progress_every == 0:
                print(f"judged {len(existing)}/{len(qids)}")

    rows = [existing[qid] for qid in qids if qid in existing]
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.deepseek_judge_metrics.v1",
        "judge_model": args.model,
        "judge_provider": "deepseek",
        "thinking": "disabled",
        "answers_file": str(args.answers_file),
        "questions_file": str(args.questions_file),
        "judgments_file": str(args.judgments_file),
        "questions": rows,
        "aggregate_stats": aggregate_stats(rows),
        "question_type_stats": question_type_stats(rows),
        **usage_totals,
    }
    write_json(args.results_file, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--results-file", type=Path, required=True)
    parser.add_argument("--judgments-file", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, required=True)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--timeout-seconds", type=float, default=180.0)
    parser.add_argument("--progress-every", type=int, default=10)
    args = parser.parse_args()
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if args.workers <= 0:
        parser.error("--workers must be positive")
    return args


def main() -> int:
    print(json.dumps(run(parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
