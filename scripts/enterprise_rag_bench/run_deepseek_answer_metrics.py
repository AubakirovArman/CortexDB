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
import urllib.parse
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from progress_logging import ProgressLogger

DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"
LOGGER = ProgressLogger("judge-runner")


def log(message: str) -> None:
    LOGGER.log(message)


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
    if not cleaned:
        return {
            "answer_correct": False,
            "completeness_pct": 0.0,
            "correctness_reasoning": "judge returned empty response",
        }

    if cleaned.startswith("```"):
        cleaned = re.sub(r"^```(?:json)?", "", cleaned).strip()
        cleaned = re.sub(r"```$", "", cleaned).strip()

    candidates = re.findall(r"\{.*?\}", cleaned, flags=re.S)
    if not candidates:
        return {
            "answer_correct": False,
            "completeness_pct": 0.0,
            "correctness_reasoning": "judge response had no JSON object",
        }

    last_error: Exception | None = None
    for candidate in candidates:
        try:
            payload = json.loads(candidate)
            return {
                "answer_correct": bool(payload.get("answer_correct")),
                "completeness_pct": max(0.0, min(100.0, float(payload.get("completeness_pct", 0.0)))),
                "correctness_reasoning": str(payload.get("correctness_reasoning", "")).strip()[:500],
            }
        except json.JSONDecodeError as error:
            last_error = error
            continue

    return {
        "answer_correct": False,
        "completeness_pct": 0.0,
        "correctness_reasoning": (
            f"judge response parse error: {last_error}; raw={cleaned[:500]!r}"
            if last_error is not None
            else "judge response parse error"
        ),
    }


def chat_json(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    timeout: float,
    retries: int,
    omit_thinking_field: bool,
    gemini_native: bool,
    gemini_thinking_budget: int,
    openai_reasoning: bool = False,
) -> tuple[dict[str, Any], dict[str, Any], int]:
    if gemini_native:
        return chat_json_gemini_native(
            api_key=api_key,
            base_url=base_url,
            model=model,
            prompt=prompt,
            timeout=timeout,
            retries=retries,
            thinking_budget=gemini_thinking_budget,
        )
    if openai_reasoning:
        # GPT-5 reasoning models require max_completion_tokens, reject a custom
        # temperature, and bill reasoning tokens against the completion budget.
        # Use minimal reasoning for a cheap, near-deterministic JSON verdict and
        # leave generous headroom so the JSON is not truncated by reasoning.
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_completion_tokens": 3000,
            "reasoning_effort": "none",
        }
    else:
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": 0,
            "max_tokens": 220,
        }
        if not omit_thinking_field:
            payload["thinking"] = {"type": "disabled"}
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


