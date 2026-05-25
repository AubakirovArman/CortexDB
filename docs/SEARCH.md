# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings and per-candidate document lengths.
- `.acv` vector files persist per-candidate integer vectors for exact dot scan.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

When live segments exist and there are no uncheckpointed changes after the
manifest checkpoint sequence, keyword search reads the persisted `.aci` postings
directly and vector search reads persisted `.acv` vectors directly. If a WAL
tail has newer put/patch/tombstone records, the engine falls back to the visible
MemTable snapshot so fresh writes are not missed.

Current smoke surfaces:

```text
cortexdb search <path> <scope> <query>
cortexdb search-vector <path> <scope> <i16-vector>
POST /v1/search?scope=<scope>&q=<query>
POST /v1/search?scope=<scope>&mode=vector&vector=<i16-vector>
```

The HTTP body is used as the keyword query text when `q` is omitted, and as the
vector literal when `mode=vector&vector=...` is omitted. Vector literals accept
comma or space separated signed 16-bit integers.

## Not Yet

- Production BM25 analyzers.
- Persistent ANN/HNSW vector pages.
