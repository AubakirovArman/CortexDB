#!/usr/bin/env python3
"""Build a dashboard from the deterministic VERIFY FACT quality report."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535
STATUSES = ("supported", "contradicted", "mixed", "insufficient")


def q16(numerator: int, denominator: int) -> int:
    if denominator <= 0:
        return Q16_ONE
    return min(Q16_ONE, (numerator * Q16_ONE) // denominator)


def read_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise RuntimeError(f"missing verification report: {path}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def int_value(data: dict[str, Any], key: str) -> int:
    value = data.get(key)
    if not isinstance(value, int) or value < 0:
        raise RuntimeError(f"verification report field {key} must be a non-negative integer")
    return value


def dict_value(data: dict[str, Any], key: str) -> dict[str, Any]:
    value = data.get(key)
    if not isinstance(value, dict):
        raise RuntimeError(f"verification report field {key} must be an object")
    return value


def build_dashboard(report: dict[str, Any]) -> dict[str, Any]:
    confusion = dict_value(report, "confusion_matrix")
    per_domain_status_counts = dict_value(report, "per_domain_status_counts")
    case_count = int_value(report, "case_count")
    false_positive_count = int_value(report, "false_positive_count")
    false_negative_count = int_value(report, "false_negative_count")

    rows = []
    correct = 0
    for expected in STATUSES:
        row = confusion.get(expected)
        if not isinstance(row, dict):
            raise RuntimeError(f"confusion matrix missing row {expected}")
        observed_counts = {}
        for observed in STATUSES:
            count = row.get(observed)
            if not isinstance(count, int) or count < 0:
                raise RuntimeError(f"confusion matrix {expected}->{observed} must be integer")
            observed_counts[observed] = count
        correct += observed_counts[expected]
        rows.append(
            {
                "expected": expected,
                "observed": observed_counts,
                "total": sum(observed_counts.values()),
                "correct": observed_counts[expected],
                "accuracy_q16": q16(observed_counts[expected], sum(observed_counts.values())),
            }
        )

    domain_rows = []
    for domain, counts in sorted(per_domain_status_counts.items()):
        if not isinstance(domain, str) or not isinstance(counts, dict):
            raise RuntimeError("per-domain status counts must map domain names to objects")
        total = 0
        for status in STATUSES:
            count = counts.get(status)
            if not isinstance(count, int) or count < 0:
                raise RuntimeError(f"per-domain {domain}.{status} must be integer")
            total += count
        domain_rows.append(
            {
                "domain": domain,
                "case_count": total,
                "status_counts": {status: counts[status] for status in STATUSES},
                # The companion gate fails when observed != expected, so per-domain
                # quality is perfect only when the source report passed.
                "accuracy_q16": q16(total, total) if report.get("status") == "passed" else 0,
            }
        )

    failures = list(report.get("failures", []))
    if false_positive_count:
        failures.append(f"false positives: {false_positive_count}")
    if false_negative_count:
        failures.append(f"false negatives: {false_negative_count}")

    return {
        "schema_version": "cortexdb.verification_quality_dashboard.v1",
        "status": "passed" if report.get("status") == "passed" and not failures else "failed",
        "case_count": case_count,
        "accuracy_q16": q16(correct, case_count),
        "confusion_matrix": confusion,
        "confusion_rows": rows,
        "false_positive_count": false_positive_count,
        "false_negative_count": false_negative_count,
        "per_domain_quality": domain_rows,
        "guard_cases": int_value(report, "guard_cases"),
        "numeric_guard_cases": int_value(report, "numeric_guard_cases"),
        "citation_guard_cases": int_value(report, "citation_guard_cases"),
        "failures": failures,
    }


def write_markdown(dashboard: dict[str, Any], path: Path) -> None:
    lines = [
        "# Verification Quality Dashboard",
        "",
        f"Status: `{dashboard['status']}`",
        f"Cases: `{dashboard['case_count']}`",
        f"Accuracy q16: `{dashboard['accuracy_q16']}`",
        f"False positives: `{dashboard['false_positive_count']}`",
        f"False negatives: `{dashboard['false_negative_count']}`",
        "",
        "## Confusion Matrix",
        "",
        "| Expected | Supported | Contradicted | Mixed | Insufficient | Accuracy q16 |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in dashboard["confusion_rows"]:
        observed = row["observed"]
        lines.append(
            f"| {row['expected']} | {observed['supported']} | {observed['contradicted']} | "
            f"{observed['mixed']} | {observed['insufficient']} | {row['accuracy_q16']} |"
        )

    lines.extend(
        [
            "",
            "## Per-Domain Quality",
            "",
            "| Domain | Cases | Supported | Contradicted | Mixed | Insufficient | Accuracy q16 |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for row in dashboard["per_domain_quality"]:
        counts = row["status_counts"]
        lines.append(
            f"| {row['domain']} | {row['case_count']} | {counts['supported']} | "
            f"{counts['contradicted']} | {counts['mixed']} | {counts['insufficient']} | "
            f"{row['accuracy_q16']} |"
        )

    if dashboard["failures"]:
        lines.extend(["", "## Failures", ""])
        lines.extend(f"- {failure}" for failure in dashboard["failures"])

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    parser.add_argument("--dashboard-json", required=True)
    parser.add_argument("--dashboard-md", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        dashboard = build_dashboard(read_report(Path(args.report)))
    except RuntimeError as error:
        print(f"verification quality dashboard failed: {error}", file=sys.stderr)
        return 1

    output = Path(args.dashboard_json)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(dashboard, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(dashboard, Path(args.dashboard_md))
    print(f"verification quality dashboard: {output}")
    return 0 if dashboard["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
