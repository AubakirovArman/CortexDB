# LongMemEval Official Score Path

Status: CortexDB has a full local LongMemEval v1 run using the official cleaned
data, official retrieval metric script, official generation format, and official
GPT-4o QA evaluator. Public leaderboard/list inclusion is still gated on
submission to the official maintainers.

## What Is Official

CortexDB uses the official LongMemEval v1 assets:

- official repository: `https://github.com/xiaowu0162/LongMemEval`;
- official cleaned dataset: `xiaowu0162/longmemeval-cleaned`;
- official retrieval metric script:
  `src/evaluation/print_retrieval_metrics.py`;
- official QA evaluator:
  `src/evaluation/evaluate_qa.py`.

The local CortexDB harness only builds a retrieval log from CortexDB results.
All printed retrieval metrics come from the official LongMemEval script.

## Commands

Download the official small cleaned split:

```bash
make longmemeval-v1-official-data
```

Run CortexDB retrieval over the official data and score it with the official
retrieval metric script:

```bash
make longmemeval-v1-official-retrieval-metrics
```

The command writes:

```text
target/longmemeval-v1/data/longmemeval_s_cleaned.json
target/longmemeval-v1/data/manifest.json
target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl
target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt
target/longmemeval-v1/cortexdb/report.json
target/longmemeval-v1/cortexdb/summary.md
```

Run official QA scoring after official generation has produced a hypothesis
JSONL file:

```bash
export OPENAI_API_KEY=...
make longmemeval-v1-official-qa-score \
  LONGMEMEVAL_V1_HYPOTHESIS_FILE=target/longmemeval-v1/generation/<file>.jsonl
```

## Current Official Local Evidence

Local full-run retrieval evidence on the official `longmemeval_s_cleaned.json`
split:

```text
Session-level metrics:
recall_all@5 = 0.8468
ndcg_any@5 = 0.7752
recall_all@10 = 0.9021
ndcg_any@10 = 0.7873
```

Dataset manifest:

```text
rows: 500
sha256: d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442
```

These are retrieval metrics. They are not the final QA leaderboard score.

Official QA evaluator evidence on the same split:

```text
model: gpt-4o
questions: 500
correct: 383
accuracy: 0.7660
```

Breakdown:

| Question type | Accuracy | Count |
| --- | ---: | ---: |
| `knowledge-update` | `0.8590` | `78` |
| `multi-session` | `0.6391` | `133` |
| `single-session-assistant` | `0.9464` | `56` |
| `single-session-preference` | `0.2667` | `30` |
| `single-session-user` | `0.9857` | `70` |
| `temporal-reasoning` | `0.7594` | `133` |

Generation token usage:

```text
prompt tokens: 14,213,801
completion tokens: 33,942
```

Artifacts:

```text
target/longmemeval-v1/generation/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl_testlog_top10context_jsonformat_useronlyfalse_20260602-0342
target/longmemeval-v1/generation/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl_testlog_top10context_jsonformat_useronlyfalse_20260602-0342.eval-results-gpt-4o
target/longmemeval-v1/logs/official_generation_gpt4o_20260602-034241.log
target/longmemeval-v1/logs/official_eval_gpt4o_20260602-042609.log
```

## Submission Package

Build the local evidence package:

```bash
make longmemeval-v1-package-submission
```

The package is written to:

```text
target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o/
target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o.tar.gz
```

Package contents:

```text
README.md
manifest.json
data_manifest.json
hypotheses.jsonl
eval-results-gpt-4o.jsonl
official_retrieval_metrics.txt
retrieval_report.json
```

The package intentionally excludes the full 255 MB retrieval log by default.
If the official maintainer asks for it, rebuild with the packaging script's
`--include-retrieval-log` flag.

## Submission Gap

To appear in an official LongMemEval list or leaderboard, the remaining work is:

1. Submit the local v1 package to the official maintainers, or
2. run LongMemEval-V2 and build its separate web+enterprise leaderboard package.

Until those steps are done, CortexDB has a full official local LongMemEval v1
score, but not an official published LongMemEval leaderboard entry.
