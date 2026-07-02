#!/usr/bin/env python3
"""Validate Phase 2 crypto dependency readiness without claiming legacy replacement."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REQUIRED_CARGO_TERMS = {
    "crates/cortex-engine/Cargo.toml": [
        "accountability-receipt = [",
        '"dep:argon2"',
        '"dep:blake3"',
        '"dep:chacha20poly1305"',
        '"dep:ed25519-dalek"',
        '"dep:getrandom"',
        '"dep:sha2"',
        '"dep:subtle"',
        '"dep:zeroize"',
        'argon2 = { version = "0.5", optional = true }',
        'blake3 = { version = "1.5", optional = true }',
        'chacha20poly1305 = { version = "0.10", optional = true }',
        'ed25519-dalek = { version = "2.1", optional = true }',
        'getrandom = { version = "0.2", optional = true }',
        'sha2 = { version = "0.10", optional = true }',
        'subtle = { version = "2.5", optional = true }',
        'zeroize = { version = "1.7", optional = true }',
    ],
    "crates/cortex-server/Cargo.toml": [
        "accountability-receipt = [",
        '"dep:ed25519-dalek"',
        '"dep:getrandom"',
        '"dep:sha2"',
        '"dep:subtle"',
        '"dep:zeroize"',
        'ed25519-dalek = { version = "2.1", optional = true }',
        'getrandom = { version = "0.2", optional = true }',
        'sha2 = { version = "0.10", optional = true }',
        'subtle = { version = "2.5", optional = true }',
        'zeroize = { version = "1.7", optional = true }',
    ],
    "crates/cortex-cli/Cargo.toml": [
        "accountability-receipt = [",
        '"dep:ed25519-dalek"',
        '"dep:getrandom"',
        '"dep:sha2"',
        '"dep:subtle"',
        '"dep:zeroize"',
        'ed25519-dalek = { version = "2.1", optional = true }',
        'getrandom = { version = "0.2", optional = true }',
        'sha2 = { version = "0.10", optional = true }',
        'subtle = { version = "2.5", optional = true }',
        'zeroize = { version = "1.7", optional = true }',
    ],
}

REQUIRED_MAKE_TERMS = [
    "CRYPTO_DEPS_READINESS_REPORT ?= target/crypto-deps-readiness/report.json",
    "crypto-deps-readiness-check:",
    'python3 scripts/crypto_deps_readiness_check.py --root "." --report "$(CRYPTO_DEPS_READINESS_REPORT)"',
]

LEGACY_POLICY_BLOCKERS = [
    {
        "name": "encrypted_backup_xor_fnv_v1",
        "path": "crates/cortex-engine/src/backup/encrypted/crypto.rs",
        "markers": [
            "cortexdb.xor-fnv64-stream.v1",
            "cortexdb.fnv64-passphrase.v1",
            "FNV_OFFSET",
            "FNV_PRIME",
        ],
        "exit_gate": "CRY-3 encrypted backup v2",
    },
    {
        "name": "server_audit_chain_fnv_v1",
        "path": "crates/cortex-server/src/audit_chain.rs",
        "markers": ["FNV_OFFSET", "FNV_PRIME"],
        "exit_gate": "CRY-4 audit chain hash/signature replacement",
    },
    {
        "name": "cli_audit_chain_fnv_v1",
        "path": "crates/cortex-cli/src/cli_audit_chain.rs",
        "markers": ["FNV_OFFSET", "FNV_PRIME"],
        "exit_gate": "CRY-4 audit chain verifier replacement",
    },
]


def read_text(root: Path, rel: str) -> str:
    path = root / rel
    if not path.exists():
        raise FileNotFoundError(path)
    return path.read_text(encoding="utf-8")


def missing_terms(label: str, text: str, terms: list[str]) -> list[str]:
    return [f"{label}: missing {term}" for term in terms if term not in text]


def find_legacy_blockers(root: Path) -> list[dict[str, Any]]:
    blockers: list[dict[str, Any]] = []
    for blocker in LEGACY_POLICY_BLOCKERS:
        text = read_text(root, blocker["path"])
        present = [marker for marker in blocker["markers"] if marker in text]
        if present:
            blockers.append({**blocker, "present_markers": present})
    return blockers


def validate(root: Path) -> dict[str, Any]:
    failures: list[str] = []
    for rel, terms in REQUIRED_CARGO_TERMS.items():
        failures.extend(missing_terms(rel, read_text(root, rel), terms))

    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", read_text(root, "mk/phony.mk"), [
        "crypto-deps-readiness-check",
    ]))

    legacy_blockers = find_legacy_blockers(root)
    return {
        "schema_version": "cortexdb.crypto_deps_readiness.report.v1",
        "status": "failed" if failures else "passed",
        "ready_for_next_crypto_slice": not failures,
        "production_safe": not failures and not legacy_blockers,
        "checked": {
            "cargo_terms": REQUIRED_CARGO_TERMS,
            "make_terms": REQUIRED_MAKE_TERMS,
            "legacy_policy_blockers": LEGACY_POLICY_BLOCKERS,
        },
        "legacy_policy_blockers": legacy_blockers,
        "next_exit_gate": "crypto-deps-policy-check after CRY-3 and CRY-4 remove legacy backup/audit crypto",
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
    print(f"crypto deps readiness check passed: {report_path}")
    if report["legacy_policy_blockers"]:
        print("legacy policy blockers remain for crypto-deps-policy-check")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
