# Search v1

`cortex-engine` exposes two search layers:

- `SearchIndexes` for standalone keyword/vector/hybrid scoring tests.
- `Database::search_keyword` / `Database::search_cells` for scoped database
  search over the current visible snapshot.
- `.aci` lexical files persist term postings, per-candidate document lengths,
  and weighted term-frequency statistics.
- `.acv` vector files persist per-candidate integer vectors for exact dot scan.
  The current `ACV1` layout stores one candidate table plus contiguous
  fixed-dimension i16 rows so exact search can scan disk rows without building
  a resident vector map.
- `.ach` HNSW graph files persist the current graph links for persisted
  vector search.

The database path filters cells through `AgentView.readable_scopes`, assigns
compact candidate ids internally, and returns full `CellId` values in
`DatabaseSearchResult`.

Permission filtering is a pre-ranking predicate, not a result post-filter.
Snapshot search builds its temporary index only from cells readable by the
provided `AgentView`. Persisted search derives an allowed candidate set from the
checkpointed bitmap scope index and passes it into lexical, vector, and HNSW
search before scoring, fusion, reranking, or top-k truncation. This prevents an
unreadable high-scoring candidate from consuming a result slot or entering
ContextPack.

When live segments exist and there are no uncheckpointed changes after the
manifest checkpoint sequence, keyword search reads the persisted `.aci` postings
directly and vector search reads persisted `.acv` vectors plus `.ach` graph
links. If a WAL tail has newer put/patch/tombstone records, the engine falls
back to the visible MemTable snapshot so fresh writes are not missed.

Keyword scoring uses canonical BM25 with deterministic Q16 fixed-point
statistics. Defaults are `k1=1.2` and `b=0.75`; field weights are applied to
per-field BM25 contributions when `.aci` field term frequencies are available.
`title` has weight 8, `path` has weight 5, and body text has weight 1.
Persisted `.aci` files store the same term, field, and length statistics so
checkpointed search keeps the same ranking signals as snapshot search. The full
formula and defaults are documented in [`SCORING.md`](SCORING.md).

`TextAnalyzer` supports Unicode alphanumeric tokenization, field weights,
stopwords, weighted terms, deterministic MRR checks, and built-in
English/Russian/Kazakh analyzer packs. `TextAnalyzerConfig` controls the
collection analyzer language and whether light suffix stemming is enabled.
The default analyzer is neutral and keeps stemming disabled for backward
compatibility. When stemming is enabled for Russian, inflections such as
`бюджету` normalize to `бюджет`. Custom lemma overrides can map normalized
terms into domain dictionaries. The packs are deterministic and dependency-light,
not full morphological analyzers.

Checkpoint, compact, replication snapshot install, live snapshot search,
persisted `.aci` search, and AQL delta merge use the configured analyzer. The
manifest stores a text analyzer profile (`ANLZ`) with analyzer version, language,
and stemming flag. Opening a database with a different analyzer profile is
rejected when persisted segments exist, which prevents mixed token streams inside
one collection. Databases without persisted segments can switch analyzer config
before their first checkpoint.

`analyze_search_query` is the engine-side query understanding primitive. It
reads only the user query text and extracts enterprise anchors such as ticket
ids, PR numbers, file paths, versions, dates, numeric values, quoted phrases,
and source hints like GitHub/Jira/Gmail/Slack when they are explicitly present
in the question. Keyword and hybrid search use its weighted query terms, so
domain expansions such as `blocked -> dependency/risk` and
`owner -> assignee/DRI` affect retrieval without relying on benchmark
`question_type`, `source_types`, expected document ids, or other oracle labels.

`SearchReranker` is the engine-side rerank hook. `SearchIndexes` and
`Database::search_cells_with_reranker` can collect a wider candidate pool,
compute an additional rerank score, and then apply the requested limit. The
built-in `WeightedScoreReranker` is deterministic and dependency-free. Custom
implementations receive query text/vector, original lexical/vector scores, the
candidate id, and, at the database layer, the candidate payload. Scope/ACL
filtering happens before reranking, so a reranker cannot promote unreadable
cells into the result set.

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
cortexdb vector rebuild <path> [--experimental-hnsw]
cortexdb search-vector <path> <scope> <i16-vector>
cortexdb search-vector-eval <path> <scope> <i16-vector>
POST /v1/search?scope=<scope>&q=<query>
POST /v1/search?scope=<scope>&mode=hybrid&q=<query>&vector=<i16-vector>
POST /v1/search/explain?scope=<scope>&mode=hybrid&q=<query>&vector=<i16-vector>
POST /v1/search?scope=<scope>&mode=vector&vector=<i16-vector>
POST /v1/search?scope=<scope>&mode=vector&algorithm=exact&vector=<i16-vector>
POST /v1/search?scope=<scope>&mode=vector&algorithm=ann&vector=<i16-vector>
POST /v1/search/ann-evaluate?scope=<scope>&vector=<i16-vector>
```

The HTTP body is used as the keyword query text when `q` is omitted, and as the
vector literal when `mode=vector&vector=...` is omitted. Vector literals accept
comma or space separated signed 16-bit integers.

`cortexdb vector rebuild` is the local recovery command for persisted ANN
bundles. It rereads live `.acs` segment cells, rewrites `.acv` vector indexes,
and, when `--experimental-hnsw` is set or the manifest already expects HNSW,
rewrites `.ach` graph files from the same cells. The command finishes by running
storage validation, so checksum corruption, vector dimension mismatch, or
stale graph/vector mismatches are repaired before the command reports success.

`algorithm=exact` forces the disk-resident `.acv` exact scan path. Dense
allowed sets are read sequentially through a buffered row scan; sparse sets seek
only to matching rows. Dot-product scoring uses a stable chunked i16 loop so it
stays deterministic on stable Rust. `algorithm=ann` requests the persisted
`.ach` HNSW graph and now applies a correctness guard: empty
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
  "fallback_performed": true,
  "requested_limit": 20,
  "allowed_candidates": 1,
  "graph_nodes": 0,
  "returned_candidates": 1,
  "recall_q16": null,
  "min_recall_q16": null,
  "hnsw_max_neighbors": 0,
  "hnsw_ef_search": 0,
  "hnsw_ef_construction": 0,
  "hnsw_layer_count": 1,
  "upper_graph_edges": 0,
  "require_slo": true,
  "production_safe": false,
  "slo_violations": ["no_persisted_segments"]
}
```

