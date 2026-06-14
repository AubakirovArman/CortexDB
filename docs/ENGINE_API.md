# Cortex Engine API

Status: Epic 26 embedded API freeze gate.

`cortex-engine` is the embedded Rust facade over AQL, core MVCC, storage, WAL,
checkpoint/compact, validation, search, ContextPack, verification, ingestion,
and local memory.

## Stable Embedded API

The stable embedded API for the current boundary is the documented public
facade imported from the crate root:

```rust
use cortex_engine::{
    BackupReport,
    CandidateId,
    CheckpointStats,
    ContextPack,
    ContextPackOptions,
    Database,
    DatabaseOptions,
    EngineConfig,
    EngineConfigError,
    DbOperation,
    EngineAqlIndex,
    EngineError,
    EngineErrorCategory,
    EngineErrorCode,
    EngineFeature,
    EngineFeatureFlags,
    EngineResult,
    Language,
    RecoveryMode,
    RepairReport,
    RestoreReport,
    RetrievedCell,
    StaleLockPolicy,
    StorageStats,
    StorageValidationReport,
    TextAnalyzer,
    TextAnalyzerConfig,
};
```

The machine-readable freeze contract is:

```text
fixtures/engine/public_api_freeze_v1.json
```

The contract names the stable facade symbols, the helper modules that must
remain private, the required rustdoc example sources, and the evidence gates
that must pass before changing the boundary.

The central entrypoints are:

- `cortex_engine::Database::open`;
- `Database::open_with_options`;
- `Database::put_cell`;
- `Database::patch_cell`;
- `Database::tombstone_cell`;
- `Database::get_latest_cell`;
- `Database::checkpoint`;
- `Database::compact`;
- `Database::validate_storage`;
- `Database::storage_stats`;
- `Database::aql_query_cache_stats`;
- `Database::backup_path`;
- `Database::restore_from_backup`;
- `Database::repair_best_effort`;
- `Database::repair_best_effort_dry_run`.

`EngineConfig::from_env()` is the shared configuration loader for CLI/server
entrypoints that need env-driven `DatabaseOptions`. Embedded callers may still
construct `DatabaseOptions` directly for deterministic local setup.
`DatabaseOptions::text_analyzer` selects the collection text analyzer profile
for new writes, checkpointed lexical indexes, and query analysis. The default is
the neutral analyzer with stemming disabled. Non-default language stemming
profiles are persisted in the manifest and must match on reopen once live
segments exist.

Production-safe feature defaults are documented in
[`ENGINE_FEATURE_FLAGS.md`](archive/ENGINE_FEATURE_FLAGS.md). Experimental HNSW,
database-level replication, and dashboard surfaces must be enabled explicitly.

The stable root-level types currently frozen are:

- `Database`, `DatabaseOptions`, `EngineConfig`, `EngineConfigError`,
  `RecoveryMode`, `StaleLockPolicy`;
- `EngineFeature`, `EngineFeatureFlags`;
- `Language`, `TextAnalyzer`, `TextAnalyzerConfig`;
- `EngineError`, `EngineResult`;
- `EngineErrorCode`, `EngineErrorCategory`;
- `DbOperation`;
- `ContextPack`, `ContextPackOptions`;
- `StorageStats`, `StorageValidationReport`;
- `RepairReport`;
- `BackupReport`, `RestoreReport`;
- `RetrievedCell`, `CheckpointStats`;
- `AqlQueryCacheStats`;
- `CandidateId`, `EngineAqlIndex`.

The stable API boundary is source-level Rust compatibility for local embedded
users. It is not a C ABI, network protocol, or promise that every re-exported
module is permanently stable.

## Internal APIs

Internal APIs are modules or functions used by the engine implementation but
not promised as stable for external callers:

- checkpoint internals;
- candidate allocation internals;
- persisted index merge helpers;
- low-level replication internals;
- search implementation modules;
- cleanup/lock/database-file helpers;
- raw storage writer internals.

External callers should prefer `Database`, option structs, report structs, and
typed result/error surfaces exported from `cortex_engine::`.

The following helper modules are intentionally private and checked by the
freeze gate:

- `cleanup`;
- `database_files`;
- `lock`;
- `options`.

## Compatibility Gate

Run:

```bash
make engine-public-api-freeze-check
make engine-api-compat-check
make engine-error-model-check
make engine-feature-flags-check
make engine-api-check
```

This gate verifies:

- stable docs exist;
- `fixtures/engine/public_api_freeze_v1.json` matches crate-root exports;
- frozen public symbols are documented and compile-tested;
- known helper modules remain private;
- rustdoc examples exist for the embedded database facade;
- engine errors expose stable code, category, HTTP status, safe message, and
  CLI hint metadata;
- engine feature flags keep experimental HNSW, replication, and dashboard
  surfaces opt-in;
- the public API compile test passes;
- the external sample crate compiles and runs through open, put/get, search,
  ContextPack, VERIFY, checkpoint, backup, and restore;
- `cortex-engine` doctests compile;
- rustdoc builds for `cortex-engine`.

Report:

```text
target/engine-api/report.json
```

## Breaking-Change Policy

A PR that changes the stable embedded API should update:

- this document;
- `MODULE_OWNERSHIP.md`;
- Rust examples or doctests;
- SDK/API docs if behavior crosses HTTP or client packages;
- release notes for the target version.

If an API remains experimental, document it as internal or experimental instead
of silently exposing it as stable.
