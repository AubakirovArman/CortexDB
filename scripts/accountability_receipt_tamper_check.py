#!/usr/bin/env python3
"""Run the AR-8 accountability receipt tamper matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from copy import deepcopy
from pathlib import Path
from typing import Any, Callable


Mutation = Callable[[dict[str, Any]], None]

MUTATION_NAMES = [
    "budget_estimated_tokens",
    "access_allowed_to_denied",
    "source_byte_start_shift",
    "drop_visible_conflict",
    "swap_verdict",
    "replay_different_query",
    "flip_signature_byte",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_TAMPER_REPORT ?= target/accountability-receipt/tamper-report.json",
    "accountability-receipt-tamper-check:",
    'python3 scripts/accountability_receipt_tamper_check.py --root "." --fixture "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)" --report "$(ACCOUNTABILITY_RECEIPT_TAMPER_REPORT)"',
]


def mutate_budget(data: dict[str, Any]) -> None:
    data["receipt"]["leaves"]["budget"][1]["cell_estimated_tokens"] += 1


def mutate_access(data: dict[str, Any]) -> None:
    data["receipt"]["leaves"]["access"][0]["decision"] = "denied"


def mutate_span(data: dict[str, Any]) -> None:
    data["receipt"]["leaves"]["provenance"][0]["source_byte_start"] += 1


def mutate_conflict(data: dict[str, Any]) -> None:
    conflict = data["receipt"]["leaves"]["conflict"][0]
    if conflict.get("visible_conflict_count", 0) < 1:
        raise RuntimeError("fixture must contain a visible conflict for drop_visible_conflict")
    conflict["visible_conflict_count"] = 0
    conflict["conflict_visibility_q16"] = 0
    conflict["anomalies"] = [
        anomaly for anomaly in conflict.get("anomalies", []) if anomaly != "visible_conflict"
    ]


def mutate_verdict(data: dict[str, Any]) -> None:
    data["receipt"]["leaves"]["verification"][0]["status"] = "contradicted"


def mutate_replay(data: dict[str, Any]) -> None:
    data["determinism_input"]["query"] += " REPLAYED"


def mutate_signature(data: dict[str, Any]) -> None:
    signature = data["receipt"]["header"]["signature"]["signature_hex"]
    data["receipt"]["header"]["signature"]["signature_hex"] = (
        ("0" if signature[0] != "0" else "1") + signature[1:]
    )


MUTATIONS: dict[str, Mutation] = {
    "budget_estimated_tokens": mutate_budget,
    "access_allowed_to_denied": mutate_access,
    "source_byte_start_shift": mutate_span,
    "drop_visible_conflict": mutate_conflict,
    "swap_verdict": mutate_verdict,
    "replay_different_query": mutate_replay,
    "flip_signature_byte": mutate_signature,
}


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def write_temp(data: dict[str, Any], name: str) -> Path:
    temp = tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=f".{name}.json", delete=False
    )
    with temp:
        json.dump(data, temp, indent=2, sort_keys=True)
        temp.write("\n")
    return Path(temp.name)


def verifier_run(root: Path, input_path: Path, expect_success: bool) -> tuple[bool, str]:
    result = subprocess.run(
        ["cargo", "run", "-p", "cortex-receipt-verify", "--", "--input", str(input_path)],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return (result.returncode == 0) == expect_success, result.stdout


def makefiles_text(root: Path) -> str:
    return "\n".join(path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk")))


def validate(root: Path, fixture: Path) -> dict[str, Any]:
    failures: list[str] = []
    fixture_data = read_json(fixture)
    make_text = makefiles_text(root) + "\n" + (root / "mk/phony.mk").read_text(encoding="utf-8")
    for term in REQUIRED_MAKE_TERMS:
        if term not in make_text:
            failures.append(f"make wiring missing {term}")

    genuine_ok, genuine_output = verifier_run(root, fixture, expect_success=True)
    if not genuine_ok:
        failures.append("genuine fixture was rejected")

    mutation_results: dict[str, dict[str, Any]] = {}
    for name in MUTATION_NAMES:
        mutated = deepcopy(fixture_data)
        try:
            MUTATIONS[name](mutated)
        except RuntimeError as error:
            failures.append(f"{name}: {error}")
            continue
        if mutated == fixture_data:
            failures.append(f"{name}: mutation did not change fixture")
        tampered_path = write_temp(mutated, name)
        rejected, output = verifier_run(root, tampered_path, expect_success=False)
        mutation_results[name] = {
            "rejected": rejected,
            "output_excerpt": output.strip().splitlines()[-1:] or [],
        }
        if not rejected:
            failures.append(f"{name}: verifier accepted tampered fixture")

    return {
        "schema_version": "cortexdb.accountability_receipt_tamper.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "fixture": str(fixture),
        "genuine_output_excerpt": genuine_output.strip().splitlines()[-1:] if genuine_output else [],
        "mutations": mutation_results,
        "checked": {
            "mutation_names": MUTATION_NAMES,
            "make_terms": REQUIRED_MAKE_TERMS,
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--fixture", required=True, help="verifier golden input")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    report = validate(root, (root / args.fixture).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"accountability receipt tamper check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
