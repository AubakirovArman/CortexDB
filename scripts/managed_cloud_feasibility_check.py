#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


REQUIRED_REPORTS = [
    "target/managed-cloud/tenant-lifecycle.json",
    "target/managed-cloud/backup-restore.json",
    "target/managed-cloud/upgrade.json",
]


def require_marker(path: Path, marker: str, errors: list[str]) -> None:
    if marker not in path.read_text(encoding="utf-8"):
        errors.append(f"{path.relative_to(ROOT)} missing marker: {marker}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/managed-cloud/feasibility-summary.json")
    args = parser.parse_args()

    errors = []
    require_marker(
        ROOT / "docs" / "MANAGED_CLOUD_FEASIBILITY.md",
        "managed_cloud_ready=false",
        errors,
    )
    require_marker(
        ROOT / "docs" / "NEXT_60_EPICS.md",
        "| 60 | Managed Cloud Feasibility Track | research |",
        errors,
    )

    reports = []
    for report_name in REQUIRED_REPORTS:
        path = ROOT / report_name
        if not path.exists():
            errors.append(f"missing managed-cloud report: {report_name}")
            continue
        report = json.loads(path.read_text(encoding="utf-8"))
        if report.get("status") != "passed":
            errors.append(f"{report_name} status is {report.get('status')!r}")
        if report.get("managed_cloud_ready") is not False:
            errors.append(f"{report_name} must keep managed_cloud_ready=false")
        reports.append(report)

    summary = {
        "schema_version": "cortexdb.managed_cloud.feasibility_summary.v1",
        "status": "passed" if not errors else "failed",
        "managed_cloud_ready": False,
        "boundary": "local managed-cloud prerequisites only; no hosted service claim",
        "reports": REQUIRED_REPORTS,
        "gates": [report.get("gate") for report in reports],
        "errors": errors,
    }
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
