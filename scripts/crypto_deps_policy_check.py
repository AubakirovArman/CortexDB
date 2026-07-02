#!/usr/bin/env python3
"""Validate the Phase 2 crypto dependency policy gate."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CRYPTO_CRATE_TERMS = [
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

REQUIRED_CONSUMER_TERMS = {
    "crates/cortex-engine/Cargo.toml": ['cortex-crypto = { path = "../cortex-crypto" }'],
    "crates/cortex-server/Cargo.toml": ['cortex-crypto = { path = "../cortex-crypto" }'],
    "crates/cortex-cli/Cargo.toml": ['cortex-crypto = { path = "../cortex-crypto" }'],
}

PRODUCTION_CRYPTO_PATHS = [
    "crates/cortex-engine/src/backup/encrypted/crypto.rs",
    "crates/cortex-engine/src/backup/encrypted/codec.rs",
    "crates/cortex-server/src/audit_chain.rs",
    "crates/cortex-cli/src/cli_audit_chain.rs",
]

FORBIDDEN_LEGACY_TERMS = [
    "FNV_OFFSET",
    "FNV_PRIME",
    "apply_keystream",
    "auth_tag",
    "hash_hex",
    "stream_word",
    "xor-fnv64-stream",
    "fnv64-passphrase",
]

REQUIRED_MAKE_TERMS = [
    "CRYPTO_DEPS_POLICY_REPORT ?= target/crypto-deps-policy/report.json",
    "crypto-deps-policy-check:",
    "$(MAKE) crypto-deps-readiness-check",
    'python3 scripts/crypto_deps_policy_check.py --root "." --report "$(CRYPTO_DEPS_POLICY_REPORT)"',
]


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def forbidden_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: forbidden legacy term remains: {term}" for term in terms if term in text]


def validate(root: Path) -> dict[str, Any]:
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    failures: list[str] = []
    failures.extend(
        missing_terms(
            "crates/cortex-crypto/Cargo.toml",
            read_text(root, "crates/cortex-crypto/Cargo.toml"),
            REQUIRED_CRYPTO_CRATE_TERMS,
        )
    )
    for rel, terms in REQUIRED_CONSUMER_TERMS.items():
        failures.extend(missing_terms(rel, read_text(root, rel), terms))
    for rel in PRODUCTION_CRYPTO_PATHS:
        failures.extend(forbidden_terms(rel, read_text(root, rel), FORBIDDEN_LEGACY_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", read_text(root, "mk/phony.mk"), [
        "crypto-deps-policy-check",
    ]))
    return {
        "schema_version": "cortexdb.crypto_deps_policy.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "crypto_crate_terms": REQUIRED_CRYPTO_CRATE_TERMS,
            "consumer_terms": REQUIRED_CONSUMER_TERMS,
            "production_crypto_paths": PRODUCTION_CRYPTO_PATHS,
            "forbidden_legacy_terms": FORBIDDEN_LEGACY_TERMS,
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
    print(f"crypto deps policy check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
