#!/usr/bin/env python3
"""Close the storage soak epic only after retained 24-hour evidence exists."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


class FinalizeError(RuntimeError):
    pass


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FinalizeError(f"missing report: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise FinalizeError(f"{path}: expected JSON object")
    return value


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise FinalizeError(f"{path}: expected exactly one match for {old!r}, found {count}")
    return text.replace(old, new, 1)


def close_next_epics(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    if "| 38 | Storage Soak History | closed |" in text:
        return
    row_old = (
        "| 38 | Storage Soak History | partial | `make storage-soak-history-check` runs a fresh soak "
        "and writes explicit 24h evidence status; `make storage-soak-24h-campaign` now provides "
        "the resumable campaign runner for accumulating real 24-hour evidence. | Run and retain "
        "the full campaign until `target/storage-soak-history/report.json` has "
        "`twenty_four_hour_evidence.met=true`. |"
    )
    row_new = (
        "| 38 | Storage Soak History | closed | `make storage-soak-history-check`, "
        "`make storage-soak-24h-campaign`, and `target/storage-soak-history/report.json` now "
        "provide retained 24-hour soak evidence with `twenty_four_hour_evidence.met=true`. | "
        "Keep rerunning the campaign for future releases before updating evidence bundles. |"
    )
    text = replace_once(text, "| closed | 57 |", "| closed | 58 |", path)
    text = replace_once(text, "| partial | 1 |", "| partial | 0 |", path)
    text = replace_once(text, row_old, row_new, path)
    text = replace_once(
        text,
        "1. Advance Epic 38: run and retain a real 24-hour storage soak campaign.",
        "1. Keep Epic 38 storage soak evidence fresh before each release evidence bundle.",
        path,
    )
    path.write_text(text, encoding="utf-8")


def close_current_epics(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    if "| 8 | Storage Compatibility And Soak History | done-local |" in text:
        return
    text = replace_once(text, "- `done-local`: 17 / 18", "- `done-local`: 18 / 18", path)
    text = replace_once(text, "- `partial`: 1 / 18", "- `partial`: 0 / 18", path)
    text = replace_once(
        text,
        "| 8 | Storage Compatibility And Soak History | partial | `target/storage-compat/report.json`, "
        "`target/storage-soak/report.json`, `target/storage-soak-history/report.json` |",
        "| 8 | Storage Compatibility And Soak History | done-local | `target/storage-compat/report.json`, "
        "`target/storage-soak/report.json`, `target/storage-soak-history/report.json` |",
        path,
    )
    status_old = (
        "Current status:\n\n"
        "- `partial`.\n"
        "- Evidence: `target/storage-compat/report.json`,\n"
        "  `target/storage-soak/report.json`, `target/storage-soak-history/report.json`,\n"
        "  backup reports, crash/fault reports, and migration compatibility reports.\n"
        "- Remaining closure: keep the 24-hour campaign running until\n"
        "  `target/storage-soak-history/report.json` reports\n"
        "  `twenty_four_hour_evidence.met=true`."
    )
    status_new = (
        "Current status:\n\n"
        "- `done-local`.\n"
        "- Evidence: `target/storage-compat/report.json`,\n"
        "  `target/storage-soak/report.json`, `target/storage-soak-history/report.json`,\n"
        "  backup reports, crash/fault reports, migration compatibility reports, and retained\n"
        "  24-hour soak evidence with `twenty_four_hour_evidence.met=true`."
    )
    text = replace_once(text, status_old, status_new, path)
    path.write_text(text, encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/storage-soak-history/report.json")
    parser.add_argument("--next-epics", default="docs/NEXT_60_EPICS.md")
    parser.add_argument("--current-epics", default="docs/PL_CURRENT_EPICS.md")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report_path = ROOT / args.report
    report = load_json(report_path)
    evidence = report.get("twenty_four_hour_evidence", {})
    if not isinstance(evidence, dict) or not evidence.get("met"):
        remaining = evidence.get("remaining_seconds", "unknown") if isinstance(evidence, dict) else "unknown"
        print(f"storage soak epic is not ready: twenty_four_hour_evidence.met=false remaining_seconds={remaining}")
        return 1
    if args.dry_run:
        print("storage soak epic is ready to close")
        return 0
    close_next_epics(ROOT / args.next_epics)
    close_current_epics(ROOT / args.current_epics)
    print("storage soak epic closed in docs")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except FinalizeError as exc:
        print(f"storage soak epic finalize failed: {exc}")
        raise SystemExit(1)
