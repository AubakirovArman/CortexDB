# Upgrade and Migration Policy

Version: `v0.1.0-core-alpha`

CortexDB Core Alpha is still experimental, but storage and API changes must be
handled as explicit compatibility events. This document defines the minimum
operator workflow and release gate for upgrading a single-node database.

## Scope

This policy covers:

- API contract changes under `/v1`;
- SDK release compatibility;
- storage format changes for `.aclog`, `.acs`, `.acb`, `.aci`, `.acv`,
  `.ach`, and `.acm`;
- backup, restore, rollback, and validation commands.

It does not claim online rolling upgrades, multi-node compatibility, or
production Raft migration support.

## Format Compatibility Matrix

| File | Current marker | Compatibility rule |
| --- | --- | --- |
| WAL `.aclog` | `ACLOGv0` | Breaking changes require a WAL version bump and migration note. |
| Segment `.acs` | `ACS1` | Breaking changes require a new segment magic and migration note. |
| Bitmap index `.acb` | `ACB0` | Breaking changes require a new bitmap magic and migration note. |
| Lexical index `.aci` | `ACI2` | `ACI0` and `ACI1` remain read-only compatible. |
| Vector index `.acv` | `ACV0` | Breaking changes require a new vector magic and migration note. |
| HNSW graph `.ach` | `ACH0` | Breaking changes require a new graph magic and migration note. |
| Manifest `.acm` | `ACM0` | Breaking changes require a new manifest magic and migration note. |

The detailed binary layouts live in [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md).

## Upgrade Workflow

For Core Alpha, upgrades are offline and single-node:

1. Stop writers and the HTTP server.
2. Run `cortexdb validate ./db`.
3. Run a backup drill:

   ```bash
   cortexdb backup-drill ./db ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade
   ```

4. Keep the pre-upgrade backup immutable until the upgraded database has passed
   validation and smoke checks.
5. Install the new binary.
6. Open the database with the new binary.
7. Run:

   ```bash
   cortexdb validate ./db
   cortexdb stats ./db
   make smoke-test
   ```

8. Run the relevant SDK/OpenAPI checks before publishing a release:

   ```bash
   make openapi-contract-check
   make sdk-contract-check
   make migration-policy-check
   make migration-compatibility-check
   ```

## Rollback Workflow

Rollback is restore-based:

1. Stop the new binary.
2. Move the upgraded database aside for inspection.
3. Restore the immutable pre-upgrade backup:

   ```bash
   cortexdb restore ./backups/cortexdb-pre-upgrade ./db.rollback
   cortexdb validate ./db.rollback
   ```

4. Start the previous binary against the restored directory.

Do not downgrade in place. Core Alpha does not guarantee that a newer binary
will leave on-disk files readable by older binaries after it has written new
segments, indexes, WAL records, or manifests.

## Migration Note Requirements

Any PR or release that changes storage, API, or SDK compatibility must include a
migration note. The note must state:

- affected format or endpoint;
- whether the change is additive or breaking;
- required backup/restore steps;
- rollback behavior;
- validation commands;
- whether older files remain read-only compatible.

For storage changes, update:

- `docs/STORAGE_FORMATS.md`;
- this document;
- `docs/API_CHANGELOG.md` if an API or SDK response changes;
- release notes for the target version.

For API changes, update:

- `docs/openapi.yaml`;
- `docs/API_JSON_SCHEMAS.md`;
- `docs/API_COMPATIBILITY.md`;
- SDK contract tests and response snapshots.

## Release Gate

The migration policy gate is:

```bash
make migration-policy-check
```

It verifies that this policy exists, names the current storage markers, includes
backup/restore/rollback instructions, and remains wired into the release check.

The compatibility fixture gate is:

```bash
make migration-compatibility-check
```

It validates `fixtures/migration/compatibility_matrix_v1.json`, including:

- storage/API/SDK compatibility boundaries;
- current and old read-only format markers;
- an offline upgrade/downgrade matrix;
- at least one historical restore fixture whose old backup is restored and
  validated by the current binary;
- proof files that back each compatibility claim.

Core Alpha release candidates should pass:

```bash
make release-check
```

## Current Limitations

- No in-place downgrade guarantee.
- No online migration while serving writes.
- No built-in remote object-store migration.
- No automatic data rewrite tool yet; current storage changes must preserve
  read compatibility or document a manual restore/rebuild path.
