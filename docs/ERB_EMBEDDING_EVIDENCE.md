# EnterpriseRAG-Bench — local Gemma-judged evidence & dense-embedding analysis

Boundary: this is **local interim diagnostic evidence, not a leaderboard or
production claim**. The judge is a local `google/gemma-4-31B-it`, **not** the
official `gpt-5.4` judge, so these numbers are **not comparable** to other
systems' results or to a `gpt-5.4`-judged submission. They exist only for
internal diagnosis. See [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md).

## 1. Headline run, re-judged with local Gemma

The existing `erb-submission/answers.jsonl` (500 answers, `google/gemma-4-31B-it`
answerer, `official-clean-v1` prompt, `question-window-digest-ranked` context)
re-judged with a local Gemma judge:

| Metric | Gemma judge | (prior Gemini-3.5-flash judge) |
| --- | ---: | ---: |
| Combined | 46.65 | 47.74 |
| Correctness | 49.0% | 50.0% |
| Completeness | 53.3% | 53.7% |
| Doc recall | 55.7% | 55.71% |

Two different judges (Gemma vs Gemini) score the same answers within ~1 point,
so the interim number is judge-robust — the judge is not the source of variance.

## 2. Per-type diagnosis (weakest pools first)

| Question type | Combined | n |
| --- | ---: | ---: |
| info_not_found | 100.0 | 20 |
| miscellaneous | 81.5 | 20 |
| conflicting_info | 68.8 | 20 |
| intra_document_reasoning | 65.8 | 40 |
| constrained | 64.5 | 30 |
| basic | 52.1 | 175 |
| **semantic** | **27.4** | **125** |
| completeness | 23.5 | 20 |
| high_level | 20.0 | 10 |
| **project_related** | **13.2** | **40** |

`semantic` (n=125) and `project_related` (n=40) are the largest / weakest pools.

## 3. Dense-embedding (BAAI/bge-m3) retrieval analysis

Comparing lexical-only retrieval (headline) against a dense+lexical hybrid run
(`run_dense_hybrid_clean.sh`, pre-embedded `corpus_bge_m3.jsonl`), both judged by
local Gemma. **Doc recall** isolates retrieval quality:

| Question type | Lexical recall | Dense-hybrid recall | Δ |
| --- | ---: | ---: | ---: |
| **Overall** | 55.7 | **67.5** | **+11.8** |
| basic | 63.4 | 77.7 | +14.3 |
| intra_document_reasoning | — | 97.5 | up |
| constrained | — | 95.0 | up |
| conflicting_info | 70.0 | 82.5 | +12.5 |
| project_related | 47.9 | 60.4 | +12.5 |
| **semantic** | 32.8 | **33.6** | **+0.8 (flat)** |
| high_level | 0.0 | 0.0 | — |

## 4. Conclusions (honest, data-driven)

1. **Dense `bge-m3` embeddings broadly improve retrieval recall (+11.8 overall)**
   — so completing the engine's embedding path is worthwhile (see the
   HTTPS-capable embedding client change).
2. **Dense embeddings do NOT help the `semantic` type** (recall flat at ~33%).
   The largest weak pool is not a dense-similarity problem, so "add embeddings"
   is **not** its fix — a different approach (e.g. query decomposition /
   multi-hop) is needed. This corrects the earlier hypothesis that embeddings
   would lift the `semantic` pool.
3. **The combined ERB score is answer/context-config-bound.** The dense-hybrid
   run raised recall (55.7 → 67.5) yet its combined score fell (46.65 → 36.18)
   because it used a less-tuned answer/context config (correctness 40.6 vs
   49.0). Retrieval-recall gains alone do not move the combined number; the
   answer/context layer dominates it.

## 5. Reproduction

Endpoints are read from `.env` (kept out of the repo): `VLLM_*` for the Gemma
answerer/judge, `CORTEXDB_EMBEDDING_*` for the LiteLLM `BAAI/bge-m3`
(dense / `-sparse` / `-colbert`) provider.

```bash
# Re-judge existing answers with local Gemma:
python3 scripts/enterprise_rag_bench/run_official_clean_judge.py \
  --provider gemma \
  --answers-file erb-submission/answers.jsonl \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --results-file <out>/results.json --judgments-file <out>/judgments.jsonl --workers 6

# Dense+lexical hybrid end-to-end (answer + judge = Gemma):
SIZE=500 scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh
```

Next levers implied by this evidence are documented in
[`NEXT_GEN_MASTER_PLAN.md`](NEXT_GEN_MASTER_PLAN.md): the answer/context layer
(dominates the combined score) and a targeted `semantic`-type retrieval fix
(query decomposition / multi-hop), **not** more embeddings.
