"""Validation helpers for backup/restore production-pack reports."""

from __future__ import annotations

from typing import Any


def _require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def validate_encrypted_backup(
    report: dict[str, Any] | None, errors: list[str]
) -> dict[str, Any]:
    if report is None:
        errors.append("encrypted backup report missing")
        return {}
    _require(report.get("status") == "ok", "encrypted backup status is not ok", errors)
    _require(
        report.get("plaintext_hidden") is True,
        "encrypted backup plaintext hiding evidence missing",
        errors,
    )
    _require(
        report.get("wrong_passphrase_rejected") is True,
        "encrypted backup wrong-passphrase rejection missing",
        errors,
    )
    _require(
        report.get("corrupt_ciphertext_rejected") is True,
        "encrypted backup corrupt-ciphertext rejection missing",
        errors,
    )
    return {
        "archive_path": report.get("archive_path"),
        "backup_duration_ms": report.get("backup_duration_ms"),
        "restore_duration_ms": report.get("restore_duration_ms"),
        "plaintext_hidden": report.get("plaintext_hidden"),
        "wrong_passphrase_rejected": report.get("wrong_passphrase_rejected"),
        "corrupt_ciphertext_rejected": report.get("corrupt_ciphertext_rejected"),
        "boundary": report.get("boundary"),
    }


def validate_encrypted_backup_rotation(
    report: dict[str, Any] | None, errors: list[str]
) -> dict[str, Any]:
    if report is None:
        errors.append("encrypted backup rotation report missing")
        return {}
    _require(
        report.get("status") == "ok",
        "encrypted backup rotation status is not ok",
        errors,
    )
    required_flags = [
        (
            "old_backup_decrypts_with_old_passphrase",
            "old backup does not decrypt with old passphrase",
        ),
        (
            "new_backup_decrypts_with_new_passphrase",
            "new backup does not decrypt with new passphrase",
        ),
        (
            "old_backup_rejects_new_passphrase",
            "old backup does not reject new passphrase",
        ),
        (
            "new_backup_rejects_old_passphrase",
            "new backup does not reject old passphrase",
        ),
        ("old_archive_plaintext_hidden", "old archive plaintext hiding missing"),
        ("new_archive_plaintext_hidden", "new archive plaintext hiding missing"),
    ]
    for key, message in required_flags:
        _require(report.get(key) is True, message, errors)
    return {
        "old_archive_path": report.get("old_archive_path"),
        "new_archive_path": report.get("new_archive_path"),
        "old_backup_decrypts_with_old_passphrase": report.get(
            "old_backup_decrypts_with_old_passphrase"
        ),
        "new_backup_decrypts_with_new_passphrase": report.get(
            "new_backup_decrypts_with_new_passphrase"
        ),
        "old_backup_rejects_new_passphrase": report.get(
            "old_backup_rejects_new_passphrase"
        ),
        "new_backup_rejects_old_passphrase": report.get(
            "new_backup_rejects_old_passphrase"
        ),
        "rotation_policy": report.get("rotation_policy"),
        "boundary": report.get("boundary"),
    }
