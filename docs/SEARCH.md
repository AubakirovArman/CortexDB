# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings, per-candidate document lengths,
  and weighted term-frequency statistics.
- `.acv` vector files persist per-candidate integer vectors for exact dot scan.
- `.ach` HNSW graph files persist the current experimental graph links.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

When live segments exist and there are no uncheckpointed changes after the
manifest checkpoint sequence, keyword search reads the persisted `.aci` postings
directly and vector search reads persisted `.acv` vectors directly. If a WAL
tail has newer put/patch/tombstone records, the engine falls back to the visible
MemTable snapshot so fresh writes are not missed.

Keyword scoring uses deterministic integer statistics. Body text has weight 1;
an optional `title=` payload header has weight 6. Persisted `.aci` files store
the same weighted term frequencies so checkpointed search keeps the same ranking
signals as snapshot search.

`TextAnalyzer` is the first analyzer layer for quality fixtures. It supports
field weights, stopwords, weighted terms, and deterministic MRR checks without
floating-point scoring. This is still intentionally simple, but it gives search
changes a repeatable quality gate.

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

- Language-specific stemming and production tokenizer packs.
- Production ANN search over persisted HNSW pages.
