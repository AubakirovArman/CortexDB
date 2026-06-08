# EnterpriseRAG-Bench First-50 Iteration

This note tracks the current local first-50 EnterpriseRAG-Bench optimization loop.

## Current Best Local Run

- Retrieval: `cortexdb_first_50_hybrid_v2_top5.jsonl`
- Answer model: `google/gemma-4-31B-it` through local vLLM
- Prompt style: `type-aware-v17`
- Context mode: `leading`
- Context budget: `8000` chars/doc
- Top-k context: `5`

Local Gemma-judge metrics:

| Metric | Value |
|---|---:|
| Answer Correctness | 78.0 |
| Answer Completeness | 74.7 |
| Combined Correctness/Completeness | 58.27 |
| Document Recall | 84.0 |
| Invalid Extra Docs | 4.16 |
| Generation tokens | 444,326 |
| Generation wall time | 93,454 ms |

## Compared Runs

| Run | Correctness | Completeness | Combined | Recall | Invalid Extra |
|---|---:|---:|---:|---:|---:|
| previous official Gemma-50 | 70.0 | 70.04 | 61.76 | 85.39 | 8.6 |
| top5 v17 windowed ranked | 66.0 | 63.44 | 41.87 | 84.0 | 4.16 |
| top5 v17 window8000 | 76.0 | 72.6 | 55.18 | 84.0 | 4.16 |
| top10 v17 window5000 | 76.0 | 69.68 | 52.96 | 90.0 | 9.1 |
| top3 v17 window8000 | 74.0 | 68.0 | 50.32 | 80.0 | 2.2 |
| top5 v15 window8000 | 76.0 | 70.1 | 53.28 | 84.0 | 4.16 |
| top5 v17 leading8000 | 78.0 | 74.7 | 58.27 | 84.0 | 4.16 |
| top5 v17 evidence-spans8000 | 56.0 | 59.22 | 33.16 | 84.0 | 4.16 |
| top5 v17 evidence-spans8000-v2 | 72.0 | 67.45 | 48.56 | 84.0 | 4.16 |
| top5 v17 leading8000 max_tokens360 | 76.0 | 74.28 | 56.45 | 84.0 | 4.16 |

The previous official Gemma-50 result was scored through the official evaluator path,
while the new rows above use the local Gemma-compatible judge wrapper. Treat them as
directional until the official judge path is configured for the same judge model.

## Product Takeaways

- Broad retrieval is not the main first-50 blocker: top500 contained almost all gold
  documents, while final top-k pruning caused most misses.
- Hybrid reranking improved first-50 retrieval recall from the earlier top10 baseline
  shape to 84 percent at top5 and 90 percent at top10.
- More documents are not always better. Top10 improves recall but adds noise and lowers
  answer completeness versus top5 in the local Gemma-judge loop.
- Windowing matters as much as document IDs. `leading8000` outperformed digest-ranked
  windows because several answers live later in email threads and transcripts.
- Materialized evidence spans are useful as an agent-database primitive, but not yet
  safe as the only context source. The first span-only run missed exact facts because
  long PR/email blocks were truncated before the answer anchor. The v2 anchor-centered
  trim fixed concrete misses such as `stream.timebox_finalized`, raising correctness
  from 56 to 72, but standalone spans still lost completeness versus `leading8000`.
- Increasing output `max_tokens` did not beat the current best run. That points to
  evidence packaging and retrieval misses as the next bottleneck, not answer length.

## Architecture Implication

For CortexDB as an AI-agent database, the target product behavior is not raw vector
top-k and not benchmark-specific prompt tuning. The engine should materialize:

1. broad candidate recall;
2. compact candidate-to-cell/document provenance;
3. evidence spans with selection signals;
4. fallback source windows when spans are not confidence-safe;
5. a dynamic ContextPack budget policy that chooses top-k and span/window mix by
   query type, source type, and rerank confidence.

The current `evidence-spans` mode should therefore be treated as a building block
for ContextPack, not as the default EnterpriseRAG answer context yet.

## Iteration Artifacts

- `scripts/enterprise_rag_bench/evidence_spans.py`: deterministic materialized
  evidence-span extractor with anchor-centered trimming.
- `target/enterprise-rag-bench/qa/vllm-gemma-first50-hybrid-v2-top5-v17-evidence-spans8000/`:
  first span-only run.
- `target/enterprise-rag-bench/qa/vllm-gemma-first50-hybrid-v2-top5-v17-evidence-spans8000-v2/`:
  anchor-centered span-only run.
- `target/enterprise-rag-bench/qa/vllm-gemma-first50-hybrid-v2-top5-v17-leading8000-maxtok360/`:
  higher answer-token-budget control run.

## Next Work

1. Add a dynamic document budget policy instead of fixed top3/top5/top10.
2. Add a span-plus-fallback context mode: evidence spans first, then source windows
   when span confidence is weak or coverage is incomplete.
3. Improve reranking for remaining misses where gold is in raw top50 but outside final top5.
4. Add an OpenAI-compatible official judge adapter only for local vLLM if official-score parity
   is required without using OpenAI keys.
5. Re-run first-50 after each change, then expand to 100 and 500 only after a real first-50 gain.
