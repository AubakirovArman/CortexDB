#!/usr/bin/env python3
"""Summarize remaining EnterpriseRAG missing-gold bottlenecks.

The gold-missing classifier already assigns one reason per missing gold
document. This script turns those rows into an operator-facing report grouped
by question type, source type, and reason so the next retrieval work can target
the largest remaining discovery failures.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def source_from_path(path: str) -> str:
    return path.split("/", 1)[0] if path else "unknown"


def short_path(path: str, max_len: int = 88) -> str:
    if len(path) <= max_len:
        return path
    return "..." + path[-(max_len - 3) :]


def top_items(counter: Counter[str], limit: int) -> list[dict[str, Any]]:
    return [
        {"key": key, "count": count}
        for key, count in counter.most_common(limit)
    ]


def candidate_rank_bucket(rank: int | None) -> str:
    if rank is None:
        return "missing"
    if rank <= 10:
        return "top10"
    if rank <= 50:
        return "top50"
    if rank <= 100:
        return "top100"
    if rank <= 500:
        return "top500"
    if rank <= 1000:
        return "top1000"
    return "after1000"


def row_source(row: dict[str, Any]) -> str:
    gold_path = str(row.get("gold_path") or "")
    if gold_path:
        return source_from_path(gold_path)
    sources = row.get("source_types")
    if isinstance(sources, list) and sources:
        return str(sources[0])
    return "unknown"


def summarize_rows(rows: list[dict[str, Any]], top_limit: int) -> dict[str, Any]:
    by_reason: Counter[str] = Counter()
    by_type: Counter[str] = Counter()
    by_source: Counter[str] = Counter()
    by_type_reason: Counter[str] = Counter()
    by_source_reason: Counter[str] = Counter()
    by_type_source_reason: Counter[str] = Counter()
    by_rank_bucket: Counter[str] = Counter()
    examples: dict[str, list[dict[str, Any]]] = defaultdict(list)

    for row in rows:
        reason = str(row.get("reason") or "unknown")
        question_type = str(row.get("question_type") or "unknown")
        source = row_source(row)
        rank = row.get("candidate_rank")
        rank_value = rank if isinstance(rank, int) else None
        by_reason[reason] += 1
        by_type[question_type] += 1
        by_source[source] += 1
        by_type_reason[f"{question_type}|{reason}"] += 1
        by_source_reason[f"{source}|{reason}"] += 1
        by_type_source_reason[f"{question_type}|{source}|{reason}"] += 1
        by_rank_bucket[candidate_rank_bucket(rank_value)] += 1
        if len(examples[reason]) < top_limit:
            examples[reason].append(
                {
                    "question_id": row.get("question_id"),
                    "question_type": question_type,
                    "source": source,
                    "candidate_rank": rank_value,
                    "gold_path": row.get("gold_path"),
                    "question": row.get("question"),
                }
            )

    return {
        "by_reason": top_items(by_reason, top_limit),
        "by_question_type": top_items(by_type, top_limit),
        "by_source": top_items(by_source, top_limit),
        "by_question_type_reason": top_items(by_type_reason, top_limit),
        "by_source_reason": top_items(by_source_reason, top_limit),
        "by_question_type_source_reason": top_items(by_type_source_reason, top_limit),
        "by_candidate_rank_bucket": top_items(by_rank_bucket, top_limit),
        "examples_by_reason": dict(sorted(examples.items())),
    }


def split_key(key: str) -> list[str]:
    return key.split("|")


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# EnterpriseRAG Missing-Gold Bottlenecks",
        "",
        f"- details_file: `{report['details_file']}`",
        f"- source_report_file: `{report['source_report_file']}`",
        f"- missing_gold_docs: `{report['missing_gold_docs']}`",
        f"- questions_with_missing_gold: `{report['questions_with_missing_gold']}`",
        "",
        "## Largest Type / Source / Reason Buckets",
        "",
        "| Question Type | Source | Reason | Missing Gold Docs |",
        "| --- | --- | --- | ---: |",
    ]
    for item in report["summary"]["by_question_type_source_reason"]:
        parts = split_key(item["key"])
        while len(parts) < 3:
            parts.append("unknown")
        lines.append(f"| `{parts[0]}` | `{parts[1]}` | `{parts[2]}` | {item['count']} |")

    lines.extend(
        [
            "",
            "## Source / Reason Buckets",
            "",
            "| Source | Reason | Missing Gold Docs |",
            "| --- | --- | ---: |",
        ]
    )
    for item in report["summary"]["by_source_reason"]:
        source, reason = split_key(item["key"])
        lines.append(f"| `{source}` | `{reason}` | {item['count']} |")

    lines.extend(
        [
            "",
            "## Candidate Rank Buckets",
            "",
            "| Candidate Rank Bucket | Missing Gold Docs |",
            "| --- | ---: |",
        ]
    )
    for item in report["summary"]["by_candidate_rank_bucket"]:
        lines.append(f"| `{item['key']}` | {item['count']} |")

    lines.extend(["", "## Example Rows", ""])
    for reason, examples in report["summary"]["examples_by_reason"].items():
        lines.extend([f"### `{reason}`", ""])
        for example in examples[: report["top_limit"]]:
            lines.append(
                "- "
                f"`{example['question_id']}` "
                f"`{example['question_type']}` "
                f"`{example['source']}` "
                f"rank=`{example['candidate_rank']}` "
                f"path=`{short_path(str(example.get('gold_path') or ''))}`"
            )
        lines.append("")

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def run(args: argparse.Namespace) -> dict[str, Any]:
    source_report = read_json(args.source_report)
    details = read_jsonl(args.details_file)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.gold_missing_bottlenecks.v1",
        "details_file": str(args.details_file),
        "source_report_file": str(args.source_report),
        "report_file": str(args.report),
        "markdown_file": str(args.markdown) if args.markdown else None,
        "missing_gold_docs": source_report.get("missing_gold_docs", len(details)),
        "questions_with_missing_gold": source_report.get("questions_with_missing_gold"),
        "top_limit": args.top_limit,
        "summary": summarize_rows(details, args.top_limit),
    }
    write_json(args.report, report)
    if args.markdown:
        write_markdown(args.markdown, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--details-file", type=Path, required=True)
    parser.add_argument("--source-report", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--top-limit", type=int, default=12)
    args = parser.parse_args()
    if args.top_limit < 1:
        parser.error("--top-limit must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "missing_gold_docs": report["missing_gold_docs"],
                "questions_with_missing_gold": report["questions_with_missing_gold"],
                "top_buckets": report["summary"]["by_question_type_source_reason"][:5],
                "report": str(report.get("report_file", "")),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
