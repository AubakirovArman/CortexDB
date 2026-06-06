# Engine Error Model v1

Status: Epic 28 stable engine error model.

`cortex-engine` exposes `EngineError` for embedded Rust callers. The stable
machine-readable surface is:

```rust
error.code().as_str()
error.category().as_str()
error.http_status()
error.safe_message()
error.cli_hint()
```

The freeze contract is:

```text
fixtures/engine/error_model_v1.json
```

## Stable Codes

| Code | Category | HTTP status | Meaning |
| --- | --- | --- | --- |
| `bad_request` | `user_input` | `400` | The caller supplied an invalid operation, option, payload, fixture, vector, or backup target. |
| `invalid_aql` | `user_input` | `400` | AQL parse or non-policy bind failure. |
| `permission_denied` | `permission` | `403` | AQL policy denied scope, brain, mode, budget, memory type, remember, verify, or audit mode. |
| `forbidden` | `permission` | `403` | Local permission failure such as filesystem permission denial. |
| `not_found` | `not_found` | `404` | A cell, path, or requested resource is missing. |
| `database_busy` | `busy` | `503` | The database lock or actor queue is busy. |
| `storage_corruption` | `corruption` | `500` | Storage checksum, WAL, manifest, segment, missing-file, or invariant failure. |
| `service_unavailable` | `unavailable` | `503` | The local node cannot currently serve the operation, for example a non-leader write. |
| `internal` | `internal` | `500` | Internal invariant or unexpected error not safely classified above. |

## Engine Variant Mapping

| `EngineError` variant | Stable code |
| --- | --- |
| `Core` | `not_found` |
| `Storage` | `storage_corruption` |
| `BitmapVm` | `internal` |
| `AqlParse` | `invalid_aql` |
| `AqlBind` | `invalid_aql_or_permission_denied` |
| `Io` | `not_found_or_forbidden_or_internal` |
| `InvalidOperation` | `bad_request` |
| `FeatureDisabled` | `bad_request` |
| `MissingWalSection` | `storage_corruption` |
| `MissingCommitSeq` | `storage_corruption` |
| `FatalCellMissingAfterWal` | `storage_corruption` |
| `MissingStorageFile` | `storage_corruption` |
| `StorageInvariant` | `storage_corruption` |
| `InvalidAnnFixture` | `bad_request` |
| `InvalidAnnCorpus` | `bad_request` |
| `CandidateIdOverflow` | `internal` |
| `VectorDimensionMismatch` | `bad_request` |
| `HnswBuildConfigOutOfRange` | `bad_request` |
| `InvalidCandidateId` | `storage_corruption` |
| `DatabaseAlreadyOpen` | `database_busy` |
| `BackupTargetExists` | `bad_request` |
| `NotLeader` | `service_unavailable` |

## Adapter Rules

- CLI output should use `EngineError::cli_hint()` instead of matching ad-hoc
  strings in command handlers.
- HTTP routing should convert engine errors through `EngineError::code()` and
  `EngineError::safe_message()`.
- SDK-visible HTTP codes remain the subset documented in
  [`API_ERROR_TAXONOMY.md`](API_ERROR_TAXONOMY.md).
- Adding or reclassifying an `EngineError` variant requires updating the JSON
  fixture, this document, tests, and release notes.

## Gate

Run:

```bash
make engine-error-model-check
make engine-api-check
make openapi-contract-check
```

Reports:

```text
target/engine-error-model/report.json
```
