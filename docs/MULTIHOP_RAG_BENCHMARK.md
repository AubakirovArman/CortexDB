# MultiHop-RAG Benchmark Plan

Status: reproducible local retrieval run completed, not a published leaderboard
result.

MultiHop-RAG is the next benchmark layer after LongMemEval for CortexDB. It
tests whether retrieval can gather evidence from multiple documents instead of
returning one similar chunk. The official dataset contains 2556 queries and
evidence distributed across 2 to 4 documents, with inference, comparison,
temporal, and null-query cases.

Official references:

- GitHub: <https://github.com/yixuantt/MultiHop-RAG>
- Hugging Face dataset: <https://huggingface.co/datasets/yixuantt/MultiHopRAG>
- OpenReview / COLM 2024: <https://openreview.net/forum?id=t4eB3zYWBK>

The official repository currently has a leaderboard section marked "Coming
soon", so CortexDB should publish reproducible local artifacts first and only
claim leaderboard inclusion after maintainers provide a submission path.

## Why This Matters

LongMemEval checks long-term memory over sessions. MultiHop-RAG checks a
different risk: whether the retrieval layer can collect several supporting
facts across documents before the answer is generated.

The useful CortexDB signal is:

```text
AQL / search / ANN retrieval
-> top-k multi-document evidence
-> official retrieval metrics
-> optional QA generation
-> official QA metrics
```

## Local Gate First

Do not start with the full 2556-query run when tuning. Use the local 50-query
gate first:

```bash
make multihop-rag-local-50-check
```

That target downloads the official JSON files, validates them, and creates a
balanced 50-query subset under:

```text
target/multihop-rag/subsets/balanced_50/
```

The generated files are:

```text
balanced_50_multihop.json
balanced_50_queries.jsonl
balanced_50_ground_truth.jsonl
balanced_50_subset_report.json
```

This is the same workflow rule as LongMemEval tuning: improve on 50 cases first,
then promote to the full benchmark only when the focused gate improves.

To run CortexDB retrieval on the 50-query subset and score it with the official
retrieval evaluator:

```bash
make multihop-rag-official-retrieval-metrics-50
```

That command builds the local `multihop_rag_retrieval` runner, loads the
official corpus into a temporary CortexDB database, checkpoints the corpus so
the persisted lexical index is used, runs keyword retrieval for the 50 selected
questions, and writes:

```text
target/multihop-rag/retrieval/cortexdb_balanced_50_retrieval.json
target/multihop-rag/retrieval/cortexdb_balanced_50_report.json
target/multihop-rag/retrieval/cortexdb_balanced_50_metrics.txt
```

To generate answers with DeepSeek Flash and score them with the official
`qa_evaluate.py`:

```bash
make multihop-rag-official-qa-metrics-50
```

To run the same 50-query gate and record DeepSeek prompt-cache metrics:

```bash
make multihop-rag-deepseek-qa-50-cache-metrics
```

To tune temporal questions without a full QA run, use the temporal-only gate.
It uses the existing full retrieval artifact, filters the first 50
`temporal_query` rows, and scores them with the official QA script:

```bash
make multihop-rag-official-qa-metrics-temporal-50-v3
make multihop-rag-qa-error-analysis-temporal-50-v3
```

If answers already exist and only the official scorer must be rerun, use:

```bash
make multihop-rag-official-qa-metrics-existing-50
make multihop-rag-qa-error-analysis-50
```

The error analysis summarizes misses by `question_type`, exact-match rate, and
false abstentions. This is the fast iteration loop for prompt or context
selection changes.

## Official Full-Run Path

After the 50-query gate is stable, run the full official data preparation:

```bash
make multihop-rag-preflight
```

Then build and evaluate a CortexDB retrieval output compatible with the official
evaluator:

```bash
make multihop-rag-official-retrieval-metrics-full
```

The full run writes:

```text
target/multihop-rag/retrieval/cortexdb_full_retrieval.json
target/multihop-rag/retrieval/cortexdb_full_report.json
target/multihop-rag/retrieval/cortexdb_full_metrics.txt
```

Full QA generation and scoring:

```bash
make multihop-rag-official-qa-metrics-full
```

That command uses `deepseek-v4-flash` with thinking disabled and the
`multihop-v2` question-type-aware prompt by default, writes answers to
`target/multihop-rag/qa/deepseek-full-v2/deepseek_qa.json`, then evaluates that
file with the official `qa_evaluate.py` script.

To rerun only the scorer and local error analysis against existing full QA
answers:

```bash
make multihop-rag-official-qa-metrics-existing-full
make multihop-rag-qa-error-analysis-full
```

The current best local QA route uses `multihop-v2` for non-temporal questions
and a temporal-only `multihop-v3` prompt for `temporal_query`:

```bash
make multihop-rag-official-qa-metrics-hybrid-full
```

The official-compatible retrieval output shape is:

