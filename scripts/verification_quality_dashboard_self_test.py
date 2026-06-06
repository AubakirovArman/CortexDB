#!/usr/bin/env python3
"""Self-test the verification quality dashboard builder."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import verification_quality_dashboard as dashboard


def sample_report() -> dict[str, object]:
    return {
        "status": "passed",
        "case_count": 4,
        "accuracy_q16": 65_535,
        "confusion_matrix": {
            "supported": {"supported": 1, "contradicted": 0, "mixed": 0, "insufficient": 0},
            "contradicted": {"supported": 0, "contradicted": 1, "mixed": 0, "insufficient": 0},
            "mixed": {"supported": 0, "contradicted": 0, "mixed": 1, "insufficient": 0},
            "insufficient": {"supported": 0, "contradicted": 0, "mixed": 0, "insufficient": 1},
        },
        "per_domain_status_counts": {
            "investment_projects": {
                "supported": 1,
                "contradicted": 1,
                "mixed": 0,
                "insufficient": 0,
            },
            "legal_policies": {
                "supported": 0,
                "contradicted": 0,
                "mixed": 1,
                "insufficient": 1,
            },
        },
        "guard_cases": 2,
        "numeric_guard_cases": 1,
        "citation_guard_cases": 1,
        "false_positive_count": 0,
        "false_negative_count": 0,
        "failures": [],
    }


def main() -> int:
    report = sample_report()
    built = dashboard.build_dashboard(report)
    assert built["status"] == "passed"
    assert built["case_count"] == 4
    assert built["accuracy_q16"] == 65_535
    assert built["false_positive_count"] == 0
    assert built["false_negative_count"] == 0
    assert len(built["confusion_rows"]) == 4
    assert len(built["per_domain_quality"]) == 2
    assert built["per_domain_quality"][0]["domain"] == "investment_projects"
    assert built["per_domain_quality"][0]["accuracy_q16"] == 65_535

    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        report_path = root / "report.json"
        dashboard_path = root / "dashboard.json"
        markdown_path = root / "dashboard.md"
        report_path.write_text(json.dumps(report), encoding="utf-8")
        code = dashboard.main(
            [
                "--report",
                str(report_path),
                "--dashboard-json",
                str(dashboard_path),
                "--dashboard-md",
                str(markdown_path),
            ]
        )
        assert code == 0
        assert "Verification Quality Dashboard" in markdown_path.read_text(encoding="utf-8")
        assert json.loads(dashboard_path.read_text(encoding="utf-8"))["status"] == "passed"

    print("verification quality dashboard self-test passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
