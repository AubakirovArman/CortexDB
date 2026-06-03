#!/usr/bin/env python3
"""Break down MultiHop-RAG temporal QA results by coarse subtype."""

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


def temporal_subtype(query: str) -> str:
    normalized = normalize(query)
    if normalized.startswith("which ") or any(
        term in normalized
        for term in ["which news source", "which source", "which article", "larger or smaller", "larger", "smaller"]
    ):
        return "source_or_entity"
    if any(term in normalized for term in ["before", "after", "subsequent", "later", "earlier", "between"]):
        if any(term in normalized for term in ["change", "changed", "different portrayal", "nature of the events", "stance"]):
            return "change_over_time"
        if any(
            term in normalized
            for term in [
                "consistent",
                "consistency",
                "agreement",
                "agree",
                "disagreement",
                "inconsistent",
                "inconsistency",
                "discrepancy",
                "contradict",
            ]
        ):
            return "consistency_conflict"
        return "chronology"
    if any(term in normalized for term in ["change", "changed", "different portrayal", "stance"]):
        return "change_over_time"
    if any(
        term in normalized
        for term in [
            "consistent",
            "consistency",
            "agreement",
            "agree",
            "disagreement",
            "inconsistent",
            "inconsistency",
            "discrepancy",
            "contradict",
        ]
    ):
        return "consistency_conflict"
    return "other"


def temporal_answer_form(query: str) -> str:
    normalized = normalize(query)
    first_word = normalized.split(" ", 1)[0] if normalized else ""
    if any(term in normalized for term in ["before or after", "after or before"]):
        return "temporal_label"
    if any(term in normalized for term in ["consistent or inconsistent", "agreement or disagreement"]):
        return "temporal_label"
    if first_word in {"which", "what", "who"} or normalized.startswith("between "):
        return "choice"
    if first_word in {
        "did",
        "was",
        "were",
        "is",
        "are",
        "has",
        "have",
        "had",
        "do",
        "does",
        "can",
        "could",
        "will",
    }:
        return "yes_no"
    if any(f" {term} " in f" {normalized} " for term in ["did", "was", "were", "is", "are", "has"]):
        return "yes_no"
    return "other"


def sample_row(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "query": row.get("query", ""),
        "model_answer": row.get("model_answer", ""),
        "gold_answer": row.get("gold_answer", ""),
    }


def analyze(rows: list[dict[str, Any]], sample_limit: int) -> dict[str, Any]:
    by_subtype: dict[str, Counter[str]] = defaultdict(Counter)
    by_answer_form: dict[str, Counter[str]] = defaultdict(Counter)
    chronology_by_answer_form: dict[str, Counter[str]] = defaultdict(Counter)
    samples: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        if row.get("question_type") != "temporal_query":
            continue
        subtype = temporal_subtype(str(row.get("query", "")))
        answer_form = temporal_answer_form(str(row.get("query", "")))
        hit = official_like_hit(str(row.get("model_answer", "")), str(row.get("gold_answer", "")))
        by_subtype[subtype]["total"] += 1
        by_subtype[subtype]["hit"] += int(hit)
        by_subtype[subtype]["miss"] += int(not hit)
        by_answer_form[answer_form]["total"] += 1
        by_answer_form[answer_form]["hit"] += int(hit)
        by_answer_form[answer_form]["miss"] += int(not hit)
        if subtype == "chronology":
            chronology_by_answer_form[answer_form]["total"] += 1
            chronology_by_answer_form[answer_form]["hit"] += int(hit)
            chronology_by_answer_form[answer_form]["miss"] += int(not hit)
        sample_key = f"{subtype}/{answer_form}"
        if not hit and len(samples[sample_key]) < sample_limit:
            next_sample = sample_row(row)
            next_sample["answer_form"] = answer_form
            samples[sample_key].append(next_sample)

    def materialize(counter: Counter[str]) -> dict[str, Any]:
        total = counter["total"]
        return {
            "total": total,
            "hits": counter["hit"],
            "misses": counter["miss"],
            "hit_rate": round(counter["hit"] / total, 4) if total else 0.0,
        }

    return {
        "schema_version": "cortexdb.multihop_rag.temporal_subtype_analysis.v2",
        "by_subtype": {subtype: materialize(counter) for subtype, counter in sorted(by_subtype.items())},
        "by_answer_form": {
            answer_form: materialize(counter) for answer_form, counter in sorted(by_answer_form.items())
        },
        "chronology_by_answer_form": {
            answer_form: materialize(counter) for answer_form, counter in sorted(chronology_by_answer_form.items())
        },
        "samples": dict(samples),
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# MultiHop-RAG Temporal Subtype Analysis",
        "",
        "This report groups `temporal_query` misses into coarse prompt-tuning buckets.",
        "",
        "| Subtype | Total | Hits | Misses | Hit rate |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for subtype, stats in report["by_subtype"].items():
        lines.append(
            f"| `{subtype}` | {stats['total']} | {stats['hits']} | {stats['misses']} | {stats['hit_rate']:.4f} |"
        )
    lines.extend(
        [
            "",
            "## By Answer Form",
            "",
            "| Answer form | Total | Hits | Misses | Hit rate |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for answer_form, stats in report["by_answer_form"].items():
        lines.append(
            f"| `{answer_form}` | {stats['total']} | {stats['hits']} | {stats['misses']} | {stats['hit_rate']:.4f} |"
        )
    lines.extend(
        [
            "",
            "## Chronology By Answer Form",
            "",
            "| Answer form | Total | Hits | Misses | Hit rate |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for answer_form, stats in report["chronology_by_answer_form"].items():
        lines.append(
            f"| `{answer_form}` | {stats['total']} | {stats['hits']} | {stats['misses']} | {stats['hit_rate']:.4f} |"
        )
    lines.extend(["", "## Miss Samples", ""])
    for subtype, samples in report["samples"].items():
        lines.append(f"### {subtype}")
        lines.append("")
        for sample in samples:
            lines.append(f"- query: {sample['query']}")
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
