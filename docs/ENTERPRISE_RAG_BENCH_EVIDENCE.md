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
target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl
```

## Current Local Gate

Latest local calibration gate:

```text
target/enterprise-rag-bench/analysis/local_calibration_gate_v27.json
target/enterprise-rag-bench/analysis/local_calibration_gate_v27.md
```

Result:

| Metric | Value |
| --- | ---: |
| local gate passed | `true` |
| top10 document recall | `70.13%` |
| top10 full-recall questions | `310` |
| top10 hit questions | `348` |
| average invalid extra docs | `8.04` |
| fact token coverage proxy | `73.99%` |
| fact full coverage proxy | `83.26%` |

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

Regression comparison against `extra_reducer_v19`:

```text
target/enterprise-rag-bench/analysis/v19_vs_v27_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v19_vs_v27_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.27` |
| full-recall questions | `+1` |
| hit questions | `+1` |
| improved questions | `5` |
| regressed questions | `2` |

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

## Reproduction Commands

Depth audit:

```bash
python scripts/enterprise_rag_bench/candidate_depth_audit.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --output-jsonl target/enterprise-rag-bench/analysis/type_topk_v27_depth_details.jsonl \
  --report target/enterprise-rag-bench/analysis/type_topk_v27_depth_report.json \
  --markdown target/enterprise-rag-bench/analysis/type_topk_v27_depth_report.md
```

Evidence pack proxy:

```bash
python scripts/enterprise_rag_bench/evaluate_evidence_pack.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --mode leading \
  --top-k 10 \
  --max-chars-per-doc 5000 \
  --output-jsonl target/enterprise-rag-bench/analysis/evidence_pack_type_topk_v27_leading_details.jsonl \
  --report target/enterprise-rag-bench/analysis/evidence_pack_type_topk_v27_leading_report.json
```

Calibration gate:

```bash
python scripts/enterprise_rag_bench/summarize_local_calibration.py \
  --depth-report target/enterprise-rag-bench/analysis/type_topk_v27_depth_report.json \
  --evidence-report target/enterprise-rag-bench/analysis/evidence_pack_type_topk_v27_leading_report.json \
  --output target/enterprise-rag-bench/analysis/local_calibration_gate_v27.json \
  --markdown target/enterprise-rag-bench/analysis/local_calibration_gate_v27.md \
  --min-top10-recall-pct 70.1 \
  --max-invalid-extra-docs 8.1
```

## Limitations

- These numbers are not the official EnterpriseRAG answer score.
- Correctness and completeness still require an answer generation run plus the
  official evaluator/judge path.
- The local evidence proxy uses benchmark gold facts to measure coverage. It is
  an analysis tool, not a production scoring signal.
- `high_level` questions are currently abstained in the local extra-doc reducer
  because the current top10 pipeline has `0%` recall for that type.
