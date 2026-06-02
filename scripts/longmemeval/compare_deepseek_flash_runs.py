#!/usr/bin/env python3
"""Compare two DeepSeek flash LongMemEval diagnostic runs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def load_eval(root: Path) -> dict[str, dict[str, Any]]:
    return {row["question_id"]: row for row in read_jsonl(root / "deepseek_flash_eval.jsonl")}


def load_hypotheses(root: Path) -> dict[str, dict[str, Any]]:
    return {row["question_id"]: row for row in read_jsonl(root / "deepseek_flash_hypotheses.jsonl")}


def is_correct(row: dict[str, Any]) -> bool:
    return bool(row.get("autoeval_label", {}).get("label"))


def is_empty(row: dict[str, Any]) -> bool:
    return not str(row.get("hypothesis", "")).strip()


def compact_row(ref: dict[str, Any], old_row: dict[str, Any], new_row: dict[str, Any]) -> dict[str, Any]:
    return {
        "question_id": ref["question_id"],
        "question_type": ref["question_type"],
        "question": ref["question"],
        "answer": ref["answer"],
        "old_correct": is_correct(old_row),
        "new_correct": is_correct(new_row),
        "old_empty": is_empty(old_row),
        "new_empty": is_empty(new_row),
    }


def empty_count(rows: dict[str, dict[str, Any]]) -> int:
    return sum(1 for row in rows.values() if is_empty(row))


def run(args: argparse.Namespace) -> dict[str, Any]:
    refs = {row["question_id"]: row for row in read_json(args.reference_file)}
    old_eval = load_eval(args.old_root)
    new_eval = load_eval(args.new_root)
    old_hyp = load_hypotheses(args.old_root)
    new_hyp = load_hypotheses(args.new_root)
    missing = sorted((set(refs) - set(old_eval)) | (set(refs) - set(new_eval)))
    if missing:
        raise RuntimeError(f"missing eval rows for question IDs: {missing[:10]}")

    transitions: dict[str, list[dict[str, Any]]] = {
        "both_correct": [],
        "both_wrong": [],
        "new_only_correct": [],
        "old_only_correct": [],
    }
    by_type: dict[str, dict[str, int]] = {}
    for qid, ref in refs.items():
        old_row = {**old_hyp.get(qid, {}), **old_eval[qid]}
        new_row = {**new_hyp.get(qid, {}), **new_eval[qid]}
        old_correct = is_correct(old_row)
        new_correct = is_correct(new_row)
        if old_correct and new_correct:
            bucket = "both_correct"
        elif old_correct:
            bucket = "old_only_correct"
        elif new_correct:
            bucket = "new_only_correct"
        else:
            bucket = "both_wrong"
        row = compact_row(ref, old_row, new_row)
        transitions[bucket].append(row)
        counts = by_type.setdefault(
            ref["question_type"],
            {"count": 0, "both_correct": 0, "both_wrong": 0, "new_only_correct": 0, "old_only_correct": 0},
        )
        counts["count"] += 1
        counts[bucket] += 1

    old_report = read_json(args.old_root / "deepseek_flash_report.json")
    new_report = read_json(args.new_root / "deepseek_flash_report.json")
    report = {
        "schema_version": "cortexdb.longmemeval.v1.deepseek_flash_diff.v1",
        "old_run": str(args.old_root),
        "new_run": str(args.new_root),
        "old_model": old_report.get("model"),
        "new_model": new_report.get("model"),
        "old_accuracy": old_report.get("accuracy"),
        "new_accuracy": new_report.get("accuracy"),
        "old_correct": old_report.get("correct"),
        "new_correct": new_report.get("correct"),
        "evaluated": len(refs),
        "old_empty_hypotheses": empty_count(old_hyp),
        "new_empty_hypotheses": empty_count(new_hyp),
        "transition_counts": {key: len(value) for key, value in transitions.items()},
        "by_question_type": by_type,
        "token_usage": {
            "old": old_report.get("usage", {}),
            "new": new_report.get("usage", {}),
        },
        "transitions": transitions,
    }
    args.output_root.mkdir(parents=True, exist_ok=True)
    write_json(args.output_root / "deepseek_flash_diff_report.json", report)
    write_markdown(args.output_root / "deepseek_flash_diff_report.md", report)
    return report


def write_json(path: Path, payload: Any) -> None:
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# DeepSeek Flash Diagnostic Diff",
        "",
        f"- old run: `{report['old_run']}`",
        f"- new run: `{report['new_run']}`",
        f"- old correct: `{report['old_correct']}` / `{report['evaluated']}`",
        f"- new correct: `{report['new_correct']}` / `{report['evaluated']}`",
        f"- old accuracy: `{report['old_accuracy']:.4f}`",
        f"- new accuracy: `{report['new_accuracy']:.4f}`",
        f"- old empty hypotheses: `{report['old_empty_hypotheses']}`",
        f"- new empty hypotheses: `{report['new_empty_hypotheses']}`",
        "",
        "## Transition Counts",
        "",
        "| Transition | Count |",
        "| --- | ---: |",
    ]
    for key, count in report["transition_counts"].items():
        lines.append(f"| `{key}` | `{count}` |")
    lines += [
        "",
        "## By Question Type",
        "",
        "| Type | Count | Both correct | Both wrong | New only | Old only |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for qtype, row in sorted(report["by_question_type"].items()):
        lines.append(
            f"| `{qtype}` | `{row['count']}` | `{row['both_correct']}` | `{row['both_wrong']}` | "
            f"`{row['new_only_correct']}` | `{row['old_only_correct']}` |"
        )
    lines += ["", "## New Only Correct", ""]
    lines.extend(format_ids(report["transitions"]["new_only_correct"]))
    lines += ["", "## Old Only Correct", ""]
    lines.extend(format_ids(report["transitions"]["old_only_correct"]))
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def format_ids(rows: list[dict[str, Any]]) -> list[str]:
    if not rows:
        return ["- none"]
    return [f"- `{row['question_id']}` `{row['question_type']}`: {row['question']}" for row in rows]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--old-root", type=Path, required=True)
    parser.add_argument("--new-root", type=Path, required=True)
    parser.add_argument("--reference-file", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    report = run(parse_args())
    summary = {
        "old_correct": report["old_correct"],
        "new_correct": report["new_correct"],
        "old_empty_hypotheses": report["old_empty_hypotheses"],
        "new_empty_hypotheses": report["new_empty_hypotheses"],
        "transition_counts": report["transition_counts"],
    }
    print(json.dumps(summary, ensure_ascii=True, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
