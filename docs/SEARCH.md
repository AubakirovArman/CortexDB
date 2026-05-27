# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings, per-candidate document lengths,
  and weighted term-frequency statistics.
- `.acv` vector files persist per-candidate integer vectors for exact dot scan.
- `.ach` HNSW graph files persist the current graph links for persisted
  vector search.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

When live segments exist and there are no uncheckpointed changes after the
manifest checkpoint sequence, keyword search reads the persisted `.aci` postings
directly and vector search reads persisted `.acv` vectors plus `.ach` graph
links. If a WAL tail has newer put/patch/tombstone records, the engine falls
back to the visible MemTable snapshot so fresh writes are not missed.

Keyword scoring uses deterministic integer statistics. Body text has weight 1;
an optional `title=` payload header has weight 6. Persisted `.aci` files store
the same weighted term frequencies so checkpointed search keeps the same ranking
signals as snapshot search.

`TextAnalyzer` supports field weights, stopwords, weighted terms, deterministic
MRR checks, and built-in English/Russian/Kazakh analyzer packs. The language
packs include light suffix stemmers and stopword lists. Custom lemma overrides
can map normalized terms into domain dictionaries. The packs are deterministic
and dependency-light, not full morphological analyzers.

`HnswIndex::apply_maintenance` reports deleted-vector pressure and rebuilds the
graph when `HnswMaintenancePolicy` thresholds are reached. This gives the engine
a deterministic lifecycle hook without a background scheduler thread.
`HnswIndex::integrity_report` also checks structural graph health: zero
candidate ids, link nodes missing from the vector index, neighbor links missing
from the vector index, self-links, and deleted-vector references. Storage
validation cross-checks `.ach` graph links against `.acv` vector candidates so
a corrupted or mismatched ANN bundle is reported before query execution.
Validation also checks that all vectors inside one persisted `.acv` file share
one non-empty dimension. Exact vector scan and HNSW scoring skip vectors whose
dimension differs from the query rather than comparing only a shared prefix.

Current smoke surfaces:

```text
cortexdb search <path> <scope> <query>
cortexdb search-vector <path> <scope> <i16-vector>
cortexdb search-vector-eval <path> <scope> <i16-vector>
POST /v1/search?scope=<scope>&q=<query>
POST /v1/search?scope=<scope>&mode=vector&vector=<i16-vector>
POST /v1/search?scope=<scope>&mode=vector&algorithm=exact&vector=<i16-vector>
POST /v1/search?scope=<scope>&mode=vector&algorithm=ann&vector=<i16-vector>
POST /v1/search/ann-evaluate?scope=<scope>&vector=<i16-vector>
```

The HTTP body is used as the keyword query text when `q` is omitted, and as the
vector literal when `mode=vector&vector=...` is omitted. Vector literals accept
comma or space separated signed 16-bit integers.

`algorithm=exact` forces the `.acv` exact scan path. `algorithm=ann` requests
the persisted `.ach` HNSW graph and now applies a correctness guard: empty
graphs, invalid graph links, and graph traversals that return fewer candidates
than the requested visible set fall back to exact vector scan. Search responses
include `search_mode` so clients can record whether keyword, exact vector, or
ANN vector search was requested.

Search responses also include `ann_report`. It is `null` for keyword and exact
vector search. For `algorithm=ann`, it records the actual path:

```json
{
  "path": "exact_fallback",
  "fallback_reason": "no_persisted_segments",
  "requested_limit": 20,
  "allowed_candidates": 1,
  "graph_nodes": 0,
  "returned_candidates": 1,
  "recall_q16": null,
  "min_recall_q16": null
}
```

`path` is either `hnsw_graph` or `exact_fallback`. `fallback_reason` is `null`
when the persisted HNSW graph is used, or one of `empty_graph`,
`invalid_graph`, `insufficient_results`, `low_recall`,
`no_persisted_segments`, or `uncheckpointed_changes` when exact scan is used
instead. `recall_q16` and `min_recall_q16` are populated when the exact top-k
recall guard runs. `low_recall` is emitted when HNSW returns enough candidates
but fails that guard, preserving result correctness while ANN quality is still
being tuned.

For ANN quality work, `Database::evaluate_vector_ann` compares the persisted
HNSW path with exact `.acv` vector scan for the same `AgentView`, query vector,
and limit. It returns an `AnnEvaluationReport` with exact top-k ids, ANN top-k
ids, overlap count, and fixed-point `recall_q16`. The evaluator only runs when
there is a persisted checkpoint and no newer WAL tail, so the exact and ANN
baselines are comparing the same durable snapshot. The CLI and HTTP evaluation
surfaces expose the same data and return `available=false` until this durable
snapshot precondition is met.

## HNSW Tuning Parameters & Production Guidelines

### 1. Hard Constraints & Dimension Enforcement
- **Dimension Homogeneity:** All vectors inside a single `.acv` partition must share a single, non-zero dimension. The write path validates each incoming payload's vector shape. If any vector has mismatched dimensions (detected on checkpoint/compaction), the indexing pipeline logs a validation mismatch and gracefully falls back to `exact_fallback`.
- **Fixed-Point Score Stability:** Dot-product and similarity scoring operate strictly on non-negative `i16` fixed-point embeddings (Q16 scaling representation), completely avoiding non-deterministic floating-point (`f64`) operations.

### 2. Tuning Parameters
- **`max_neighbors` (M):** The maximum number of bidirectional connection links per node in the HNSW graph (default: 8). Higher values improve search quality (recall) on high-dimensional vectors but increase graph build time and memory usage during compaction.
- **`ef_search` (EF):** The size of the dynamic candidate list kept during the graph traversal phase (default: 64). Increasing `ef_search` improves recall but adds a linear search latency cost.
- **`deleted_fraction_q16`:** HNSW rebuild threshold (default: `16,384`, representing 25% deletion pressure). When deleted vectors exceed this fraction, `HnswIndex::apply_maintenance` triggers a graph rebuild.

### 3. Limitations & Fail-Safes
- **Static Rebuild Lifecycle:** Graphs are built during the `checkpoint` (compaction) phase and remain static in the `.ach` files. Real-time updates inside the MemTable (WAL tail) bypass HNSW and are merged on-the-fly using `Exact Scan`, ensuring 100% freshness and correctness.
- **Zero-Recall exact fallback:** If a graph traversal yields fewer candidates than the requested visible set, or if the index validation detects corruption, the system automatically degrades to `Exact Scan` (`fallback_reason: "low_recall" / "uncheckpointed_changes"`), preserving exact accuracy.