```json
[
  {
    "query": "...",
    "answer": "...",
    "question_type": "inference_query",
    "evidence_list": [{"fact": "..."}],
    "retrieval_list": [{"text": "..."}]
  }
]
```

The official retrieval evaluator reports:

```text
Hits@10
Hits@4
MAP@10
MRR@10
```

QA generation can then be scored with the official `qa_evaluate.py`, which
reports Exact Match and F1.

## Required Artifact Set

Every public CortexDB MultiHop-RAG run must archive:

- official dataset source and manifest;
- CortexDB retrieval config;
- embedding model and endpoint family;
- top-k and chunking settings;
- raw retrieval output JSON;
- official retrieval evaluator output;
- generation model and prompt, if QA is run;
- raw QA output;
- official QA evaluator output;
- a short report with non-claims.

## Non-Claims

Until a full official run is completed and archived, CortexDB must not claim:

- MultiHop-RAG leaderboard placement;
- production multi-hop QA quality;
- superiority over other RAG systems;
- official maintainer endorsement.

## Latest Local Retrieval Evidence

Latest local CortexDB keyword retrieval run using the official MultiHop-RAG
retrieval evaluator:

| Run | Questions | Hits@10 | Hits@4 | MAP@10 | MRR@10 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced local gate | 50 | 1.0000 | 0.9545 | 0.4396 | 0.7760 |
| Full official dataset | 2556 | 0.9902 | 0.9295 | 0.4503 | 0.7906 |

Latest local QA run using CortexDB retrieval, DeepSeek Flash generation, and
the official `qa_evaluate.py` script:

| Run | Questions | Overall Precision | Overall Recall | Overall F1 | Overall Accuracy |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced local gate, `multihop-v2` prompt | 50 | 0.68 | 0.68 | 0.68 | 0.61 |
| Temporal-only gate, `multihop-v3` prompt | 50 | 0.62 | 0.62 | 0.62 | 0.57 |
| Full official dataset, hybrid `multihop-v2` + temporal `multihop-v3` | 2556 | 0.75 | 0.75 | 0.75 | 0.67 |

Full QA by question type:

| Type | Precision | Recall | F1 | Accuracy |
| --- | ---: | ---: | ---: | ---: |
| `inference_query` | 0.94 | 0.94 | 0.94 | 0.90 |
| `comparison_query` | 0.65 | 0.65 | 0.65 | 0.59 |
| `null_query` | 0.99 | 0.99 | 0.99 | 0.99 |
| `temporal_query` | 0.51 | 0.51 | 0.51 | 0.50 |

Latest DeepSeek prompt-cache observation on the repeat 50-query gate:

| Run | Prompt tokens | Cache hit tokens | Cache miss tokens | Cache hit rate | Wall time |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced 50, `multihop-v2` repeat | 71,513 | 68,608 | 2,905 | 95.94% | 14.811s |

Using DeepSeek Flash pricing from the official pricing page, this repeat gate is
estimated at about `$0.00063` with cache versus about `$0.01004` if all input
tokens were charged as cache misses. The estimate is informational; actual
billing depends on the provider account and current pricing.

Artifacts:

```text
target/multihop-rag/retrieval/cortexdb_balanced_50_retrieval.json
target/multihop-rag/retrieval/cortexdb_balanced_50_metrics.txt
target/multihop-rag/retrieval/cortexdb_full_retrieval.json
target/multihop-rag/retrieval/cortexdb_full_metrics.txt
target/multihop-rag/qa/deepseek-balanced-50-v2/deepseek_qa.json
target/multihop-rag/qa/deepseek-balanced-50-v2/official_qa_metrics.txt
target/multihop-rag/qa/deepseek-balanced-50-v2/qa_error_analysis.json
target/multihop-rag/qa/deepseek-full-v2/deepseek_qa.json
target/multihop-rag/qa/deepseek-full-v2/official_qa_metrics.txt
target/multihop-rag/qa/deepseek-full-v2/qa_error_analysis.json
target/multihop-rag/qa/deepseek-temporal-v3/deepseek_qa.json
target/multihop-rag/qa/deepseek-temporal-50-v3/deepseek_qa.json
target/multihop-rag/qa/deepseek-full-v3-hybrid/deepseek_qa.json
target/multihop-rag/qa/deepseek-full-v3-hybrid/official_qa_metrics.txt
target/multihop-rag/qa/deepseek-balanced-50-cache-metrics/deepseek_qa_report.json
```

Current status:

```text
MultiHop-RAG scaffold exists.
Local 50-query retrieval gate runs and scores with the official evaluator.
Full 2556-query retrieval run completes and scores with the official evaluator.
DeepSeek Flash QA generation completes and scores with the official evaluator.
Temporal QA remains the main quality gap, but temporal-only prompt routing now
improves the full-run temporal F1 from 0.44 to 0.51.
```
