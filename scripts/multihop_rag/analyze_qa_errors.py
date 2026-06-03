#!/usr/bin/env python3
"""Summarize MultiHop-RAG QA misses by question type."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


WORD_RE = re.compile(r"[A-Za-z0-9]+")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def normalize(value: str) -> str:
    return " ".join(WORD_RE.findall(value.lower()))


def words(value: str) -> set[str]:
    return set(WORD_RE.findall(value.lower()))


def official_like_hit(predicted: str, gold: str) -> bool:
    return bool(words(predicted) & words(gold))


def is_insufficient(value: str) -> bool:
    return normalize(value) in {"insufficient information", "insufficient info"}


def sample_row(row: dict[str, Any], reason: str) -> dict[str, Any]:
    return {
        "reason": reason,
        "question_type": row.get("question_type", ""),
        "query": row.get("query", ""),
        "model_answer": row.get("model_answer", ""),
        "gold_answer": row.get("gold_answer", ""),
    }


def analyze(rows: list[dict[str, Any]], sample_limit: int) -> dict[str, Any]:
    by_type: dict[str, Counter[str]] = defaultdict(Counter)
    samples: dict[str, list[dict[str, Any]]] = defaultdict(list)
    overall = Counter()
    for row in rows:
        qtype = str(row.get("question_type", "unknown"))
        predicted = str(row.get("model_answer", ""))
        gold = str(row.get("gold_answer", ""))
        hit = official_like_hit(predicted.lower(), gold.lower())
        exact = normalize(predicted) == normalize(gold)
        false_abstain = is_insufficient(predicted) and not is_insufficient(gold)
        missed_null = is_insufficient(gold) and not is_insufficient(predicted)
        reason = "hit" if hit else "miss"
        if false_abstain:
            reason = "false_abstain"
        elif missed_null:
            reason = "missed_null"
        elif exact:
            reason = "exact"
        by_type[qtype]["total"] += 1
        by_type[qtype]["hit"] += int(hit)
        by_type[qtype]["exact"] += int(exact)
        by_type[qtype]["miss"] += int(not hit)
        by_type[qtype]["false_abstain"] += int(false_abstain)
        by_type[qtype]["missed_null"] += int(missed_null)
        overall["total"] += 1
        overall["hit"] += int(hit)
        overall["exact"] += int(exact)
        overall["miss"] += int(not hit)
        overall["false_abstain"] += int(false_abstain)
        overall["missed_null"] += int(missed_null)
        if reason != "hit" and len(samples[qtype]) < sample_limit:
            samples[qtype].append(sample_row(row, reason))

    def materialize(counter: Counter[str]) -> dict[str, Any]:
        total = counter["total"]
        return {
            "total": total,
            "official_like_hits": counter["hit"],
            "exact_matches": counter["exact"],
            "misses": counter["miss"],
            "false_abstentions": counter["false_abstain"],
            "missed_nulls": counter["missed_null"],
            "official_like_hit_rate": round(counter["hit"] / total, 4) if total else 0.0,
            "exact_match_rate": round(counter["exact"] / total, 4) if total else 0.0,
        }

    return {
        "schema_version": "cortexdb.multihop_rag.qa_error_analysis.v1",
        "overall": materialize(overall),
        "by_type": {qtype: materialize(counter) for qtype, counter in sorted(by_type.items())},
        "samples": dict(samples),
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# MultiHop-RAG QA Error Analysis",
        "",
        "This report uses the same word-overlap hit rule as the official QA script,",
        "plus exact-match and abstention diagnostics for tuning.",
        "",
        "## Overall",
        "",
        "| Total | Hits | Misses | Exact | False abstentions | Missed nulls |",
        "| ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    overall = report["overall"]
    lines.append(
        f"| {overall['total']} | {overall['official_like_hits']} | {overall['misses']} | "
        f"{overall['exact_matches']} | {overall['false_abstentions']} | {overall['missed_nulls']} |"
    )
    lines.extend(
        [
            "",
            "## By Type",
            "",
            "| Type | Total | Hit rate | Exact rate | False abstentions | Missed nulls |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for qtype, stats in report["by_type"].items():
        lines.append(
            f"| `{qtype}` | {stats['total']} | {stats['official_like_hit_rate']:.4f} | "
            f"{stats['exact_match_rate']:.4f} | {stats['false_abstentions']} | {stats['missed_nulls']} |"
        )
    lines.extend(["", "## Miss Samples", ""])
    for qtype, samples in report["samples"].items():
        lines.append(f"### {qtype}")
        lines.append("")
        for sample in samples:
            lines.append(f"- reason: `{sample['reason']}`")
            lines.append(f"  - query: {sample['query']}")
            lines.append(f"  - predicted: {sample['model_answer']}")
            lines.append(f"  - gold: {sample['gold_answer']}")
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--qa-file", type=Path, required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-md", type=Path, required=True)
    parser.add_argument("--sample-limit", type=int, default=8)
    args = parser.parse_args()
    report = analyze(read_json(args.qa_file), args.sample_limit)
    write_json(args.output_json, report)
    write_markdown(args.output_md, report)
    print(json.dumps({"qa_file": str(args.qa_file), "output_json": str(args.output_json)}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
