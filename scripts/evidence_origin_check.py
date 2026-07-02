#!/usr/bin/env python3
"""Gate production evidence origin classification regressions."""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path
from typing import Any

from evidence_origin import classify_evidence_origin, is_local_reference


LOCAL_REFERENCE_CASES = {
    "file_uri": "file:/tmp/evidence.json",
    "generated_segment": "target/codex-verification/evidence.json",
    "fixture_segment": "fixtures/accountability/evidence.json",
    "temporary_path": "/tmp/operator/evidence.json",
    "loopback_url": "http://localhost/evidence.json",
    "legacy_ipv4": "http://2130706433/evidence.json",
    "windows_drive": "C:/operator/evidence.json",
    "unc_path": "//server/share/evidence.json",
    "local_transport": "unix:///operator/evidence.sock",
    "shell_home": "~/operator/evidence.json",
    "env_home": "$HOME/operator/evidence.json",
    "windows_env_temp": "%TEMP%/operator/evidence.json",
    "relative_path": "operator-evidence/report.pdf",
    "parent_relative_path": "../operator-evidence/report.pdf",
    "absolute_posix_path": "/home/operator/evidence/report.pdf",
    "double_encoded_path": "%252E%252E%252Foperator-evidence%252Freport.pdf",
}

REMOTE_REFERENCE_CASES = {
    "https_report": "https://auditor.example/reports/soc2.pdf",
    "s3_report": "s3://operator-evidence/reports/soc2.pdf",
    "gs_report": "gs://operator-evidence/reports/soc2.pdf",
    "arn_key": "arn:aws:kms:us-east-1:123456789012:key/receipt-key-2026q3",
    "kms_key": "kms://provider/key/receipt-key-2026q3",
    "prose": "operate read write controls",
    "version": "v0.2.0-beta.2",
}


def record_failure(failures: list[str], name: str, detail: str) -> None:
    failures.append(f"{name}: {detail}")


def assert_origin(
    failures: list[str],
    name: str,
    path: Path,
    evidence: dict[str, Any],
    expected_origin: str,
    expected_synthetic: bool,
) -> dict[str, Any]:
    result = classify_evidence_origin(path, evidence)
    if result["origin"] != expected_origin:
        record_failure(
            failures,
            name,
            f"origin {result['origin']!r} != {expected_origin!r}",
        )
    if result["synthetic"] is not expected_synthetic:
        record_failure(
            failures,
            name,
            f"synthetic {result['synthetic']!r} != {expected_synthetic!r}",
        )
    return result


def check_reference_cases(failures: list[str]) -> dict[str, int]:
    checked = {"local": 0, "remote": 0}
    for name, value in LOCAL_REFERENCE_CASES.items():
        checked["local"] += 1
        if not is_local_reference(value.lower()):
            record_failure(failures, name, "local reference was not detected")
    for name, value in REMOTE_REFERENCE_CASES.items():
        checked["remote"] += 1
        if is_local_reference(value.lower()):
            record_failure(failures, name, "remote/operator reference was local")
    return checked


def check_origin_cases(failures: list[str], root: Path) -> dict[str, str]:
    generated = root / "target" / "evidence-origin-check" / "generated.json"
    generated.parent.mkdir(parents=True, exist_ok=True)
    generated.write_text("{}\n", encoding="utf-8")

    with tempfile.TemporaryDirectory(prefix="cortexdb-evidence-origin-check-") as tmp:
        tmp_path = Path(tmp)
        temp_file = tmp_path / "operator-evidence.json"
        temp_file.write_text("{}\n", encoding="utf-8")
        symlink = tmp_path / "generated-link.json"
        symlink.symlink_to(generated)

        results = {
            "operator": assert_origin(
                failures,
                "operator_origin",
                root / "operator-evidence" / "receipt.json",
                {"uri": "s3://operator-evidence/receipt.json"},
                "operator",
                False,
            )["origin"],
            "generated": assert_origin(
                failures,
                "generated_path_origin",
                generated,
                {},
                "generated_local_artifact",
                True,
            )["origin"],
            "temporary": assert_origin(
                failures,
                "temporary_path_origin",
                temp_file,
                {},
                "temporary_local_artifact",
                True,
            )["origin"],
            "symlink": assert_origin(
                failures,
                "resolved_symlink_origin",
                symlink,
                {},
                "generated_local_artifact",
                True,
            )["origin"],
            "nested_local": assert_origin(
                failures,
                "nested_local_reference_origin",
                root / "operator-evidence" / "receipt.json",
                {"uri": "../operator-evidence/report.pdf"},
                "local_reference_artifact",
                True,
            )["origin"],
            "fixture": assert_origin(
                failures,
                "fixture_origin",
                root / "fixtures" / "accountability" / "receipt.json",
                {},
                "synthetic_fixture",
                True,
            )["origin"],
        }
    return results


def build_report(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    reference_counts = check_reference_cases(failures)
    origin_results = check_origin_cases(failures, root)
    return {
        "schema_version": "cortexdb.evidence_origin_check.v1",
        "status": "passed" if not failures else "failed",
        "reference_case_counts": reference_counts,
        "origin_results": origin_results,
        "claim_boundary": (
            "classifier regression gate only; this does not supply operator "
            "KMS/HSM custody or compliance certification evidence"
        ),
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".")
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    report = build_report(Path(args.root).resolve())
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"evidence origin check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