`path` is either `hnsw_graph` or `exact_fallback`. `fallback_reason` is `null`
when the persisted HNSW graph is used, or one of `empty_graph`,
`invalid_graph`, `insufficient_results`, `low_recall`,
`visit_budget_exceeded`, `no_persisted_segments`, or `uncheckpointed_changes`
when exact scan is used instead. `fallback_performed` shows whether exact scan
actually served the result. `recall_q16` and `min_recall_q16` are populated when
the exact top-k recall guard runs. `low_recall` is emitted when HNSW returns
enough candidates but fails that guard, preserving result correctness while ANN
quality is still being tuned.

Set `require_slo=true` on CLI/HTTP ANN search and ANN evaluation calls to make
the report explicit about production guardrails. `production_safe=false` and
`slo_violations` identify graph, fallback, recall, and visit-budget violations.
The result still returns using the configured fallback policy; callers that need
hard SLO enforcement should reject responses where `production_safe=false`.

When an operator explicitly passes `no_fallback_rollout=true`, or uses the
persisted profile with `no_fallback_profile=active`, vector ANN responses
include `no_fallback_decision`. This does not remove exact fallback globally.
It only reports whether the current request, ANN policy, and `ann_report`
satisfy the selected fallback-free rollout guard:

```json
{
  "no_fallback_decision": {
    "allowed": false,
    "reasons": ["fallback_enabled", "slo_not_required"]
  }
}
```

Operators can persist the local profile through CLI or the admin HTTP endpoint:

```bash
cortexdb hnsw-no-fallback-profile-set --min-recall 100% ./data
cortexdb search-vector-eval --fallback false --require-slo \
  --use-no-fallback-profile ./data project:investments "0.1,0.2"
```

```http
PUT /v1/admin/search/hnsw/no-fallback-profile
{"rollout_enabled":true,"min_recall_q16":65535,"require_upper_layers":true}
```

For ANN quality work, `Database::evaluate_vector_ann` compares the persisted
HNSW path with exact `.acv` vector scan for the same `AgentView`, query vector,
and limit. It returns an `AnnEvaluationReport` with exact top-k ids, ANN top-k
ids, overlap count, and fixed-point `recall_q16`. The evaluator only runs when
there is a persisted checkpoint and no newer WAL tail, so the exact and ANN
baselines are comparing the same durable snapshot. The CLI and HTTP evaluation
surfaces expose the same data and return `available=false` until this durable
snapshot precondition is met.

## HNSW Tuning Parameters & Guarded Alpha Guidelines

### 1. Hard Constraints & Dimension Enforcement
- **Dimension Homogeneity:** All vectors inside a single `.acv` partition must share a single, non-zero dimension. The write path validates each incoming payload's vector shape via `HnswIndex::add_vector`; mismatches return `EngineError::VectorDimensionMismatch` so they cannot happen silently.
- **Fixed-Point Score Stability:** All distance metrics operate on `i16` fixed-point embeddings (Q16 scaling representation). Cosine similarity uses integer-only fixed-point approximation (`dot * 65_535 / sqrt_norm`) without `f64` arithmetic.

### 2. Supported Distance Metrics
`HnswIndex` supports three distance metrics, selectable per collection:
- **`DotProduct` (default):** Non-negative dot-product similarity. Higher is better.
- **`Cosine:`** Cosine similarity scaled to `[0, 65_535]`. Higher is better. Automatically handles zero-length vectors.
- **`L2:`** Negative squared Euclidean distance. Higher (less negative) is better. Computed as `max_dist - dist_sq` so scores remain comparable across queries.

