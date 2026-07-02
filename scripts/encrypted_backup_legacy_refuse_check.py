#!/usr/bin/env python3
"""Validate encrypted backup v2 legacy-refusal and old routine removal."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


FORBIDDEN_BACKUP_TERMS = [
    "apply_keystream",
    "auth_tag",
    "hash_hex",
    "stream_word",
    "FNV_OFFSET",
    "FNV_PRIME",
    "xor-fnv64-stream",
    "fnv64-passphrase",
]

REQUIRED_BACKUP_TERMS = [
    "cortexdb.xchacha20poly1305-argon2id.v2",
    "cortexdb.argon2id.v1",
    "cortexdb.encrypted_backup.v2",
    "xchacha20poly1305_seal",
    "xchacha20poly1305_open",
    "derive_argon2id_key",
]

REQUIRED_TEST_TERMS = [
    "encrypted_backup_legacy_v1_archive_is_refused",
    "encrypted_backup_tamper_matrix_is_rejected_without_target",
]

REQUIRED_MAKE_TERMS = [
    "ENCRYPTED_BACKUP_LEGACY_REFUSE_REPORT ?= target/encrypted-backup-legacy-refuse/report.json",
    "encrypted-backup-legacy-refuse-check:",
    "cargo test -p cortex-engine encrypted_backup_legacy_v1_archive_is_refused",
    'python3 scripts/encrypted_backup_legacy_refuse_check.py --root "." --report "$(ENCRYPTED_BACKUP_LEGACY_REFUSE_REPORT)"',
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
    crypto = read_text(root, "crates/cortex-engine/src/backup/encrypted/crypto.rs")
    tests = read_text(root, "crates/cortex-engine/tests/backup_restore.rs")
    makefiles = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted((root / "mk").glob("*.mk"))
    )
    failures: list[str] = []
    failures.extend(forbidden_terms("encrypted backup crypto", crypto, FORBIDDEN_BACKUP_TERMS))
    failures.extend(missing_terms("encrypted backup crypto", crypto, REQUIRED_BACKUP_TERMS))
    failures.extend(missing_terms("backup_restore.rs", tests, REQUIRED_TEST_TERMS))
    failures.extend(missing_terms("mk targets", makefiles, REQUIRED_MAKE_TERMS))
    failures.extend(missing_terms("mk/phony.mk", read_text(root, "mk/phony.mk"), [
        "encrypted-backup-legacy-refuse-check",
    ]))
    return {
        "schema_version": "cortexdb.encrypted_backup_legacy_refuse.report.v1",
        "status": "failed" if failures else "passed",
        "production_safe": not failures,
        "checked": {
            "forbidden_backup_terms": FORBIDDEN_BACKUP_TERMS,
            "required_backup_terms": REQUIRED_BACKUP_TERMS,
            "required_test_terms": REQUIRED_TEST_TERMS,
            "required_make_terms": REQUIRED_MAKE_TERMS,
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
    print(f"encrypted backup legacy refuse check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
