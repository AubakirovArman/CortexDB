# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings and per-candidate document lengths.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

When live segments exist and there are no uncheckpointed changes after the
manifest checkpoint sequence, keyword search reads the persisted `.aci` postings
directly. If a WAL tail has newer put/patch/tombstone records, the engine falls
back to the visible MemTable snapshot so fresh writes are not missed.

Current smoke surfaces:

```text
cortexdb search <path> <scope> <query>
POST /v1/search?scope=<scope>&q=<query>
```

The HTTP body is used as the query text when `q` is omitted.

## Not Yet

- Production BM25 analyzers.
- Public vector input over CLI/HTTP.
