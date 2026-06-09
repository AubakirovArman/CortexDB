#!/usr/bin/env python3
"""Generate an EnterpriseRAG evidence-slot plan report."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from evidence_slot_planner import build_evidence_plan, read_jsonl


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def build_report(plans: list[dict[str, Any]], output_jsonl: Path, output_report: Path) -> dict[str, Any]:
    by_type = Counter(str(plan.get("question_type") or "unknown") for plan in plans)
    by_policy = Counter(str(plan.get("answer_policy") or "unknown") for plan in plans)
    by_slot_kind: Counter[str] = Counter()
    required_slots = 0
    total_slots = 0
    for plan in plans:
        plan_slots = plan.get("slots", [])
        total_slots += len(plan_slots)
        for slot in plan_slots:
            by_slot_kind[str(slot.get("kind") or "unknown")] += 1
            if slot.get("required"):
                required_slots += 1
    return {
        "schema_version": "cortexdb.enterprise_rag_bench.evidence_slot_plan_report.v1",
        "output_jsonl": str(output_jsonl),
        "output_report": str(output_report),
        "questions": len(plans),
        "total_slots": total_slots,
        "required_slots": required_slots,
        "average_slots_per_question": round(total_slots / len(plans), 2) if plans else 0.0,
        "by_question_type": dict(sorted(by_type.items())),
        "by_answer_policy": dict(sorted(by_policy.items())),
        "by_slot_kind": dict(sorted(by_slot_kind.items())),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--output-jsonl", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--limit", type=int)
    args = parser.parse_args()
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    args = parse_args()
    questions = read_jsonl(args.questions_file)
    if args.limit is not None:
        questions = questions[: args.limit]
    plans = [build_evidence_plan(row) for row in questions]
    report = build_report(plans, args.output_jsonl, args.report)
    write_jsonl(args.output_jsonl, plans)
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
