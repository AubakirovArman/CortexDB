# Encrypted Backups Design

Status: local MVP implemented for passphrase-protected archive/restore drills.

This document defines the intended encrypted-backup model for the single-node
product line. Core Alpha already supports validated local filesystem backups
and local offsite staging. The beta-track MVP adds a single-file passphrase
archive through:

```bash
export CORTEXDB_BACKUP_PASSPHRASE="choose-a-long-local-passphrase"
cortexdb backup-encrypted ./data ./backup.cdbenc --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
cortexdb restore-encrypted ./backup.cdbenc ./restore --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
```

The MVP is intended for local release evidence and operator workflow testing.
It is not a KMS-backed or compliance-certified encryption system.

## Goals

- Add encryption to backup artifacts without changing live storage formats.
- Keep backup validation and restore drills deterministic.
- Make key handling explicit and auditable.
- Preserve current `cortexdb backup`, `restore`, `backup-drill`, and
  `backup-offsite-stage` workflows with clear encrypted variants or flags.

## Proposed Format

The long-term production format should use envelope encryption:

```text
backup/
  manifest.json
  data.tar.zst.enc
  encryption.json
  checksums.json
```

`encryption.json` should record:

- schema version;
- cipher suite;
- key wrapping provider;
- key id or alias;
- nonce/IV metadata;
- authenticated additional data fields;
- creation timestamp;
- encrypted data key.

The encrypted payload must be authenticated. Restore must reject a backup when
authentication fails before writing to the target database path.

## Key Management

The implemented MVP uses an operator-managed passphrase. Production deployments
should use an external KMS or secret manager through a small provider trait.

Required provider operations:

```text
generate_data_key()
wrap_data_key()
unwrap_data_key()
provider_id()
key_id()
```

Private key material must never be stored in backup manifests, logs, JSON
reports, or audit files.

## Restore Flow

1. Read and validate the encrypted backup manifest.
2. Verify `encryption.json` and backup checksums.
3. Unwrap the encrypted data key through the configured provider.
4. Decrypt into a temporary restore directory.
5. Run normal `Database::validate_storage`.
6. Atomically publish the restored target only after validation passes.

## Operational Policy

- Encrypted backups should have their own drill gate before beta promotion.
- Offsite staging should reject unencrypted backups when an encrypted backup
  policy is enabled.
- Error messages must not expose key ids beyond safe aliases.
- Audit logs should record backup operation metadata, not key material.

## Implemented MVP

The current repository implements:

- `Database::encrypted_backup_path`;
- `Database::restore_from_encrypted_backup`;
- `cortexdb backup-encrypted`;
- `cortexdb restore-encrypted`;
- passphrase validation;
- archive ciphertext integrity checks;
- wrong-passphrase and corrupt-ciphertext rejection tests;
- restore validation with `Database::validate_storage`.
- repeatable evidence through `make encrypted-backup-check`, written to
  `target/encrypted-backup/report.json`.
- archive-scoped passphrase rotation evidence through
  `make encrypted-backup-rotation-check`, written to
  `target/encrypted-backup-rotation/report.json`.

The current archive format is a CortexDB-local beta format with clear header
metadata and encrypted payload bytes. Operators should prefer `--passphrase-env`
or `CORTEXDB_BACKUP_PASSPHRASE` so passphrases do not need to be typed directly
into shell history.

The evidence gate verifies:

- encrypted archive creation;
- correct-passphrase restore and validation;
- fixture payload bytes are not visible in the archive bytes;
- wrong-passphrase restore fails without creating the target path;
- corrupt-ciphertext restore fails without creating the target path.

## Rotation Policy MVP

Passphrase rotation is archive-scoped in the current MVP:

1. Create a fresh backup archive with the new passphrase.
2. Verify the fresh archive can restore with the new passphrase.
3. Keep older archives decryptable with their original passphrase until the
   retention window expires.
4. Verify cross-key restore attempts fail safely: old archives must reject the
   new passphrase, and new archives must reject the old passphrase.
5. Retire old passphrases only after every archive that needs them has expired
   or has been replaced by a newly encrypted backup.

The repeatable evidence gate is:

```bash
make encrypted-backup-rotation-check
```

It proves old-backup decrypt, new-backup encrypt/decrypt, cross-key fail-safe
behavior, and plaintext hiding for both archive generations.

## Not Implemented

The current repository still does not implement:

- provider trait and local-file provider;
- KMS-backed data-key wrapping;
- KMS-backed key rotation;
- externally auditable authenticated-encryption proof;
- remote object-store upload;
- compliance-grade backup custody workflow.
