#!/usr/bin/env python3
"""Drift gate for trainer-emitted frozen ranking weights."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path


SCHEMA_VERSION = "cortexdb.ranking_weights_drift_check.v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--checked-in-artifact", required=True)
    parser.add_argument("--generated-artifact", required=True)
    parser.add_argument("--calibration-report", required=True)
    parser.add_argument("--module-report", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--min-heldout-mrr-lift-bps", type=int, default=2500)
    parser.add_argument("--min-heldout-win-rate-pct", type=int, default=75)
    return parser.parse_args()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run_command(command: list[str]) -> tuple[int, str, str]:
    completed = subprocess.run(command, text=True, capture_output=True, check=False)
    return completed.returncode, completed.stdout, completed.stderr


def run_trainer(args: argparse.Namespace, errors: list[str]) -> dict:
    command = [
        sys.executable,
        "scripts/learned_ranking_calibration_check.py",
        "--fixture",
        args.fixture,
        "--report",
        args.calibration_report,
        "--min-heldout-mrr-lift-bps",
        str(args.min_heldout_mrr_lift_bps),
        "--min-heldout-win-rate-pct",
        str(args.min_heldout_win_rate_pct),
        "--compiled-artifact-template",
        args.checked_in_artifact,
        "--compiled-artifact",
        args.generated_artifact,
    ]
    code, stdout, stderr = run_command(command)
    if code != 0:
        errors.append(f"trainer failed: {stderr.strip() or stdout.strip()}")
        return {}
    report = json.loads(Path(args.calibration_report).read_text(encoding="utf-8"))
    if report.get("status") != "passed":
        errors.append(f"trainer report status is {report.get('status')!r}")
    return report


def compare_artifacts(args: argparse.Namespace, errors: list[str]) -> dict:
    generated = Path(args.generated_artifact).read_bytes()
    checked_in = Path(args.checked_in_artifact).read_bytes()
    generated_hash = sha256_bytes(generated)
    checked_in_hash = sha256_bytes(checked_in)
    if generated != checked_in:
        errors.append(
            "trainer-emitted artifact differs from checked-in frozen artifact "
            f"({generated_hash} != {checked_in_hash})"
        )
    return {
        "generated_artifact": args.generated_artifact,
        "generated_sha256": generated_hash,
        "checked_in_artifact": args.checked_in_artifact,
        "checked_in_sha256": checked_in_hash,
        "byte_identical": generated == checked_in,
    }


def run_module_check(args: argparse.Namespace, errors: list[str]) -> dict:
    command = [
        sys.executable,
        "scripts/ranking_frozen_weights_check.py",
        "--root",
        ".",
        "--fixture",
        args.checked_in_artifact,
        "--report",
        args.module_report,
    ]
    code, stdout, stderr = run_command(command)
    if code != 0:
        errors.append(f"frozen module check failed: {stderr.strip() or stdout.strip()}")
        return {}
    return json.loads(Path(args.module_report).read_text(encoding="utf-8"))


def write_report(path: Path, report: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    errors: list[str] = []
    calibration_report = run_trainer(args, errors)
    artifact_report = compare_artifacts(args, errors) if Path(args.generated_artifact).exists() else {}
    module_report = run_module_check(args, errors)
    report = {
        "schema_version": SCHEMA_VERSION,
        "status": "passed" if not errors else "failed",
        "errors": errors,
        "calibration": {
            "report": args.calibration_report,
            "status": calibration_report.get("status"),
            "heldout_mrr_lift_bps": calibration_report.get("heldout_mrr_lift_bps"),
            "heldout_win_rate_pct": calibration_report.get("heldout_win_rate_pct"),
            "compiled_artifact": calibration_report.get("compiled_artifact"),
        },
        "artifact": artifact_report,
        "module": {
            "report": args.module_report,
            "status": module_report.get("status"),
            "fixture_version": module_report.get("fixture_version"),
        },
    }
    write_report(Path(args.report), report)
    print(json.dumps({"status": report["status"], "report": args.report}, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
