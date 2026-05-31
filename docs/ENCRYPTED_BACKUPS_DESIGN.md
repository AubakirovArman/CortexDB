# Encrypted Backups Design

Status: design only, not implemented.

This document defines the intended encrypted-backup model for the single-node
product line. Core Alpha currently supports validated local filesystem backups
and local offsite staging; those backups are not encrypted by CortexDB itself.

## Goals

- Add encryption to backup artifacts without changing live storage formats.
- Keep backup validation and restore drills deterministic.
- Make key handling explicit and auditable.
- Preserve current `cortexdb backup`, `restore`, `backup-drill`, and
  `backup-offsite-stage` workflows with clear encrypted variants or flags.

## Proposed Format

Encrypted backup bundles should use envelope encryption:

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

The first implementation should support local operator-managed key files for
offline testing. Production deployments should use an external KMS or secret
manager through a small provider trait.

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

## Not Implemented

The current repository does not implement encrypted backups yet. The existing
hardening gate checks this design boundary and keeps the public claim honest:

```bash
make production-hardening-check
```

Future implementation work should add:

- encrypted backup command flags or subcommands;
- provider trait and local-file provider;
- encrypted backup restore tests;
- corrupted ciphertext/authentication failure tests;
- docs and OpenAPI/CLI contract updates.
