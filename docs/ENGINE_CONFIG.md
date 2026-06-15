# Engine Config

`EngineConfig` is the env-driven loader for embedded engine startup settings.
It produces `DatabaseOptions` for callers that want the same configuration
surface as the CLI and server.

Stable options include durability recovery mode, stale lock policy, WAL archive
retention, compaction policy, payload residency, feature flags, and the text
analyzer profile.

WAL archive options:

| Variable | Default | Values | Effect |
| --- | --- | --- | --- |
| `CORTEXDB_WAL_ARCHIVE` | `false` | bool | Copy closed `db.*.aclog` files into `wal_archive/` after checkpoint or compact so backups can restore with `--to-seq`. |
| `CORTEXDB_WAL_ARCHIVE_MAX_FILES` | `1024` | non-negative integer | Maximum retained archived WAL files. Values below 1 are clamped to 1. |

`DatabaseOptions::text_analyzer` defaults to a neutral analyzer with stemming
disabled. Non-default analyzer profiles are persisted in the storage manifest
after checkpoint/compact and must match on reopen while persisted segments
exist.
