#!/usr/bin/env python3
"""Audit the normalized next-60 epic map against retained evidence."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALID_STATUSES = {"closed", "partial", "not started", "research"}


class AuditError(RuntimeError):
    pass


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AuditError(f"{path}: expected JSON object")
    return value


def parse_summary(text: str) -> dict[str, int]:
    summary: dict[str, int] = {}
    in_summary = False
    for line in text.splitlines():
        if line == "## Current Summary":
            in_summary = True
            continue
        if in_summary and line.startswith("## "):
            break
        match = re.match(r"\| ([a-z ]+) \| (\d+) \|", line)
        if match:
            summary[match.group(1)] = int(match.group(2))
    return summary


def parse_epic_rows(text: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in text.splitlines():
        match = re.match(r"\| (\d+) \| ([^|]+) \| ([^|]+) \|", line)
        if not match:
            continue
        rows.append({
            "number": int(match.group(1)),
            "epic": match.group(2).strip(),
            "status": match.group(3).strip(),
        })
    return rows


def audit(next_epics: Path, storage_report: Path, require_complete: bool) -> dict[str, Any]:
    text = next_epics.read_text(encoding="utf-8")
    summary = parse_summary(text)
    rows = parse_epic_rows(text)
    errors: list[str] = []

    if len(rows) != 60:
        errors.append(f"expected 60 epic rows, found {len(rows)}")
    expected_numbers = list(range(1, 61))
    actual_numbers = [row["number"] for row in rows]
    if actual_numbers != expected_numbers:
        errors.append("epic row numbers are not exactly 1..60")

    status_counts = Counter(row["status"] for row in rows)
    unknown = sorted(set(status_counts) - VALID_STATUSES)
    if unknown:
        errors.append(f"unknown statuses: {unknown}")
    for status in VALID_STATUSES | {"total"}:
        expected = len(rows) if status == "total" else status_counts.get(status, 0)
        if summary.get(status) != expected:
            errors.append(f"summary mismatch for {status}: summary={summary.get(status)} actual={expected}")

    storage = load_json(storage_report)
    evidence = storage.get("twenty_four_hour_evidence", {})
    twenty_four_hour_met = bool(isinstance(evidence, dict) and evidence.get("met"))
    epic38 = next((row for row in rows if row["number"] == 38), None)
    if epic38 is None:
        errors.append("missing Epic 38")
    elif epic38["status"] == "closed" and not twenty_four_hour_met:
        errors.append("Epic 38 is closed without twenty_four_hour_evidence.met=true")
    elif twenty_four_hour_met and epic38["status"] != "closed":
        errors.append("24-hour storage soak evidence is met, but Epic 38 is not closed")

    if require_complete:
        if status_counts.get("partial", 0) != 0:
            errors.append("completion requires zero partial epics")
        if status_counts.get("not started", 0) != 0:
            errors.append("completion requires zero not-started epics")
        if not twenty_four_hour_met:
            errors.append("completion requires retained 24-hour storage soak evidence")

    return {
        "status": "failed" if errors else "passed",
        "require_complete": require_complete,
        "summary": summary,
        "actual_counts": {status: status_counts.get(status, 0) for status in sorted(VALID_STATUSES)},
        "total_epics": len(rows),
        "epic38_status": epic38["status"] if epic38 else None,
        "twenty_four_hour_met": twenty_four_hour_met,
        "storage_report": str(storage_report),
        "errors": errors,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--next-epics", default="docs/NEXT_60_EPICS.md")
    parser.add_argument("--storage-report", default="target/storage-soak-history/report.json")
    parser.add_argument("--output", default="target/next-60-epics-audit/report.json")
    parser.add_argument("--require-complete", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = audit(ROOT / args.next_epics, ROOT / args.storage_report, args.require_complete)
    output = ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"next 60 epics audit {report['status']}: {output}")
    for error in report["errors"]:
        print(f"- {error}")
    return 1 if report["errors"] else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AuditError as exc:
        print(f"next 60 epics audit failed: {exc}")
        raise SystemExit(1)
