# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings and per-candidate document lengths
  for later persisted BM25 execution.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

Current smoke surfaces:

```text
cortexdb search <path> <scope> <query>
POST /v1/search?scope=<scope>&q=<query>
```

The HTTP body is used as the query text when `q` is omitted.

## Not Yet

- Dedicated search over `.aci` without snapshot rebuild.
- Production BM25 analyzers.
- Public vector input over CLI/HTTP.
