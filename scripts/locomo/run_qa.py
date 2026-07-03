#!/usr/bin/env python3
"""F3.4 (QA half, reader): generate LoCoMo answers from retrieved dialogue turns.

Reads a LoCoMo QA input log (each row: question_id, category, question, gold
`answer`, and the top-k `retrieved_turns` with speaker/text/timestamp), builds a
category-aware answer prompt, calls any OpenAI-compatible reader (DeepSeek /
GPT-4o / local), and emits a hypotheses JSONL — one `{question_id, category,
question, gold_answer, hypothesis}` per question — for downstream official
snap-research/locomo per-category F1 scoring.

Two entry points (the F2.2/F3.1/A6.2 self-tested-harness pattern):

  - FAST (offline, `--self-test`): replay the committed 4-category QA fixture
    (`fixtures/benchmarks/locomo/qa_input_log.jsonl`) through the generator in
    `--mock` mode (a deterministic canned answer; NO network) and assert (1) each
    category selects its distinctive task instruction, (2) the assembled prompt
    carries the retrieved turns + question, (3) the adversarial category carries
    abstention guidance, (4) the emitted JSONL has the fields the scorer needs and
    is byte-deterministic on a re-run. No key, no endpoint, no wall clock.

  - REAL: `--input-log` the LoCoMo retrieval-with-text log, `--output` the
    hypotheses JSONL, and a reader (`--api-key`/`--api-key-file`, `--base-url`,
    `--model`). Follow the repo rule: a `--limit 50` subset first, then full 1,986.

Dependency-free (stdlib only); deterministic prompt construction.
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
FIX = REPO / "fixtures" / "benchmarks" / "locomo"
SELFTEST_LOG = FIX / "qa_input_log.jsonl"

DEFAULT_MODEL = "deepseek-v4-flash"
DEFAULT_BASE_URL = "https://api.deepseek.com"
MOCK_ANSWER = "MOCK_ANSWER"


def read_jsonl(path: pathlib.Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


# Category-aware answer instructions. LoCoMo's four evaluated categories each
# reward a distinct answering behaviour; adversarial questions test refusal.
CATEGORY_INSTRUCTIONS = {
    "multi-hop": (
        "This is a multi-hop question. Chain evidence across more than one turn: "
        "resolve each intermediate reference, then state the single final answer."
    ),
    "temporal-reasoning": (
        "This is a temporal-reasoning question. Use each turn's timestamp as its "
        "event date, order events by calendar time, and compute the requested date, "
        "order, or interval directly from the dialogue."
    ),
    "open-domain": (
        "This is an open-domain memory question. Find the most relevant turn and "
        "answer the user's explicit fact concisely, using only the dialogue."
    ),
    "adversarial": (
        "This may be unanswerable from the dialogue. If the required fact is not "
        "present in any turn, reply exactly 'Not mentioned'; do not guess or "
        "fabricate details."
    ),
}
DEFAULT_INSTRUCTION = (
    "Answer the question using only the dialogue turns. If the dialogue is "
    "insufficient, reply exactly 'Not mentioned'."
)


def qa_prompt(row: dict[str, Any]) -> str:
    turns = []
    for index, turn in enumerate(row.get("retrieved_turns", []), start=1):
        text = str(turn.get("text", "")).strip()
        speaker = str(turn.get("speaker", "")).strip()
        timestamp = str(turn.get("timestamp", "")).strip()
        if text:
            turns.append(f"[Turn {index} | {speaker} | {timestamp}] {text}")
    instruction = CATEGORY_INSTRUCTIONS.get(row.get("category", ""), DEFAULT_INSTRUCTION)
    return "\n\n".join(
        [
            "I will give you retrieved dialogue turns between a user and an assistant.",
            instruction,
            "",
            "Dialogue Turns:",
            "\n".join(turns),
            "",
            f"Question: {row['question']}",
            "Answer:",
        ]
    )


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
                    f"qa request failed: http={exc.code} detail={detail}"
                ) from exc
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def generate(
    *,
    rows: list[dict[str, Any]],
    out_path: pathlib.Path,
    mock: bool,
    api_key: str,
    base_url: str,
    model: str,
    max_tokens: int,
    retries: int,
    limit: int | None = None,
) -> dict[str, Any]:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    category_counts: dict[str, int] = {}
    written = 0
    selected = rows[:limit] if limit is not None else rows
    with out_path.open("w", encoding="utf-8") as handle:
        for row in selected:
            category = row.get("category", "unknown")
            prompt = qa_prompt(row)
            if mock:
                hypothesis = MOCK_ANSWER
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
                "question_id": row["question_id"],
                "category": category,
                "question": row["question"],
                "gold_answer": row.get("answer", ""),
                "hypothesis": hypothesis,
            }
            handle.write(json.dumps(entry, ensure_ascii=True, sort_keys=True) + "\n")
            category_counts[category] = category_counts.get(category, 0) + 1
            written += 1
    return {"written": written, "category_counts": category_counts}


def run_real(args: argparse.Namespace) -> int:
    api_key = ""
    if args.api_key_file:
        api_key = pathlib.Path(args.api_key_file).read_text(encoding="utf-8").strip()
    elif args.api_key:
        api_key = args.api_key
    if not api_key:
        print("Set --api-key or --api-key-file for the reader endpoint", file=sys.stderr)
        return 2
    rows = read_jsonl(pathlib.Path(args.input_log))
    summary = generate(
        rows=rows,
        out_path=pathlib.Path(args.output),
        mock=False,
        api_key=api_key,
        base_url=args.base_url,
        model=args.model,
        max_tokens=args.max_tokens,
        retries=args.retries,
        limit=args.limit,
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    print(f"wrote {summary['written']} answers -> {args.output}")
    return 0


def run_self_test() -> int:
    rows = read_jsonl(SELFTEST_LOG)
    failures: list[str] = []

    seen_categories = set()
    for row in rows:
        category = row.get("category", "")
        seen_categories.add(category)
        prompt = qa_prompt(row)
        instruction = CATEGORY_INSTRUCTIONS.get(category)
        if instruction is None:
            failures.append(f"{row['question_id']}: fixture category {category!r} has no instruction")
        elif instruction not in prompt:
            failures.append(f"{row['question_id']}: category {category!r} did not select its instruction")
        if "Dialogue Turns:" not in prompt or str(row["question"]) not in prompt:
            failures.append(f"{row['question_id']}: prompt missing dialogue/question scaffolding")
        if row.get("retrieved_turns") and "[Turn 1 |" not in prompt:
            failures.append(f"{row['question_id']}: prompt missing the retrieved turns")

    # The adversarial branch must instruct exact-abstention (LoCoMo refusal test).
    adv_rows = [r for r in rows if r.get("category") == "adversarial"]
    for row in adv_rows:
        if "Not mentioned" not in qa_prompt(row):
            failures.append(f"{row['question_id']}: adversarial prompt lacks abstention guidance")

    missing = set(CATEGORY_INSTRUCTIONS) - seen_categories
    if missing:
        failures.append(f"fixture does not cover categories: {sorted(missing)}")

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        out_a = pathlib.Path(tmp) / "qa_a.jsonl"
        out_b = pathlib.Path(tmp) / "qa_b.jsonl"
        common = dict(
            rows=rows,
            mock=True,
            api_key="",
            base_url="",
            model=DEFAULT_MODEL,
            max_tokens=8,
            retries=0,
        )
        summary_a = generate(out_path=out_a, **common)
        summary_b = generate(out_path=out_b, **common)
        if out_a.read_bytes() != out_b.read_bytes():
            failures.append("re-run produced different answers bytes (non-deterministic)")
        emitted = read_jsonl(out_a)
        if len(emitted) != len(rows):
            failures.append(f"emitted {len(emitted)} answers for {len(rows)} rows")
        for entry in emitted:
            for field in ("question_id", "category", "hypothesis", "gold_answer"):
                if field not in entry:
                    failures.append(f"emitted entry missing scorer field {field!r}: {entry}")
        if summary_a != summary_b:
            failures.append("generation summary differs across runs")

    if failures:
        print("F3.4 LoCoMo QA reader self-test FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print(
        "F3.4 LoCoMo QA reader self-test passed: "
        f"{len(rows)} rows, {len(seen_categories)} categories "
        f"({', '.join(sorted(seen_categories))}); adversarial abstention wired; "
        "emitted JSONL carries the scorer fields and is byte-deterministic."
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true", help="offline harness check (no endpoint)")
    parser.add_argument("--input-log", help="LoCoMo QA input log JSONL (rows with retrieved_turns)")
    parser.add_argument("--output", help="hypotheses JSONL to write")
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--api-key", default="")
    parser.add_argument("--api-key-file", default="")
    parser.add_argument("--max-tokens", type=int, default=256)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--limit", type=int, default=None, help="cap questions (repo rule: 50 first)")
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()
    missing = [name for name in ("input_log", "output") if not getattr(args, name)]
    if missing:
        parser.error("real QA needs --input-log --output (or pass --self-test)")
    return run_real(args)


if __name__ == "__main__":
    raise SystemExit(main())