def chat_json_gemini_native(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    timeout: float,
    retries: int,
    thinking_budget: int,
) -> tuple[dict[str, Any], dict[str, Any], int]:
    model_name = model.removeprefix("models/")
    payload = {
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": 0,
            "maxOutputTokens": 220,
            "thinkingConfig": {"thinkingBudget": thinking_budget},
        },
    }
    data = json.dumps(payload).encode("utf-8")
    url = (
        base_url.rstrip("/")
        + f"/models/{urllib.parse.quote(model_name, safe='')}:generateContent"
        + "?key="
        + urllib.parse.quote(api_key)
    )
    for attempt in range(retries + 1):
        request = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            content = "".join(
                str(part.get("text", ""))
                for candidate in body.get("candidates", [])
                for part in candidate.get("content", {}).get("parts", [])
            )
            usage = body.get("usageMetadata", {})
            normalized_usage = {
                "prompt_tokens": int(usage.get("promptTokenCount", 0) or 0),
                "completion_tokens": int(usage.get("candidatesTokenCount", 0) or 0),
                "total_tokens": int(usage.get("totalTokenCount", 0) or 0),
                "thoughts_tokens": int(usage.get("thoughtsTokenCount", 0) or 0),
            }
            return parse_judge_json(content), normalized_usage, elapsed_ms
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"gemini judge HTTP {error.code}: {detail}") from error
            time.sleep(min(30, 2**attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"gemini judge request failed: {error}") from error
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
    combined = mean(
        [
            float(row.get("completeness_pct") or 0.0) if row.get("answer_correct") else 0.0
            for row in rows
        ]
    )
    stats: dict[str, Any] = {
        "average_correctness_pct": round(correct_pct, 2),
        "average_completeness_pct": completeness,
        "combined_correctness_completeness_score": combined,
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
    global LOGGER
    LOGGER = ProgressLogger(
        "judge-runner",
        log_file=getattr(args, "log_file", None),
        status_file=getattr(args, "status_file", None),
    )
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    questions = by_question_id(read_jsonl(args.questions_file))
    answers = by_question_id(read_jsonl(args.answers_file))
    qids = list(questions.keys())
    if args.limit is not None:
        qids = qids[: args.limit]
    existing = by_question_id(read_jsonl(args.judgments_file))
    output_lock = threading.Lock()
    usage_lock = threading.Lock()
    progress_lock = threading.Lock()
    completed_counter = {"value": len(existing)}
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    started = time.perf_counter()
    pending = [qid for qid in qids if qid not in existing]
    log(
        "loaded judge run "
        f"questions={len(qids)} existing={len(existing)} pending={len(pending)} "
        f"workers={args.workers} model={args.model}"
    )
    LOGGER.progress(
        stage="judging",
        state="running",
        completed=len(existing),
        total=len(qids),
        unit="questions",
        total_questions=len(qids),
        existing_questions=len(existing),
        pending_questions=len(pending),
        completed_questions=len(existing),
        prompt_tokens=0,
        completion_tokens=0,
        total_tokens=0,
        results_file=str(args.results_file),
        judgments_file=str(args.judgments_file),
    )
    if not pending and args.results_file.exists():
        log(f"nothing pending; reuse results {args.results_file}")
        LOGGER.progress(
            stage="judging",
            state="done",
            completed=len(existing),
            total=len(qids),
            unit="questions",
            total_questions=len(qids),
            completed_questions=len(existing),
            pending_questions=0,
            results_file=str(args.results_file),
            judgments_file=str(args.judgments_file),
        )
        return read_json(args.results_file)

    def completed_count() -> int:
        with progress_lock:
            return completed_counter["value"]

    def judge(qid: str) -> dict[str, Any]:
        question = questions[qid]
        answer = answers.get(qid, {"question_id": qid, "answer": "", "document_ids": []})
        recall, invalid_extra = document_metrics(question, answer)
        log(
            "question start judge "
            f"question_id={qid} doc_recall={recall} invalid_extra_docs={invalid_extra}"
        )
        LOGGER.status(
            stage="judging",
            state="running",
            active_step="judge_question",
            active_question_id=qid,
            active_document_recall_pct=recall,
            active_invalid_extra_docs=invalid_extra,
            model=args.model,
            completed_questions=completed_count(),
            total_questions=len(qids),
            pending_questions=max(0, len(qids) - completed_count()),
            prompt_tokens=usage_totals["prompt_tokens"],
            completion_tokens=usage_totals["completion_tokens"],
            total_tokens=usage_totals["total_tokens"],
        )
        try:
            judged, usage, elapsed_ms = chat_json(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=build_prompt(question, answer),
                timeout=args.timeout_seconds,
                retries=args.retries,
                omit_thinking_field=args.omit_thinking_field,
                gemini_native=args.gemini_native,
                gemini_thinking_budget=args.gemini_thinking_budget,
                openai_reasoning=getattr(args, "openai_reasoning", False),
            )
        except Exception as error:
            log(f"question failed judge question_id={qid} error={error}")
            LOGGER.status(
                stage="judging",
                state="failed",
                active_step="judge_question",
                active_question_id=qid,
                failed_question_id=qid,
                error=str(error),
                completed_questions=completed_count(),
                total_questions=len(qids),
                pending_questions=max(0, len(qids) - completed_count()),
            )
            raise
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        log(
            "question done judge "
            f"question_id={qid} answer_correct={judged['answer_correct']} "
            f"completeness_pct={judged['completeness_pct']} elapsed_ms={elapsed_ms} "
            f"prompt_tokens={int(usage.get('prompt_tokens', 0) or 0)} "
            f"completion_tokens={int(usage.get('completion_tokens', 0) or 0)} "
            f"total_tokens={int(usage.get('total_tokens', 0) or 0)}"
        )
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
            "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
            "total_tokens": int(usage.get("total_tokens", 0) or 0),
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(judge, qid) for qid in pending]
        log(f"queued judge jobs pending={len(pending)} workers={args.workers}")
        LOGGER.status(
            stage="judging",
            state="running",
            active_step="queued_judge_jobs",
            queued_questions=len(pending),
            workers=args.workers,
            completed_questions=completed_count(),
            total_questions=len(qids),
            pending_questions=max(0, len(qids) - completed_count()),
        )
        for future in concurrent.futures.as_completed(futures):
            row = future.result()
            existing[row["question_id"]] = row
            append_jsonl(args.judgments_file, row, output_lock)
            with progress_lock:
                completed_counter["value"] = len(existing)
                completed = completed_counter["value"]
            should_log = (
                (args.progress_every and completed % args.progress_every == 0)
                or completed == len(qids)
            )
            if should_log:
                LOGGER.progress(
                    stage="judging",
                    state="running",
                    completed=completed,
                    total=len(qids),
                    unit="questions",
                    total_questions=len(qids),
                    completed_questions=completed,
                    pending_questions=max(0, len(qids) - completed),
                    prompt_tokens=usage_totals["prompt_tokens"],
                    completion_tokens=usage_totals["completion_tokens"],
                    total_tokens=usage_totals["total_tokens"],
                    last_question_id=str(row["question_id"]),
                )
            else:
                LOGGER.status(
                    stage="judging",
                    state="running",
                    total_questions=len(qids),
                    completed_questions=completed,
                    pending_questions=max(0, len(qids) - completed),
                    prompt_tokens=usage_totals["prompt_tokens"],
                    completion_tokens=usage_totals["completion_tokens"],
                    total_tokens=usage_totals["total_tokens"],
                    elapsed_seconds=round(time.perf_counter() - started, 1),
                    last_question_id=str(row["question_id"]),
                )

    rows = [existing[qid] for qid in qids if qid in existing]
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.local_judge_metrics.v2",
        "judge_model": args.model,
        "judge_provider": "gemini" if args.gemini_native else "openai_compatible",
        "thinking": (
            f"gemini_budget_{args.gemini_thinking_budget}"
            if args.gemini_native
            else "omitted"
            if args.omit_thinking_field
            else "disabled"
        ),
        "answers_file": str(args.answers_file),
        "questions_file": str(args.questions_file),
        "judgments_file": str(args.judgments_file),
        "questions": rows,
        "aggregate_stats": aggregate_stats(rows),
        "question_type_stats": question_type_stats(rows),
        **usage_totals,
    }
    write_json(args.results_file, report)
    LOGGER.progress(
        stage="judging",
        state="done",
        completed=len(rows),
        total=len(qids),
        unit="questions",
        total_questions=len(qids),
        completed_questions=len(rows),
        pending_questions=max(0, len(qids) - len(rows)),
        prompt_tokens=usage_totals["prompt_tokens"],
        completion_tokens=usage_totals["completion_tokens"],
        total_tokens=usage_totals["total_tokens"],
        results_file=str(args.results_file),
        judgments_file=str(args.judgments_file),
        overall=report["aggregate_stats"].get("combined_correctness_completeness_score"),
    )
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
    parser.add_argument(
        "--omit-thinking-field",
        action="store_true",
        help="Do not send the DeepSeek-specific thinking field; required by some OpenAI-compatible APIs.",
    )
    parser.add_argument("--gemini-native", action="store_true", help="Use Gemini native generateContent API.")
    parser.add_argument("--gemini-thinking-budget", type=int, default=0)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if args.workers <= 0:
        parser.error("--workers must be positive")
    return args


def main() -> int:
    try:
        print(json.dumps(run(parse_args()), sort_keys=True))
        return 0
    except Exception as error:
        LOGGER.status(stage="judging", state="failed", error=str(error))
        raise


if __name__ == "__main__":
    raise SystemExit(main())
