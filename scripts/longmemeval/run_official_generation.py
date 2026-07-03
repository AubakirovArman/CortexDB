#!/usr/bin/env python3
"""A6.2: type-aware official LongMemEval generation over the retrieval log.

Ports the type-aware ``generation_prompt()`` branching (verbatim from
``run_deepseek_flash_subset.py:126-191``) into an *official* generator that calls
any OpenAI-compatible endpoint (GPT-4o for the official run) and emits a
hypotheses JSONL that the **untouched** official ``evaluate_qa.py`` consumes:
each line is ``{"question_id", "hypothesis", ...}`` and is matched to the
reference by ``question_id`` (see
``target/external-benchmarks/longmemeval/src/evaluation/evaluate_qa.py``).

Two entry points (the F2.2/F3.1/F5.2 self-tested-harness pattern):

  - FAST (offline, this file's ``--self-test``): replay a committed 5-row
    generation fixture (one row per prompt branch) through the generator in
    ``--mock`` mode (a deterministic canned hypothesis; NO network), and assert
    (1) each question type selects its distinctive task instruction, (2) the
    assembled prompt carries the retrieved sessions + Current Date + question,
    (3) the emitted JSONL is ``evaluate_qa.py``-compatible (``question_id`` +
    ``hypothesis`` present, question_ids ⊆ reference), and (4) the whole pass is
    byte-deterministic on a re-run. No key, no endpoint, no wall clock.

  - REAL: point ``--retrieval-log`` at an official retrieval log and
    ``--reference-file`` at the official reference; set ``--api-key`` (or
    ``--api-key-file``) + ``--base-url`` + ``--model gpt-4o-2024-08-06``. The
    generator streams hypotheses JSONL for the official evaluator.

Dependency-free (stdlib only); deterministic prompt construction; the prompt
branching is identical to the DeepSeek diagnostic, so a type-aware win there is
reproduced on the official endpoint.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
import time
import urllib.error
import urllib.request
from typing import Any

REPO = pathlib.Path(__file__).resolve().parents[2]
FIX = REPO / "fixtures" / "benchmarks" / "longmemeval"
SELFTEST_LOG = FIX / "generation_input_log.jsonl"
SELFTEST_REFERENCE = FIX / "generation_reference.jsonl"

DEFAULT_MODEL = "gpt-4o-2024-08-06"
MOCK_HYPOTHESIS = "MOCK_HYPOTHESIS"


def read_jsonl(path: pathlib.Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def read_reference(path: pathlib.Path) -> dict[str, dict[str, Any]]:
    """Read the reference as a JSON array or JSONL, keyed by question_id.

    Matches evaluate_qa.py's own tolerant loader (it accepts either shape).
    """
    text = path.read_text(encoding="utf-8")
    try:
        rows = json.loads(text)
        if isinstance(rows, dict):
            rows = [rows]
    except json.JSONDecodeError:
        rows = [json.loads(line) for line in text.splitlines() if line.strip()]
    return {row["question_id"]: row for row in rows}


# ---------------------------------------------------------------------------
# Ported VERBATIM from run_deepseek_flash_subset.py:126-191 (the type-aware
# branching is the A6.2 IP; keep it byte-identical so a diagnostic win carries
# over to the official endpoint).
# ---------------------------------------------------------------------------
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


# The distinctive substring proving each branch fired (a stable slice of each
# task instruction above). Used by the self-test.
BRANCH_MARKERS = {
    "single-session-preference": "preference-based personalization question",
    "single-session-user": "single-session user-memory question",
    "multi-session": "multi-session memory question",
    "temporal-reasoning": "temporal-reasoning memory question",
    "knowledge-update": "Answer the question using only the history.",
}


def chat(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    retries: int,
) -> str:
    """Minimal OpenAI-compatible chat/completions call (temperature 0)."""
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0,
    }
    data = json.dumps(payload).encode("utf-8")
    url = base_url.rstrip("/") + "/chat/completions"
    for attempt in range(retries + 1):
        req = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": "Bearer " + api_key,
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as response:
                body = json.loads(response.read().decode("utf-8"))
            return str(body["choices"][0]["message"].get("content", "")).strip()
        except urllib.error.HTTPError as exc:
            if exc.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = exc.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(
                    f"generation request failed: http={exc.code} detail={detail}"
                ) from exc
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def generate(
    *,
    rows: list[dict[str, Any]],
    refs: dict[str, dict[str, Any]],
    out_path: pathlib.Path,
    mock: bool,
    api_key: str,
    base_url: str,
    model: str,
    max_tokens: int,
    retries: int,
) -> dict[str, Any]:
    """Build the type-aware prompt per row and emit a hypotheses JSONL.

    Returns a small deterministic summary (for the self-test / report).
    """
    out_path.parent.mkdir(parents=True, exist_ok=True)
    branch_counts: dict[str, int] = {}
    written = 0
    with out_path.open("w", encoding="utf-8") as handle:
        for row in rows:
            qid = row["question_id"]
            ref = refs.get(qid, {})
            qtype = ref.get("question_type", "unknown")
            prompt = generation_prompt(row, ref)
            if mock:
                hypothesis = MOCK_HYPOTHESIS
            else:
                hypothesis = chat(
                    api_key=api_key,
                    base_url=base_url,
                    model=model,
                    prompt=prompt,
                    max_tokens=max_tokens,
                    retries=retries,
                )
            entry = {
                "question_id": qid,
                "question_type": qtype,
                "hypothesis": hypothesis,
            }
            handle.write(json.dumps(entry, ensure_ascii=True, sort_keys=True) + "\n")
            branch_counts[qtype] = branch_counts.get(qtype, 0) + 1
            written += 1
    return {"written": written, "branch_counts": branch_counts}


def run_real(args: argparse.Namespace) -> int:
    api_key = ""
    if args.api_key_file:
        api_key = pathlib.Path(args.api_key_file).read_text(encoding="utf-8").strip()
    elif args.api_key:
        api_key = args.api_key
    if not api_key:
        print("Set --api-key or --api-key-file for the official endpoint", file=sys.stderr)
        return 2
    rows = read_jsonl(pathlib.Path(args.retrieval_log))
    refs = read_reference(pathlib.Path(args.reference_file))
    summary = generate(
        rows=rows,
        refs=refs,
        out_path=pathlib.Path(args.output),
        mock=False,
        api_key=api_key,
        base_url=args.base_url,
        model=args.model,
        max_tokens=args.max_tokens,
        retries=args.retries,
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    print(f"wrote {summary['written']} hypotheses -> {args.output}")
    return 0


def run_self_test() -> int:
    rows = read_jsonl(SELFTEST_LOG)
    refs = read_reference(SELFTEST_REFERENCE)
    failures: list[str] = []

    # (1) Every branch is exercised by the fixture, and each row's prompt selects
    #     the distinctive task instruction for its question type.
    seen_types = set()
    for row in rows:
        qid = row["question_id"]
        ref = refs.get(qid)
        if ref is None:
            failures.append(f"{qid}: missing from reference fixture")
            continue
        qtype = ref["question_type"]
        seen_types.add(qtype)
        prompt = generation_prompt(row, ref)
        marker = BRANCH_MARKERS.get(qtype)
        if marker is None:
            failures.append(f"{qid}: fixture question_type {qtype!r} has no branch marker")
        elif marker not in prompt:
            failures.append(f"{qid}: type {qtype!r} did not select its task instruction")
        # (2) The assembled prompt carries the retrieved sessions + Current Date + question.
        if "History Chats:" not in prompt or "Current Date:" not in prompt:
            failures.append(f"{qid}: prompt missing history/date scaffolding")
        if str(row["question"]) not in prompt:
            failures.append(f"{qid}: prompt missing the question text")
        ranked = row["retrieval_results"].get("ranked_items", [])
        if ranked and "[Session 1 |" not in prompt:
            failures.append(f"{qid}: prompt missing the retrieved session context")

    missing_branches = set(BRANCH_MARKERS) - seen_types
    if missing_branches:
        failures.append(f"fixture does not cover branches: {sorted(missing_branches)}")

    # (3) Emitted JSONL is evaluate_qa.py-compatible, run twice for determinism.
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        out_a = pathlib.Path(tmp) / "hyp_a.jsonl"
        out_b = pathlib.Path(tmp) / "hyp_b.jsonl"
        common = dict(
            rows=rows,
            refs=refs,
            mock=True,
            api_key="",
            base_url="",
            model=DEFAULT_MODEL,
            max_tokens=8,
            retries=0,
        )
        summary_a = generate(out_path=out_a, **common)
        summary_b = generate(out_path=out_b, **common)
        bytes_a = out_a.read_bytes()
        bytes_b = out_b.read_bytes()
        if bytes_a != bytes_b:
            failures.append("re-run produced different hypotheses bytes (non-deterministic)")
        emitted = read_jsonl(out_a)
        if len(emitted) != len(rows):
            failures.append(f"emitted {len(emitted)} hypotheses for {len(rows)} rows")
        for entry in emitted:
            # evaluate_qa.py reads exactly these two fields.
            if "question_id" not in entry or "hypothesis" not in entry:
                failures.append(f"emitted entry missing evaluate_qa fields: {entry}")
            if entry["question_id"] not in refs:
                failures.append(f"emitted question_id {entry['question_id']!r} not in reference")
        if summary_a != summary_b:
            failures.append("generation summary differs across runs")

    if failures:
        print("A6.2 type-aware generation self-test FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print(
        "A6.2 type-aware generation self-test passed: "
        f"{len(rows)} rows, {len(seen_types)} branches "
        f"({', '.join(sorted(seen_types))}); "
        "emitted JSONL is evaluate_qa.py-compatible and byte-deterministic."
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run the hermetic offline harness check (no endpoint, no key)",
    )
    parser.add_argument("--retrieval-log", help="official retrieval log JSONL (rows)")
    parser.add_argument("--reference-file", help="official reference (JSON array or JSONL)")
    parser.add_argument("--output", help="hypotheses JSONL to write")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--base-url", default="https://api.openai.com/v1")
    parser.add_argument("--api-key", default="")
    parser.add_argument("--api-key-file", default="")
    parser.add_argument("--max-tokens", type=int, default=512)
    parser.add_argument("--retries", type=int, default=3)
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()
    missing = [
        name
        for name in ("retrieval_log", "reference_file", "output")
        if not getattr(args, name)
    ]
    if missing:
        parser.error(
            "real generation needs --retrieval-log --reference-file --output "
            "(or pass --self-test)"
        )
    return run_real(args)


if __name__ == "__main__":
    raise SystemExit(main())
