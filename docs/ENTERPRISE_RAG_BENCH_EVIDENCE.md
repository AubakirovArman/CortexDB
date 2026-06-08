# EnterpriseRAG-Bench Evidence

This page records the current local EnterpriseRAG-Bench evidence for CortexDB.
It is intentionally separated from official answer-generation scores.

## Scope

Dataset:

```text
EnterpriseRAG-Bench v1.0.0
500 questions
511,958 generated enterprise documents
```

Current evidence type:

```text
local retrieval/evidence calibration only
no LLM calls
no external API calls
top10-focused document recall
```

Current best retrieval artifact:

```text
target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v46_top10.jsonl
```

## Current Local Gate

Latest local calibration gate:

```text
target/enterprise-rag-bench/analysis/local_calibration_gate_v46.json
target/enterprise-rag-bench/analysis/local_calibration_gate_v46.md
```

Result:

| Metric | Value |
| --- | ---: |
| local gate passed | `true` |
| top10 document recall | `71.08%` |
| top10 full-recall questions | `313` |
| top10 hit questions | `351` |
| average invalid extra docs | `8.02` |
| fact token coverage proxy | `74.08%` |
| fact full coverage proxy | `83.58%` |

Gate thresholds:

| Threshold | Value |
| --- | ---: |
| min top10 recall | `70.1%` |
| max invalid extra docs | `8.1` |

Candidate generator gate:

```text
target/enterprise-rag-bench/analysis/candidate_v22_top1000_gate.json
```

| Metric | Value |
| --- | ---: |
| gate passed | `true` |
| candidate recall@500 | `90.44%` |
| candidate recall@1000 | `90.44%` |
| candidate full-recall@1000 | `415` |
| candidate hit questions@1000 | `433` |

High-level coverage gate:

```text
target/enterprise-rag-bench/analysis/high_level_coverage_v31_report.json
target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_high_level_v31_top10.jsonl
```

| Metric | Value |
| --- | ---: |
| gate passed | `true` |
| high-level questions | `10` |
| questions with docs | `10` |
| average retrieved docs | `10.0` |
| fact token coverage proxy | `73.0%` |
| fact full coverage proxy | `87.5%` |

High-level questions have no `expected_doc_ids` in the benchmark file. The
separate high-level gate therefore reports answer-fact coverage instead of
ordinary document recall or invalid-extra-doc counts.

## Progression

| Stage | Top10 Recall | Full Recall | Hit Questions | Notes |
| --- | ---: | ---: | ---: | --- |
| `multi_index_v1` candidates | `61.52%` | `269` | `310` | baseline candidate generation |
| `multi_index_v8` candidates | `65.56%` | `290` | `328` | multi-index + router + entity terms |
| `dense_hybrid_v13` | `68.94%` | `303` | `345` | local embedding cache rerank |
| `hybrid_rrf_v14` | `69.83%` | `308` | `347` | weighted RRF over candidate + dense |
| `completeness_route_v17` | `69.86%` | `309` | `347` | completeness route |
| `extra_reducer_v19` | `69.86%` | `309` | `347` | not-found/high-level abstention |
| `semantic_route_v24` | `70.07%` | `310` | `348` | semantic-only hybrid route |
| `coverage_route_v25` | `70.13%` | `310` | `348` | completeness candidate injection |
| `type_topk_v27` | `70.13%` | `310` | `348` | type-specific noise caps |
| `doc_view_v30` | `71.06%` | `313` | `351` | multi-view rerank for semantic/completeness/project-related only |
| `doc_view_v46` | `71.08%` | `313` | `351` | completeness-only coverage pass over v30 |

## What Improved

- Multi-index candidate generation raised top10 recall from `61.52%` to
  `65.56%`.
- Dense reranking from the local embedding cache raised top10 recall to
  `68.94%`.
- Weighted RRF raised top10 recall to `69.83%`.
- Completeness routing raised top10 recall to `69.86%`.
- Abstention for `info_not_found` and currently unrecovered `high_level`
  questions reduced average invalid extra docs to `8.45` without reducing
  local document recall.
- Semantic-only routing raised global top10 recall to `70.07%` and semantic
  recall from `41.6%` to `42.4%`.
- Completeness candidate injection raised global top10 recall to `70.13%` and
  completeness recall from `44.1%` to `45.55%`.
- Type-specific top-k caps kept recall at `70.13%` while reducing average
  invalid extra docs from `8.45` to `8.04`.
- Multi-view document discovery v30 raised global top10 recall to `71.06%`
  while keeping average invalid extra docs inside the local gate at `8.02`.
  The promoted route is intentionally limited to `semantic`, `completeness`,
  and `project_related` questions.
- Separate high-level coverage v31 retrieves documents for all `10`
  high-level questions and reaches `73.0%` fact token coverage without changing
  the default top10 retrieval gate.
- Completeness-only coverage v46 preserves v30 project/semantic behavior while
  raising completeness recall from `48.68%` to `49.24%` with zero regressions
  against v30.

Regression comparison against `extra_reducer_v19`:

```text
target/enterprise-rag-bench/analysis/v19_vs_v46_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v19_vs_v46_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+1.22` |
| full-recall questions | `+4` |
| hit questions | `+4` |
| improved questions | `16` |
| regressed questions | `1` |

