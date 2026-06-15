# Engine Config Model

Status: Epic 34 engine config model.

`EngineConfig` is the shared configuration loader for CortexDB entrypoints that
open a local database from process environment. It keeps CLI and server startup
aligned while preserving the embedded Rust API: callers can still construct
`DatabaseOptions` directly when they need deterministic setup in tests or local
applications.

## Embedded API

```rust
use cortex_engine::{Database, EngineConfig};

let config = EngineConfig::from_env()?;
let db = Database::open_with_options("./data", config.database_options())?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Environment Variables

| Variable | Default | Values | Effect |
| --- | --- | --- | --- |
| `CORTEXDB_DURABILITY_MODE` | `strict` | `strict`, `balanced` | WAL fsync behavior for newly opened databases. |
| `CORTEXDB_RECOVERY_MODE` | `strict` | `strict`, `best_effort` | WAL recovery policy. |
| `CORTEXDB_STALE_LOCK_POLICY` | `reject` | `reject`, `break` | Existing `db.lock` behavior on open. |
| `CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES` | `128` | non-negative integer | Maximum entries in the bounded FIFO AQL query-plan cache. |
| `CORTEXDB_WAL_ARCHIVE` | `false` | bool | Copy closed `db.*.aclog` files into `wal_archive/` after checkpoint or compact. |
| `CORTEXDB_WAL_ARCHIVE_MAX_FILES` | `1024` | non-negative integer | Maximum retained archived WAL files; values below 1 are clamped to 1. |
| `CORTEXDB_HNSW_PROFILE` | `balanced` | `fast`, `balanced`, `semantic`, `audit` | HNSW graph build profile when HNSW is enabled. |
| `CORTEXDB_EXPERIMENTAL_HNSW` | `false` | bool | Enables persisted `.ach` graph build/use. |
| `CORTEXDB_EXPERIMENTAL_REPLICATION` | `false` | bool | Enables database-level replication snapshot/install helpers. |
| `CORTEXDB_DASHBOARD` | `false` | bool | Enables dashboard-aware engine/server surfaces. |

Boolean values accept `true`, `false`, `1`, `0`, `yes`, `no`, `on`, and `off`.
Invalid configured values fail startup instead of silently falling back.

## CLI and Server

The CLI and HTTP server both use `EngineConfig::from_env()` when opening local
databases. CLI flags such as `--experimental-hnsw` can still enable specific
experimental paths for a command, but they do not disable an environment-enabled
feature.

Examples:

```bash
CORTEXDB_RECOVERY_MODE=best_effort cortexdb validate ./data
CORTEXDB_DURABILITY_MODE=balanced cortexdb put ./data 1 "scope=docs\nstatus=ready\nhello"
CORTEXDB_EXPERIMENTAL_HNSW=true cortex-server ./data 127.0.0.1:8080
```

## Invariants

1. `DatabaseOptions::default()` remains production-safe and env-independent.
2. `EngineConfig::from_env()` is the only env parser for engine database
   options used by CLI/server startup.
3. Invalid env values must fail explicitly.
4. New engine-level env variables must be documented here and covered by tests.
