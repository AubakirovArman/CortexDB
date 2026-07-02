#!/usr/bin/env python3
"""Validate the shared CortexDB crypto primitives gate wiring."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_WORKSPACE_TERMS = [
    '"crates/cortex-crypto"',
]

REQUIRED_CARGO_TERMS = [
    'argon2 = "0.5"',
    'blake3 = "1.5"',
    'chacha20poly1305 = "0.10"',
    'ed25519-dalek = "2.1"',
    'getrandom = "0.2"',
    'hmac = "0.12"',
    'sha2 = "0.10"',
    'subtle = "2.5"',
    'zeroize = "1.7"',
]

REQUIRED_API_TERMS = [
    "pub mod audit_chain",
    "pub mod receipt_key",
    "pub fn event_hash",
    "pub fn event_mac",
    "pub fn sha256",
    "pub fn blake3_256",
    "pub fn xchacha20poly1305_seal",
    "pub fn xchacha20poly1305_open",
    "pub fn derive_argon2id_key",
    "pub fn hmac_sha256",
    "pub fn verify_hmac_sha256",
    "pub fn ed25519_sign",
    "pub fn ed25519_verify",
    "pub struct ReceiptSigningKey",
    "pub struct ReceiptKeyRing",
    "RECEIPT_SIGNING_DOMAIN",
    "pub fn constant_time_eq",
    "pub struct KeyId",
    "pub struct SecretBytes",
]

REQUIRED_TEST_TERMS = [
    "audit_event_hash_is_sha256_width_and_order_sensitive",
    "audit_event_mac_is_keyed_and_sha256_width",
    "sha256_and_blake3_known_answer_vectors_match",
    "hmac_sha256_known_answer_vector_and_constant_time_verify_match",
    "xchacha20poly1305_known_answer_vector_opens_and_rejects_tamper",
    "argon2id_known_answer_vector_matches_pinned_params",
    "ed25519_rfc8032_known_answer_vector_signs_and_verifies",
    "receipt_signatures_are_deterministic_and_domain_separated",
    "receipt_keyring_verifies_current_and_previous_keys",
]

REQUIRED_MAKE_TERMS = [
    "CRYPTO_PRIMITIVES_REPORT ?= target/crypto-primitives/report.json",
    "crypto-primitives-check:",
    "cargo test -p cortex-crypto",
    'python3 scripts/crypto_primitives_check.py --root "." --report "$(CRYPTO_PRIMITIVES_REPORT)"',
]


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def validate(root: Path) -> dict[str, Any]:
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    source = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((root / "crates/cortex-crypto/src").glob("*.rs"))
    )
    failures: list[str] = []
    failures.extend(missing_terms("workspace Cargo.toml", read_text(root, "Cargo.toml"), REQUIRED_WORKSPACE_TERMS))
    failures.extend(
        missing_terms(
            "crates/cortex-crypto/Cargo.toml",
            read_text(root, "crates/cortex-crypto/Cargo.toml"),
            REQUIRED_CARGO_TERMS,
        )
    )
    failures.extend(missing_terms("cortex-crypto src", source, REQUIRED_API_TERMS))
    tests = source + "\n" + read_text(root, "crates/cortex-crypto/tests/primitives_kat.rs")
    failures.extend(missing_terms("cortex-crypto tests", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", read_text(root, "mk/phony.mk"), [
        "crypto-primitives-check",
    ]))
    return {
        "schema_version": "cortexdb.crypto_primitives.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "workspace_terms": REQUIRED_WORKSPACE_TERMS,
            "cargo_terms": REQUIRED_CARGO_TERMS,
            "api_terms": REQUIRED_API_TERMS,
            "test_terms": REQUIRED_TEST_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
        },
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--report", required=True, help="output JSON report path")
    args = parser.parse_args()

    report = validate(Path(args.root).resolve())
    report_path = Path(args.report)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["failures"]:
        for failure in report["failures"]:
            print(failure)
        return 1
    print(f"crypto primitives check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
