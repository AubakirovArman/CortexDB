# Next-Gen Plan — Progress Ledger

This is the running, honest record of what has actually landed against
[`NEXT_GEN_MASTER_PLAN.md`](NEXT_GEN_MASTER_PLAN.md). It distinguishes a landed
**vertical slice** from the plan's full task scope, so the delta stays visible
rather than being rounded up to "done". Each row links its regression gate.

Status vocabulary:

- **Landed** — merged to `main`, gated, and (where noted) live-verified.
- **Slice** — a usable subset merged; the plan task's remaining scope is listed
  under *Not yet* so it is not mistaken for full completion.
- **Not started** — no code yet.

## Milestone — text-in → context-out loop closed over HTTP

The end-to-end embedding loop works over the HTTP surface and is live-verified
against a real provider (`text_in_context_out_loop_is_closed_live`, ignored by
default):

1. **Write side** — `POST /v1/ingest/text?embed=true` embeds each chunk at
   ingest, or `POST /v1/embedding/backfill` embeds an already-ingested corpus
   idempotently by content hash.
2. **Read side** — `POST /v1/search?embed_query=true` (also `/v1/context`,
   `/v1/search/explain`) embeds a natural-language query at request time, so a
   caller retrieves by meaning without ever supplying a literal `vector=`.

Every step is fail-closed: with no `CORTEXDB_EMBEDDING_*` endpoint configured,
each embedding entry point returns `bad_request` rather than silently degrading.
What remains (see *Not yet* columns) is the contract-grade layer — embedding
profile provenance in the manifest and the CLI-side query embedder.

## Track A — Retrieval quality & embeddings

