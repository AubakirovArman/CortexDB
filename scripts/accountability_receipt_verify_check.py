#!/usr/bin/env python3
"""Validate the AR-7 standalone accountability receipt verifier gate."""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any


VERIFIER_FILES = [
    "crates/cortex-receipt-verify/Cargo.toml",
    "crates/cortex-receipt-verify/src/lib.rs",
    "crates/cortex-receipt-verify/src/canonical.rs",
    "crates/cortex-receipt-verify/src/hex.rs",
    "crates/cortex-receipt-verify/src/model.rs",
    "crates/cortex-receipt-verify/src/receipt_hash.rs",
    "crates/cortex-receipt-verify/src/verifier.rs",
    "crates/cortex-receipt-verify/src/tests.rs",
    "crates/cortex-receipt-verify/src/main.rs",
    "fixtures/accountability_receipt/verify_input.golden.json",
]

REQUIRED_TERMS = [
    "cortex-receipt-verify",
    "VerifyInput",
    "ReceiptHeader",
    "ReceiptLeaves",
    "verify_input",
    "canonical_json_bytes",
    "decode_hex",
    "canonical_header_bytes",
    "audit_chain_head",
    "merkle_root",
    "ReceiptPublicKey",
    "ReceiptSignature",
    "RECEIPT_SIGNING_DOMAIN",
    "cortexdb.accountability_receipt_verify_input.v1",
]

REQUIRED_MAKE_TERMS = [
    "ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE ?= fixtures/accountability_receipt/verify_input.golden.json",
    "ACCOUNTABILITY_RECEIPT_VERIFY_REPORT ?= target/accountability-receipt/verify-report.json",
    "accountability-receipt-verify-check:",
    "cargo test -p cortex-receipt-verify",
    'cargo run -p cortex-receipt-verify -- --input "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)"',
    'python3 scripts/accountability_receipt_verify_check.py --root "." --fixture "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)" --report "$(ACCOUNTABILITY_RECEIPT_VERIFY_REPORT)"',
]

FORBIDDEN_CRATES = [
    "cortex-engine",
    "cortex-storage",
    "cortex-aql",
    "cortex-server",
]

MAX_RUST_FILE_LINES = 300


def read_text(path: Path) -> str:
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def line_count_failures(root: Path, paths: list[str]) -> list[str]:
    failures = []
    for path in paths:
        if not path.endswith(".rs"):
            continue
        line_count = len(read_text(root / path).splitlines())
        if line_count > MAX_RUST_FILE_LINES:
            failures.append(
                f"{path}: {line_count} lines exceeds {MAX_RUST_FILE_LINES} line bound"
            )
    return failures


def command(root: Path, args: list[str], expect_success: bool) -> tuple[bool, str]:
    result = subprocess.run(
        args,
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    ok = result.returncode == 0
    return ok == expect_success, result.stdout


def mutate_fixture(fixture: Path, field: str) -> Path:
    data = json.loads(read_text(fixture))
    if field == "signature":
        signature = data["receipt"]["header"]["signature"]["signature_hex"]
        data["receipt"]["header"]["signature"]["signature_hex"] = (
            ("0" if signature[0] != "0" else "1") + signature[1:]
        )
    elif field == "budget":
        data["receipt"]["leaves"]["budget"][1]["cell_estimated_tokens"] += 1
    else:
        raise ValueError(field)
    temp = tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", suffix=f".{field}.json", delete=False
    )
    with temp:
        json.dump(data, temp, indent=2, sort_keys=True)
        temp.write("\n")
    return Path(temp.name)


def validate(root: Path, fixture: Path) -> dict[str, Any]:
    failures: list[str] = []
    verifier_text = "\n".join(read_text(root / path) for path in VERIFIER_FILES)
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    phony = read_text(root / "mk/phony.mk")

    failures.extend(missing_terms("standalone verifier", verifier_text, REQUIRED_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", phony, ["accountability-receipt-verify-check"]))
    failures.extend(line_count_failures(root, VERIFIER_FILES))

    tree_ok, tree_output = command(
        root,
        ["cargo", "tree", "-p", "cortex-receipt-verify", "--edges", "normal"],
        expect_success=True,
    )
    if not tree_ok:
        failures.append("cargo tree for cortex-receipt-verify failed")
    for forbidden in FORBIDDEN_CRATES:
        if forbidden in tree_output:
            failures.append(f"dependency graph must not include {forbidden}")

    genuine_ok, _ = command(
        root,
        ["cargo", "run", "-p", "cortex-receipt-verify", "--", "--input", str(fixture)],
        expect_success=True,
    )
    if not genuine_ok:
        failures.append("standalone verifier did not accept genuine fixture")

    for field in ["signature", "budget"]:
        tampered = mutate_fixture(fixture, field)
        tamper_ok, _ = command(
            root,
            ["cargo", "run", "-p", "cortex-receipt-verify", "--", "--input", str(tampered)],
            expect_success=False,
        )
        if not tamper_ok:
            failures.append(f"standalone verifier did not reject {field} tamper")

    return {
        "schema_version": "cortexdb.accountability_receipt_verify.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "verifier_files": VERIFIER_FILES,
            "fixture": str(fixture),
            "required_terms": REQUIRED_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "forbidden_crates": FORBIDDEN_CRATES,
            "max_rust_file_lines": MAX_RUST_FILE_LINES,
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
    fixture = (root / args.fixture).resolve()
    report = validate(root, fixture)
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"accountability receipt verify check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