Incremental comparison against `doc_view_v30`:

```text
target/enterprise-rag-bench/analysis/v30_vs_v46_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v30_vs_v46_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.02` |
| completeness recall | `+0.56` |
| full-recall questions | `0` |
| hit questions | `0` |
| improved questions | `1` |
| regressed questions | `0` |

Missing gold reason classifier:

```text
target/enterprise-rag-bench/analysis/gold_missing_reasons_v21_report.json
target/enterprise-rag-bench/analysis/gold_missing_reasons_v21_report.md
```

Largest current missing-gold buckets:

| Reason | Missing Gold Docs |
| --- | ---: |
| `not_in_top1000` | `66` |
| `near_duplicate_confusion` | `57` |
| `in_top500_not_top100` | `56` |
| `lost_by_embedding_rerank` | `40` |

## What Was Tested And Not Promoted

The following were measured and kept out of the default retrieval path because
they regressed local top10 recall or evidence coverage:

- path n-gram boosting as a candidate source;
- path n-gram existing-only boost;
- pure evidence digest as the only context pack;
- question-window context at a `5000` character budget;
- project-chain linked-doc reranking;
- answer-aware rerank preset.
- global hybrid v23 as default: it improved semantic slightly but regressed
  overall recall and increased invalid extra docs;
- high-level v26 as default: it improved high-level fact coverage proxy but
  raised average invalid extra docs above the current local gate.
- wide doc-view v29 route: it raised recall to `71.16%` and had zero recall
  regressions against v27, but average invalid extra docs rose to `8.38`, above
  the `8.1` local gate threshold.
- raw-candidate tail v33: it tested adding two raw candidate slots for
  semantic/completeness questions, but average recall dropped to `71.00%` and
  project-related regressions appeared.
- lower-protection doc-view v34: it tested replacing two tail slots for
  semantic/completeness/project-related questions, but average recall dropped
  to `70.88%`.
- wider semantic/completeness doc-view v35-v40: these were safe but did not
  produce enough net gain to replace the current route.
- aggressive completeness v43-v45: these raised completeness more strongly, up
  to `51.25%`, but reduced full-recall questions from `313` to `312`.

## Reproduction Commands

Build candidate doc views:

```bash
python scripts/enterprise_rag_bench/build_doc_view_subset.py \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_multi_index_v22_candidates_top1000.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --candidate-limit 50 \
  --output target/enterprise-rag-bench/index/doc_views_candidates_v28_top50.jsonl \
  --report target/enterprise-rag-bench/index/doc_views_candidates_v28_top50_report.json
```

Run targeted doc-view rerank:

```bash
python scripts/enterprise_rag_bench/doc_view_rerank.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_multi_index_v22_candidates_top1000.jsonl \
  --baseline-retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --doc-views-file target/enterprise-rag-bench/index/doc_views_candidates_v28_top50.jsonl \
  --embedding-cache target/enterprise-rag-bench/retrieval/embedding_cache.jsonl \
  --output target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v30_top10.jsonl \
  --report target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v30_top10_report.json \
  --score-candidate-limit 50 \
  --limit 10 \
  --seed-count 3 \
  --protect-baseline-prefix 9 \
  --route-question-types semantic,completeness,project_related
```

Run completeness-only coverage pass over v30:

```bash
make enterprise-rag-bench-completeness-coverage
```

Depth audit:

```bash
python scripts/enterprise_rag_bench/candidate_depth_audit.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v46_top10.jsonl \
  --output-jsonl target/enterprise-rag-bench/analysis/doc_view_v46_depth_details.jsonl \
  --report target/enterprise-rag-bench/analysis/doc_view_v46_depth_report.json \
  --markdown target/enterprise-rag-bench/analysis/doc_view_v46_depth_report.md
```

Evidence pack proxy:

```bash
python scripts/enterprise_rag_bench/evaluate_evidence_pack.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v46_top10.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --mode leading \
  --top-k 10 \
  --max-chars-per-doc 5000 \
  --output-jsonl target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v46_leading_details.jsonl \
  --report target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v46_leading_report.json
```

Calibration gate:

```bash
python scripts/enterprise_rag_bench/summarize_local_calibration.py \
  --depth-report target/enterprise-rag-bench/analysis/doc_view_v46_depth_report.json \
  --evidence-report target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v46_leading_report.json \
  --output target/enterprise-rag-bench/analysis/local_calibration_gate_v46.json \
  --markdown target/enterprise-rag-bench/analysis/local_calibration_gate_v46.md \
  --min-top10-recall-pct 70.1 \
  --max-invalid-extra-docs 8.1
```

High-level coverage gate:

```bash
make enterprise-rag-bench-high-level-coverage
```

## Limitations

- These numbers are not the official EnterpriseRAG answer score.
- Correctness and completeness still require an answer generation run plus the
  official evaluator/judge path.
- The local evidence proxy uses benchmark gold facts to measure coverage. It is
  an analysis tool, not a production scoring signal.
- `high_level` questions are abstained in the default local retrieval gate
  because they have no `expected_doc_ids`; use
  `enterprise-rag-bench-high-level-coverage` for their separate fact-coverage
  evidence.
