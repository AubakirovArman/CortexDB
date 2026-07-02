#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


REQUIRED_REPORTS = [
    "target/consensus/distributed-consensus.json",
    "target/consensus/partition-soak.json",
    "target/consensus/failover-slo.json",
    "target/consensus/rejoin.json",
]


def load_report(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def require_marker(path: Path, marker: str, errors: list[str]) -> None:
    if marker not in path.read_text(encoding="utf-8"):
        errors.append(f"{path.relative_to(ROOT)} missing marker: {marker}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/consensus/research-summary.json")
    args = parser.parse_args()

    errors = []
    require_marker(
        ROOT / "docs" / "archive" / "DISTRIBUTED_CONSENSUS_RESEARCH.md",
        "production_ready=false",
        errors,
    )
    require_marker(
        ROOT / "docs" / "archive" / "NEXT_60_EPICS.md",
        "| 59 | Distributed Consensus Research Track | research |",
        errors,
    )

    reports = []
    for report_name in REQUIRED_REPORTS:
        path = ROOT / report_name
        if not path.exists():
            errors.append(f"missing consensus report: {report_name}")
            continue
        report = load_report(path)
        if report.get("status") != "passed":
            errors.append(f"{report_name} status is {report.get('status')!r}")
        if report.get("production_ready") is not False:
            errors.append(f"{report_name} must keep production_ready=false")
        reports.append(report)

    summary = {
        "schema_version": "cortexdb.consensus.research_summary.v1",
        "status": "passed" if not errors else "failed",
        "production_ready": False,
        "boundary": "research evidence only; no production distributed consensus claim",
        "reports": REQUIRED_REPORTS,
        "gates": [report.get("gate") for report in reports],
        "total_passed_tests_from_evidence": sum(
            int(report.get("total_passed_tests_from_evidence", 0)) for report in reports
        ),
        "errors": errors,
    }
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
