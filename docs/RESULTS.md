# CortexDB — Machine-Verifiable Results

Every headline number on this page is checked against a **committed** evidence
artifact by [`scripts/results_page_check.py`](../scripts/results_page_check.py)
(gate `results-page-check`): each row carries a hidden `verify:` annotation that
re-reads the number from its source JSON, so a published result can never drift
from the evidence that produced it. Numbers whose evidence lives only under
`target/` (regenerated, gitignored) are described narratively and linked to their
write-up, not machine-verified here.

## Retrieval-augmented QA — EnterpriseRAG-Bench (official 500)

The end-to-end product path (retrieve through `cortex_engine::Database` → generate
→ judge), scored by the official ERB metrics with a Gemini judge of record.

| Metric | Value | Source |
| --- | --- | --- |
| Combined correctness × completeness | **47.74** | `erb-submission/official_results.json` |
| Document recall | 55.71% | same |
| Answer correctness | 50.0% | same |
| Answer completeness | 53.7% | same |
| Questions scored | 500 | same |

> **Judge caveat (read before citing 47.74).** This is an **interim** number, judged
> by `gemini-3.5-flash` — **not** the leaderboard-official `gpt-5.4` ERB evaluator.
> A three-judge cross-check of the *same* 500 answers shows the combined score is
> judge-dependent and lands in a **~42–48 band**, with *stronger* judges scoring
> *stricter*: gemini-3.5-flash **47.74**, Gemma-31B **46.71**, gpt-oss-120B **41.82**
> (document recall is deterministic and identical at 55.71% across all three). So
> 47.74 is best read as the **upper end** of the interim band; the official gpt-5.4
> evaluator (stronger still) is likely to land at or below it. See
> [`archive/GEMMA_BOUNDED_INTERIM_RUNS.md`](archive/GEMMA_BOUNDED_INTERIM_RUNS.md).

<!-- verify: erb-submission/official_results.json :: aggregate_stats.combined_correctness_completeness_score == 47.74 -->
<!-- verify: erb-submission/official_results.json :: aggregate_stats.average_recall_pct == 55.71 -->
<!-- verify: erb-submission/official_results.json :: aggregate_stats.average_correctness_pct == 50.0 -->
<!-- verify: erb-submission/official_results.json :: aggregate_stats.average_completeness_pct == 53.7 -->
<!-- verify: erb-submission/official_results.json :: aggregate_stats.total_questions == 500 -->

## Retrieval quality — engine-native two-stage rerank (narrative)

Measured this cycle through the engine's **own** `two_stage_rerank` on the full
511,958-doc ERB corpus (balanced-50, same DB and lexical shortlist), scored by the
same official `metrics_based_eval`. Evidence lives under `target/` (regenerated),
so it is documented in [`ERB_ENGINE_PATH_RETRIEVAL.md`](ERB_ENGINE_PATH_RETRIEVAL.md)
rather than machine-verified here:

- Lexical shortlist, no rerank: **54.40%** recall@10.
- Engine `two_stage_rerank`, candidate pool 1024: **67.29%** (+12.9), converging
  on the float-cosine reference reranker's 68.85%.
- Raw top-1024 shortlist coverage ceiling: **91.25%** — so the path to a higher
  bar is a stronger reranker, not stage-1 coverage.

## How to reproduce

- ERB-500 official: see `erb-submission/` and the ERB make targets in
  `mk/enterprise-rag-*.mk`.
- Engine-native two-stage: `make enterprise-rag-bench-cortexdb-retrieval-50`
  variants with `--rerank two-stage --candidate-pool 1024`
  (see [`ERB_ENGINE_PATH_RETRIEVAL.md`](ERB_ENGINE_PATH_RETRIEVAL.md)).

_Run `make results-page-check` to verify every machine-checked number above._
