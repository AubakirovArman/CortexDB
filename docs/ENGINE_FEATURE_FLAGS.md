# Engine Feature Flags

CortexDB keeps Core Alpha defaults conservative. Experimental surfaces must be
enabled explicitly so the default single-node path remains production-safe.

## Engine Flags

`EngineFeatureFlags::production_safe()` is the default for `DatabaseOptions`.

| Flag | Default | Scope |
| --- | --- | --- |
| `experimental_hnsw` | `false` | Builds and uses persisted `.ach` HNSW graphs. When disabled, persisted vector search uses exact `.acv` scan with an ANN fallback report. |
| `experimental_replication` | `false` | Enables database-level replication snapshot/install helpers. Consensus primitives remain available for tests and design work. |
| `dashboard` | `false` | Reserved engine-level product flag for dashboard-aware integrations. The HTTP server has its own matching startup flag. |

Example:

```rust
use cortex_engine::{Database, DatabaseOptions, EngineFeatureFlags};

let options = DatabaseOptions {
    feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
    ..DatabaseOptions::default()
};
let db = Database::open_with_options("./data", options)?;
# Ok::<(), cortex_engine::EngineError>(())
```

## Server Flags

The server also defaults to production-safe engine flags. Enable experimental
server surfaces explicitly through environment variables:

```bash
CORTEXDB_EXPERIMENTAL_HNSW=true cortex-server ./data 127.0.0.1:8080
CORTEXDB_EXPERIMENTAL_REPLICATION=true cortex-server ./data 127.0.0.1:8080
CORTEXDB_DASHBOARD=true cortex-server ./data 127.0.0.1:8080
```

`CORTEXDB_EXPERIMENTAL_HNSW=true` allows the server-opened database to build and
use `.ach` graphs. `CORTEXDB_EXPERIMENTAL_REPLICATION=true` allows
database-level snapshot install APIs used by replication repair flows. Without
`CORTEXDB_DASHBOARD=true`, `/dashboard` and dashboard assets are not served.

## Invariants

1. Default `DatabaseOptions` must not build HNSW graphs.
2. Default vector search must remain functional through exact persisted vectors.
3. Database-level replication snapshot/install must fail closed unless
   `experimental_replication=true`.
4. Dashboard routes must be absent unless enabled explicitly.
5. Adding a new experimental product surface must add a feature flag, test, and
   documentation entry.
