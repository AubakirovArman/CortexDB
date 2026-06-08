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
target/enterprise-rag-bench/retrieval/cortexdb_full_extra_reducer_v19_top10.jsonl
```

## Current Local Gate

Latest local calibration gate:

```text
target/enterprise-rag-bench/analysis/local_calibration_gate_v19.json
target/enterprise-rag-bench/analysis/local_calibration_gate_v19.md
```

Result:

| Metric | Value |
| --- | ---: |
| local gate passed | `true` |
| top10 document recall | `69.86%` |
| top10 full-recall questions | `309` |
| top10 hit questions | `347` |
| average invalid extra docs | `8.45` |
| fact token coverage proxy | `74.09%` |
| fact full coverage proxy | `83.61%` |

Gate thresholds:

| Threshold | Value |
| --- | ---: |
| min top10 recall | `69.8%` |
| max invalid extra docs | `8.5` |

## Progression

| Stage | Top10 Recall | Full Recall | Hit Questions | Notes |
| --- | ---: | ---: | ---: | --- |
| `multi_index_v1` candidates | `61.52%` | `269` | `310` | baseline candidate generation |
| `multi_index_v8` candidates | `65.56%` | `290` | `328` | multi-index + router + entity terms |
| `dense_hybrid_v13` | `68.94%` | `303` | `345` | local embedding cache rerank |
| `hybrid_rrf_v14` | `69.83%` | `308` | `347` | weighted RRF over candidate + dense |
| `completeness_route_v17` | `69.86%` | `309` | `347` | completeness route |
| `extra_reducer_v19` | `69.86%` | `309` | `347` | not-found/high-level abstention |

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

## What Was Tested And Not Promoted

The following were measured and kept out of the default retrieval path because
they regressed local top10 recall or evidence coverage:

- path n-gram boosting as a candidate source;
- path n-gram existing-only boost;
- pure evidence digest as the only context pack;
- question-window context at a `5000` character budget;
- project-chain linked-doc reranking;
- answer-aware rerank preset.

## Reproduction Commands

Depth audit:

```bash
python scripts/enterprise_rag_bench/candidate_depth_audit.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_extra_reducer_v19_top10.jsonl \
  --output-jsonl target/enterprise-rag-bench/analysis/extra_reducer_v19_top10_depth_details.jsonl \
  --report target/enterprise-rag-bench/analysis/extra_reducer_v19_top10_depth_report.json \
  --markdown target/enterprise-rag-bench/analysis/extra_reducer_v19_top10_depth_report.md
```

Evidence pack proxy:

```bash
python scripts/enterprise_rag_bench/evaluate_evidence_pack.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_extra_reducer_v19_top10.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --mode leading \
  --top-k 10 \
  --max-chars-per-doc 5000 \
  --output-jsonl target/enterprise-rag-bench/analysis/evidence_pack_extra_reducer_v19_leading_details.jsonl \
  --report target/enterprise-rag-bench/analysis/evidence_pack_extra_reducer_v19_leading_report.json
```

Calibration gate:

```bash
python scripts/enterprise_rag_bench/summarize_local_calibration.py \
  --depth-report target/enterprise-rag-bench/analysis/extra_reducer_v19_top10_depth_report.json \
  --evidence-report target/enterprise-rag-bench/analysis/evidence_pack_extra_reducer_v19_leading_report.json \
  --output target/enterprise-rag-bench/analysis/local_calibration_gate_v19.json \
  --markdown target/enterprise-rag-bench/analysis/local_calibration_gate_v19.md \
  --min-top10-recall-pct 69.8 \
  --max-invalid-extra-docs 8.5
```

## Limitations

- These numbers are not the official EnterpriseRAG answer score.
- Correctness and completeness still require an answer generation run plus the
  official evaluator/judge path.
- The local evidence proxy uses benchmark gold facts to measure coverage. It is
  an analysis tool, not a production scoring signal.
- `high_level` questions are currently abstained in the local extra-doc reducer
  because the current top10 pipeline has `0%` recall for that type.
