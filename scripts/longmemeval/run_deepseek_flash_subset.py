#!/usr/bin/env python3
"""Run an unofficial DeepSeek check on LongMemEval v1 false cases."""

from __future__ import annotations

import argparse
import json
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_MODEL = "deepseek-v4-flash"
DEFAULT_BASE_URL = "https://api.deepseek.com"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n")


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# DeepSeek Flash LongMemEval False-Case Subset Check",
        "",
        "- official score: `false`",
        f"- model: `{report['model']}`",
        f"- generation thinking: `{report['generation_thinking']}`",
        f"- judge thinking: `{report['judge_thinking']}`",
        f"- questions: `{report['questions']}` baseline GPT-4o false cases",
        f"- correct by DeepSeek judge: `{report['correct']}` / `{report['evaluated']}`",
        f"- accuracy: `{report['accuracy']:.4f}`",
        f"- empty hypotheses: `{report['empty_hypotheses']}`",
        f"- prompt tokens: `{report['usage']['prompt_tokens']}`",
        f"- completion tokens: `{report['usage']['completion_tokens']}`",
        "",
        "## By Question Type",
        "",
        "| Type | Correct | Count | Accuracy |",
        "| --- | ---: | ---: | ---: |",
    ]
    for name, row in report["by_question_type"].items():
        lines.append(
            f"| `{name}` | `{row['correct']}` | `{row['count']}` | `{row['accuracy']:.4f}` |"
        )
    lines += [
        "",
        "This is an unofficial diagnostic check. It uses DeepSeek for both generation "
        "and judging, so it is useful for local iteration but is not comparable to the "
        "official LongMemEval GPT-4o evaluator score.",
    ]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def append_jsonl(path: Path, row: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def load_existing(path: Path) -> dict[str, dict[str, Any]]:
    if not path.exists():
        return {}
    return {row["question_id"]: row for row in read_jsonl(path)}


def chat(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    thinking: str,
    reasoning_effort: str,
    retries: int,
) -> tuple[str, dict[str, Any]]:
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "thinking": {"type": thinking},
    }
    if thinking == "disabled":
        payload["temperature"] = 0
    else:
        payload["reasoning_effort"] = reasoning_effort
    data = json.dumps(payload).encode("utf-8")
    url = base_url.rstrip("/") + "/chat/completions"
    for attempt in range(retries + 1):
        req = urllib.request.Request(
            url,
            data=data,
            headers={"Authorization": "Bearer " + api_key, "Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as response:
                body = json.loads(response.read().decode("utf-8"))
            choice = body["choices"][0]
            message = choice["message"]
            metadata = {
                "finish_reason": choice.get("finish_reason"),
                "reasoning_content_chars": len(str(message.get("reasoning_content") or "")),
                "thinking": thinking,
                "usage": body.get("usage", {}),
            }
            return str(message.get("content", "")).strip(), metadata
        except urllib.error.HTTPError as exc:
            if exc.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = exc.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"chat request failed: http={exc.code} detail={detail}") from exc
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def generation_prompt(row: dict[str, Any], ref: dict[str, Any]) -> str:
    contexts = []
    for index, item in enumerate(row["retrieval_results"].get("ranked_items", []), start=1):
        text = str(item.get("text", "")).strip()
        timestamp = str(item.get("timestamp", "")).strip()
        if text:
            contexts.append(f"[Session {index} | {timestamp}]\n{text}")
    if ref.get("question_type") == "single-session-preference":
        task_instruction = (
            "This is a preference-based personalization question. Use the history to infer the "
            "user's preferences, constraints, brands, interests, style, or prior experiences, then "
            "answer the current question with concrete personalized recommendations. Do not refuse "
            "just because the exact current item is not already in the history; transfer the user's "
            "known preferences to the new recommendation request. If no relevant preference signal "
            "exists at all, say what is missing briefly."
        )
    elif ref.get("question_type") == "single-session-user":
        task_instruction = (
            "This is a single-session user-memory question. Find the one most relevant session and "
            "extract the user's explicit personal fact from it. The answer may be phrased near the "
            "question topic rather than with the exact same words. For where, what, who, when, how "
            "many, and how much questions, return the concise location, name, date, count, amount, "
            "duration, or personal attribute found in the relevant session. Do not refuse merely "
            "because the full question sentence is not repeated verbatim; refuse only when no "
            "relevant fact is present in any provided session. Answer only the field requested by "
            "the question; do not add adjacent gifts, items, events, or explanations unless the "
            "question asks for a list."
        )
    elif ref.get("question_type") == "multi-session":
        task_instruction = (
            "This is a multi-session memory question. Use evidence across all provided sessions, "
            "not just one session. Identify every relevant event, item, amount, date, or place, "
            "reconcile duplicates, and answer the final aggregate, comparison, or count directly. "
            "For count, total, duration, or money questions, compute the result from the listed "
            "facts; do not say the history is insufficient merely because it does not state the "
            "combined total explicitly. If the available sessions truly miss a required fact, state "
            "only the missing fact briefly."
        )
    elif ref.get("question_type") == "temporal-reasoning":
        task_instruction = (
            "This is a temporal-reasoning memory question. Build a small timeline from the provided "
            "sessions before answering. Use each session timestamp as the event date unless the text "
            "gives a more specific date; resolve relative phrases such as yesterday, last week, or "
            "ago relative to that session timestamp or the current date. For first/last/latest/order "
            "questions, sort events by calendar time, not by retrieval rank. For days, weeks, months, "
            "or duration questions, compute the interval from the dates or start/end events; do not "
            "say the history is insufficient merely because the final interval is not written "
            "explicitly. Answer with the concise final date, order, or duration."
        )
    else:
        task_instruction = (
            "Answer the question using only the history. If the history is insufficient, say so."
        )
    return "\n\n".join(
        [
            "I will give you relevant history chats between an assistant and a user.",
            task_instruction,
            "",
            "History Chats:",
            "\n\n".join(contexts),
            "",
            f"Current Date: {row.get('question_date', '')}",
            f"Question: {row['question']}",
            "Answer:",
        ]
    )


def judge_prompt(ref: dict[str, Any], hypothesis: str) -> str:
    qtype = ref["question_type"]
    question = ref["question"]
    answer = ref["answer"]
    if "_abs" in ref["question_id"]:
        return (
            "I will give you an unanswerable question, an explanation, and a response from a model. "
            "Please answer yes if the model correctly identifies the question as unanswerable. "
            "Answer yes or no only.\n\n"
            f"Question: {question}\n\nExplanation: {answer}\n\nModel Response: {hypothesis}\n\n"
            "Does the model correctly identify the question as unanswerable?"
        )
    if qtype == "single-session-preference":
        return (
            "I will give you a question, a rubric for desired personalized response, and a model "
            "response. Answer yes if the response satisfies the desired response. Answer yes or no "
            "only.\n\n"
            f"Question: {question}\n\nRubric: {answer}\n\nModel Response: {hypothesis}\n\n"
            "Is the model response correct?"
        )
    return (
        "I will give you a question, a correct answer, and a model response. Answer yes if the "
        "response contains the correct answer or is equivalent. Otherwise answer no. Answer yes or "
        "no only.\n\n"
        f"Question: {question}\n\nCorrect Answer: {answer}\n\nModel Response: {hypothesis}\n\n"
        "Is the model response correct?"
    )


def run(args: argparse.Namespace) -> dict[str, Any]:
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    retrieval_rows = read_jsonl(args.retrieval_log)
    refs = {row["question_id"]: row for row in read_json(args.reference_file)}
    args.output_root.mkdir(parents=True, exist_ok=True)
    hyp_path = args.output_root / "deepseek_flash_hypotheses.jsonl"
    eval_path = args.output_root / "deepseek_flash_eval.jsonl"
    report_path = args.output_root / "deepseek_flash_report.json"
    existing_hyp = load_existing(hyp_path)
    existing_eval = load_existing(eval_path)
    previous_report = read_json(report_path) if report_path.exists() else {}
    previous_usage = previous_report.get("usage", {}) if isinstance(previous_report, dict) else {}

    prompt_tokens = completion_tokens = 0
    for index, row in enumerate(retrieval_rows, start=1):
        qid = row["question_id"]
        if qid not in existing_hyp:
            content, metadata = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=generation_prompt(row, refs[qid]),
                max_tokens=args.gen_max_tokens,
                thinking=args.generation_thinking,
                reasoning_effort=args.reasoning_effort,
                retries=args.retries,
            )
            existing_hyp[qid] = {
                "question_id": qid,
                "hypothesis": content,
                "generation": {
                    "finish_reason": metadata["finish_reason"],
                    "reasoning_content_chars": metadata["reasoning_content_chars"],
                    "thinking": metadata["thinking"],
                },
            }
            append_jsonl(hyp_path, existing_hyp[qid])
            usage = metadata["usage"]
            prompt_tokens += int(usage.get("prompt_tokens", 0))
            completion_tokens += int(usage.get("completion_tokens", 0))
        if qid not in existing_eval:
            label_text, metadata = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=judge_prompt(refs[qid], existing_hyp[qid]["hypothesis"]),
                max_tokens=args.judge_max_tokens,
                thinking=args.judge_thinking,
                reasoning_effort=args.reasoning_effort,
                retries=args.retries,
            )
            label = "yes" in label_text.lower()
            existing_eval[qid] = {
                **existing_hyp[qid],
                "autoeval_label": {
                    "model": args.model,
                    "label": label,
                    "raw": label_text,
                    "finish_reason": metadata["finish_reason"],
                    "reasoning_content_chars": metadata["reasoning_content_chars"],
                    "thinking": metadata["thinking"],
                },
            }
            append_jsonl(eval_path, existing_eval[qid])
            usage = metadata["usage"]
            prompt_tokens += int(usage.get("prompt_tokens", 0))
            completion_tokens += int(usage.get("completion_tokens", 0))
        if index % 10 == 0:
            print(f"processed {index}/{len(retrieval_rows)}")

    labels = [1 if row["autoeval_label"]["label"] else 0 for row in existing_eval.values()]
    by_type: dict[str, list[int]] = {}
    for qid, row in existing_eval.items():
        by_type.setdefault(refs[qid]["question_type"], []).append(1 if row["autoeval_label"]["label"] else 0)
    report = {
        "schema_version": "cortexdb.longmemeval.v1.deepseek_flash_subset.v1",
        "status": "complete",
        "official_score": False,
        "model": args.model,
        "generation_thinking": args.generation_thinking,
        "judge_thinking": args.judge_thinking,
        "questions": len(retrieval_rows),
        "evaluated": len(labels),
        "correct": sum(labels),
        "accuracy": sum(labels) / len(labels) if labels else 0.0,
        "empty_hypotheses": sum(
            1 for row in existing_hyp.values() if not str(row.get("hypothesis", "")).strip()
        ),
        "by_question_type": {
            key: {"count": len(values), "correct": sum(values), "accuracy": sum(values) / len(values)}
            for key, values in sorted(by_type.items())
        },
        "usage": {
            "prompt_tokens": prompt_tokens + int(previous_usage.get("prompt_tokens", 0)),
            "completion_tokens": completion_tokens + int(previous_usage.get("completion_tokens", 0)),
        },
        "hypotheses": str(hyp_path),
        "eval_results": str(eval_path),
    }
    write_json(report_path, report)
    write_markdown(args.output_root / "deepseek_flash_report.md", report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-log", type=Path, required=True)
    parser.add_argument("--reference-file", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, default=Path("/mnt/hf_model_weights/arman/3bit/.deepseek"))
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--gen-max-tokens", type=int, default=1024)
    parser.add_argument("--judge-max-tokens", type=int, default=64)
    parser.add_argument("--generation-thinking", choices=["enabled", "disabled"], default="disabled")
    parser.add_argument("--judge-thinking", choices=["enabled", "disabled"], default="disabled")
    parser.add_argument("--reasoning-effort", choices=["high", "max"], default="high")
    parser.add_argument("--retries", type=int, default=4)
    return parser.parse_args()


def main() -> int:
    report = run(parse_args())
    print(json.dumps(report, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