| Task | State | What landed | Not yet (remaining plan scope) | Gate |
| --- | --- | --- | --- | --- |
| A1.1 Score-fusion scale fix | Landed | `min_max_normalize_q16` + `weighted_retrieval_score` in `retrieval_rank.rs`: all four signals (lexical, semantic, recency, trust) min-max normalized to Q16 before per-mode weighting; no-spread components map to `u16::MAX` to preserve multiplicative decay signals. Cross-mode adversarial tests (Fast lexical, Audit trust, recency tie-break). | A1.2/A1.3/A1.4 (corpus BM25 stats, cosine parity re-verification, weight re-derivation). | `retrieval-recall-baseline-check` |
| A2 (client) HTTPS embedding | Landed | `embed_query_with_config` rewritten on `ureq` (tls) for `http`+`https`; i16 quantization `clamp(-1,1)*i16::MAX`; ignored `live_embed_query_over_https` test (live-verified). | — | server tests |
| A2.1 Embedder adapter | Landed | Engine `Embedder` trait + offline `DeterministicTestEmbedder`. **Provenance (contract-grade):** manifest gains an additive `EMBD` section recording the `EmbeddingProfile` (model/dimension/metric); each embedded cell carries an `embedding_ref=emb1:<model>:<dim>:<metric>:<hash>` payload header (ingest + backfill share one formatter, byte-identical); `Database::open` fails closed on a model/dim/metric mismatch. Additive + backward-compatible: pre-A2.1 DBs open unchanged, and accountability/pack/determinism goldens are untouched (ref lives in payload text, not the descriptor) — verified green. The profile is kept in lockstep with the vector profile (cleared when vectors are gone; stale labels dropped on unconfigured rebuild) and open rejects an internally-inconsistent manifest — both hardened after an adversarial review. | Receipt integration (promote `embedding_ref` into the canonical receipt) is deferred to a C3-5 minor schema bump; the frozen experimental-replication install path does not yet re-stamp the follower profile (documented limitation). | `storage-format-freeze-check` |
| A2.2 Auto-embed at ingest | Slice | Engine `ingest_text_chunks_with_embedder(..)` writes a `vector=` payload header per chunk through an injected `&dyn Embedder` (engine stays network-free). Server `HttpEmbedder` + `embedder_from_env()`; opt-in `POST /v1/ingest/text?embed=true`, parsed fail-closed. **Corpus backfill** `POST /v1/embedding/backfill` drives the engine's pre-existing idempotent, content-hash-keyed `backfill_embedding_debt_batched` over HTTP (batched, `max_items` bound, fail-closed on bad params/no config). Hermetic + ignored live end-to-end tests (live-verified: `embed=true` writes a `vector=` header over HTTPS; backfill embeds an existing cell then converges to zero debt on re-run). | `allow_unembedded=true` job-report escape; checkpoint ACV1/ACH0 interaction; ERB-50 engine-hybrid gate with **no** external vector files; 10k-doc test-embedder → working HNSW; ANN recall gates; crash-between-embed-and-write fail-closed. | `ingest_embedding` tests |
| A2.0 Embedding-model selection | Landed | Selection harness (`scripts/embedding_model_eval.py`, deterministic recall@k/MRR/index-size math + self-test) validates the candidate matrix in `fixtures/embedding/model_selection_v1.json` and asserts the recorded `chosen` profile is the measured winner. **Chosen: `BAAI/bge-m3`, dim 1024, `dot_product`** (67.5 vs 55.7 overall doc recall). Write-up: [`EMBEDDING_MODEL_SELECTION.md`](EMBEDDING_MODEL_SELECTION.md), honest caveat that dense does not fix the `semantic` pool. | Live multi-model eval over ERB-50 + LME-50 (harness supports offline math; broader live candidates deferred). | `embedding-model-selection-check` |
| A2.3 Query-side text→vector | Landed | Server (`/v1/search`, `/v1/context`, `/v1/search/explain`) embeds the query when `embed_query=true`; MCP `search mode=semantic\|hybrid\|auto`; and offline **CLI** `cortexdb search --mode vector\|hybrid\|auto` embeds the query text (no `--vector` needed) via the new shared `cortex-embed-client` crate — the single embedding transport for server **and** CLI, so query and stored vectors are quantized identically and can't drift. Fail-closed everywhere with no endpoint; the CLI reports `vector_source=literal\|embedded\|none`. Live-verified against the real provider. | — | `cli-embedded-query-check` |
| A3.1 Graph-descent HNSW insert | Landed | Standard-HNSW insert replacing the O(N²) full-scan build: deterministic entry point → greedy upper-layer descent → `ef_construction` beam per layer → diversity neighbor-selection heuristic + bounded-degree linking. **Key fix:** `keepPrunedConnections` backfill is applied ONLY to the new node's out-edges (density) and NOT when pruning existing neighbors (which would evict long-range bridges) — with M0 = 2·M on layer 0. This gives **recall 1.0 on the dim-8 synthetic production gate** (`ann-fixture-check`) and 0.95+ on realistic high-dim clustered data, byte-identical double build, bounded degree, and integrity after build+remove. Determinism/pack goldens unchanged (HNSW behind the experimental flag). | — | `hnsw-build-scaling-check` |
| A4.1 Temporal recency window | Slice | Data-relative temporal window in the recency signal (`retrieval_rank.rs`): cells created within `window` seconds of the newest candidate rank as fully fresh; the reference time is the newest created timestamp (no wall-clock, INV-3), so it stays deterministic. Wired via `DatabaseOptions.retrieval_recency_window_seconds`, **off by default** (goldens byte-identical, verified). Recency was already relative/deterministic; this adds the window. | Per-query AQL temporal window + explicit `reference_time` injection (needs the C3-1/C3-5 plan-option protocol); temporal supersession (A4 rest). | `retrieval-temporal-window-check` |
| A5 MMR diversification | Slice | Deterministic integer-Q16 Maximal Marginal Relevance reorder (`retrieval_rank/diversify.rs`): first pick = top relevance, then `λ·rel + (1−λ)·(1−max_sim_to_selected)`; no floats / wall-clock (INV-3), ties to lower index. Wired as a `DiversifyOp` after ranking, gated by `DatabaseOptions.retrieval_diversify_lambda_q16` — **off by default**, so all existing goldens are byte-identical (verified). Unit + RetrievedCell end-to-end tests. | AQL surface (`DIVERSIFY λ`) as a plan-visible option (needs the C3-1/C3-5 canonical-set protocol + golden re-baseline); explain contribution of the diversity term. | `retrieval-diversify-check` |

## Cross-cutting landings

| Item | State | Notes | Gate |
| --- | --- | --- | --- |
| B1.1 Feedback silent-overwrite | Landed | `next_feedback_cell_id` probes for a free id in a loop (mirrors session-id allocation) so a second feedback write cannot clobber the first. | engine tests |
| TE1 `search` MCP tool | Landed | Permission-scoped `search` tool across `tools.rs`/`server.rs`/`sdk_executor.rs` + `MCP.md`. Now supports `mode=semantic\|hybrid\|auto`: the agent sends text only and the server embeds the query (`embed_query=true`) — semantic search reaches the MCP surface. SDK gains `search_embedded` (sync + async). | mcp tests |
| Track F baseline | Landed | Per-PR `retrieval-recall-baseline-check` with cross-mode adversarial fixtures; foundation for detecting silent recall regressions. | `retrieval-recall-baseline-check` |

## Reading this ledger

The distinction that matters here is *slice* vs *full task*. The ingest, query,
and corpus-backfill embedding paths are usable end-to-end today, and the store
now records embedding-profile provenance and fails closed on a model/dim/metric
mismatch (A2.1). What remains is promoting `embedding_ref` into the canonical
accountability receipt (a C3-5 minor schema bump) and A2.2's checkpoint/HNSW
interaction proofs. Those are tracked in the *Not yet* columns above.