### 3. Tuning Parameters
- **`max_neighbors` (M):** The maximum number of bidirectional connection links per node in the HNSW graph (default: 8). Higher values improve search quality (recall) on high-dimensional vectors but increase graph build time and memory usage during compaction.
- **`ef_search` (EF):** The size of the dynamic candidate list kept during the graph traversal phase (default: 64). Increasing `ef_search` improves recall but adds a linear search latency cost.
- **`ef_construction`:** The build-time candidate beam used while selecting graph neighbors during checkpoint/compact (default profile-dependent; balanced is 128). Increasing it can improve graph quality at checkpoint/compact cost.
- **`deleted_fraction_q16`:** HNSW rebuild threshold (default: `16,384`, representing 25% deletion pressure). When deleted vectors exceed this fraction, `HnswIndex::apply_maintenance` triggers a graph rebuild and increments `rebuild_count`.
- **`MIN_ANN_RECALL_Q16`:** Production recall guard (default: `49_151`, representing 75%). If HNSW traversal recall falls below this threshold, the engine falls back to exact vector scan to preserve correctness.

### 4. ANN Metrics & Observability
`GET /v1/ann/metrics` and `cortexdb ann validate` expose:
- `graph_nodes` / `total_edges` — graph size
- `persisted_segments` — number of live segments
- `has_checkpoint` / `has_uncheckpointed_changes` — durability state
- `deleted_vectors` — tombstoned vectors across all live segments
- `rebuild_count` — number of graph rebuilds performed

`GET /v1/metrics` also exposes operator counters for fallback-free rollout
decisions:

- `ann_no_fallback_requests` — ANN responses that included a no-fallback
  rollout decision
- `ann_no_fallback_allowed` — decisions that allowed serving for the selected
  profile
- `ann_no_fallback_blocked` — decisions blocked by recall, graph, fallback, or
  SLO guardrails
- `ann_search_latency_ms` — cumulative live latency buckets for ANN-capable
  search responses

### 5. Limitations & Fail-Safes
- **Static Rebuild Lifecycle:** Graphs are built deterministically during the `checkpoint`/`compact` phase and remain static in `.ach` files. Real-time updates inside the MemTable (WAL tail) bypass HNSW and are merged on-the-fly using exact scan, ensuring 100% freshness and correctness.
- **Profile-aware graph construction:** `DatabaseOptions::hnsw_build_config` controls the HNSW shape written by checkpoint/compact. Use wider profiles for semantic/audit workloads and compact after a profile change to rebuild existing `.ach` files.
- **Exact fallback guardrails:** If a graph is empty, structurally invalid, corrupt/truncated on disk, stale relative to the persisted vector index, returns insufficient candidates, or fails the 75% recall guard, the system automatically degrades to exact scan. Fallback reasons are exposed in `ann_report.path` and `ann_report.fallback_reason`.
- **Recall benchmark fixtures:** Unit fixture gates assert that checkpointed ANN evaluation meets `MIN_ANN_RECALL_Q16`, and the `core_baseline` benchmark includes an ANN recall section (`ann_recall_q16_1k`, `ann_graph_nodes_1k`, `ann_eval_latency_1k`) for regression tracking.
- **Repeatable ANN gate:** `make ann-fixture-check` runs the deterministic synthetic corpus in release mode and compares recall, graph shape, upper-layer shape, p95/p99/max latency ceilings, `production_safe`, and zero fallback against `crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json`. `make ann-drift-check` compares the current report against `ann_drift_baseline_v1.json` to catch recall, graph-shape, p99 tail-latency, and max-latency regressions. `make ann-external-check` runs a checked-in JSONL corpus fixture so HNSW is also tested against explicit non-generated vectors and named queries. `make ann-metric-matrix-check` evaluates dot-product, cosine, and L2 rows against exact top-k with the same metric. `make ann-reference-suite-check` validates synthetic, explicit JSONL, and domain reports against checked-in external-reference SLO fixtures with recall-ratio, latency-ratio, upper-layer, `production_safe`, and zero-fallback requirements. `ann_corpus_check` accepts external vectors/queries/ground-truth JSONL files for larger recall suites; the file contract is documented in [`ANN_CORPUS_FORMAT.md`](archive/ANN_CORPUS_FORMAT.md). CI uploads all checked-in ANN reports as `ann-regression-reports`.
- **Query understanding lift gate:** `make enterprise-rag-bench-query-understanding-lift-check` runs a clean fixture with only documents, questions, and expected document ids. It compares a plain lexical baseline with real engine keyword search, so query expansion and anchor handling must improve recall without reading benchmark oracle fields such as `question_type` or `source_types`.
- **Guarded production mode:** ANN/HNSW exposes recall, fallback, visit-budget, and graph-validity SLO signals through `ann_report`. Checkpoint and compact now build deterministic multi-layer `.ach` graphs, and `core_baseline` emits `ann_repeatable_report_json` for recall/latency history. Production tuning policy is documented in [`ANN_PRODUCTION_TUNING.md`](archive/ANN_PRODUCTION_TUNING.md). `make ann-recall-probe-report` repeats the local domain corpus gate, and `make ann-production-slo-history-check` builds a fresh 10-run local SLO history to catch recall, latency, or graph-shape regressions before no-fallback rollout. Large external corpora and real production traffic history remain future work. Exact vector scan remains the most predictable default for critical workloads.
