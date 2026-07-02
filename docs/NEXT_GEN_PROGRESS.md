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

## Track A — Retrieval quality & embeddings

| Task | State | What landed | Not yet (remaining plan scope) | Gate |
| --- | --- | --- | --- | --- |
| A1.1 Score-fusion scale fix | Landed | `min_max_normalize_q16` + `weighted_retrieval_score` in `retrieval_rank.rs`: all four signals (lexical, semantic, recency, trust) min-max normalized to Q16 before per-mode weighting; no-spread components map to `u16::MAX` to preserve multiplicative decay signals. Cross-mode adversarial tests (Fast lexical, Audit trust, recency tie-break). | A1.2/A1.3/A1.4 (corpus BM25 stats, cosine parity re-verification, weight re-derivation). | `retrieval-recall-baseline-check` |
| A2 (client) HTTPS embedding | Landed | `embed_query_with_config` rewritten on `ureq` (tls) for `http`+`https`; i16 quantization `clamp(-1,1)*i16::MAX`; ignored `live_embed_query_over_https` test (live-verified). | — | server tests |
| A2.1 Embedder adapter | Slice | Engine `Embedder` trait + offline `DeterministicTestEmbedder` (splitmix64/fnv1a64) in `embedding_pipeline/adapter.rs`, exported from `lib.rs`. | `EmbeddingProfile` in manifest; per-cell `embedding_ref` (profile+model+dim+content-hash); profile-mismatch typed error at `open`; goldens re-baselined for the new canonical fields. | (covered by ingest tests) |
| A2.2 Auto-embed at ingest | Slice | Engine `ingest_text_chunks_with_embedder(..)` writes a `vector=` payload header per chunk through an injected `&dyn Embedder` (engine stays network-free). Server `HttpEmbedder` + `embedder_from_env()`; opt-in `POST /v1/ingest/text?embed=true`, parsed fail-closed. Hermetic + ignored live end-to-end test (live-verified writing a `vector=` header over HTTPS). | Batch-embed (batch 64, config); `allow_unembedded=true` job-report escape; checkpoint ACV1/ACH0 interaction; idempotent content-hash backfill; ERB-50 engine-hybrid gate with **no** external vector files; 10k-doc test-embedder → working HNSW; ANN recall gates; crash-between-embed-and-write fail-closed. | `ingest_embedding` tests |
| A2.0 Embedding-model selection | Not started | Empirical input recorded: [`ERB_EMBEDDING_EVIDENCE.md`](ERB_EMBEDDING_EVIDENCE.md) shows dense BGE-M3 vectors did **not** move ERB semantic recall (flat ~32.8→33.6), so the semantic gap is not a pure embedding-pipeline gap. | Formal 2–3 candidate eval on ERB-50 + LongMemEval compact-50 (recall@10/MRR, latency, index size at Q15 i16); frozen profile before A2.2 hardcodes one. | `embedding-model-selection-check` (planned) |
| A2.3 Query-side text→vector | Not started | Server query path already accepts `embed_query=true` (search/context/explain). | CLI `--mode hybrid\|semantic` without `--vector` when an adapter is configured; explain records vector source (literal vs embedded + profile); prior fail-closed text preserved with no config. | `cli-embedded-query-check` (planned) |

## Cross-cutting landings

| Item | State | Notes | Gate |
| --- | --- | --- | --- |
| B1.1 Feedback silent-overwrite | Landed | `next_feedback_cell_id` probes for a free id in a loop (mirrors session-id allocation) so a second feedback write cannot clobber the first. | engine tests |
| TE1 `search` MCP tool | Landed | Permission-scoped `search` tool across `tools.rs`/`server.rs`/`sdk_executor.rs` + `MCP.md`. | mcp tests |
| Track F baseline | Landed | Per-PR `retrieval-recall-baseline-check` with cross-mode adversarial fixtures; foundation for detecting silent recall regressions. | `retrieval-recall-baseline-check` |

## Reading this ledger

The distinction that matters here is *slice* vs *full task*. The ingest/query
embedding path is usable end-to-end today, but the plan's A2.1/A2.2 also require
profile provenance in the manifest, idempotent backfill, and checkpoint/HNSW
interaction proofs before the embedding capability is contract-grade. Those
remain future work and are tracked in the *Not yet* columns above.
