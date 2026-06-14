# Core Engine

The `cortex-engine` core owns local database open, write, read, checkpoint,
compaction, validation, search, AQL retrieval, ContextPack, verification, and
repair flows.

External embedded callers should enter through `cortex_engine::Database` and
the stable crate-root types listed in `docs/ENGINE_API.md`.

`Database::open` creates the embedded database handle. `PutCell` operations and
`Database::put_cell` append durable cell writes through the WAL before they are
visible to reads.

Storage bytes and manifest compatibility are tracked separately in
`docs/STORAGE_FORMATS.md`.
