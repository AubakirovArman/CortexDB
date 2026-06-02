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

## Error Analysis

After the official local QA run is available, generate the post-hoc error
analysis report:

```bash
make longmemeval-v1-error-analysis
```

The command writes:

```text
target/longmemeval-v1/analysis/error_report.json
target/longmemeval-v1/analysis/error_report.md
target/longmemeval-v1/analysis/false_cases.jsonl
target/longmemeval-v1/analysis/retrieval_diagnostics.jsonl
```

Latest local error analysis on the official v1 run:

| Category | False cases |
| --- | ---: |
| `multi_session_reader_failure` | `27` |
| `retrieval_partial_miss_top10` | `27` |
| `retrieval_miss_no_answer_session_top10` | `17` |
| `preference_reader_failure` | `16` |
| `temporal_reasoning_failure` | `16` |
| `abstention_failure` | `7` |
| `knowledge_update_reader_failure` | `7` |

This report is diagnostic only. It uses official labels after evaluation to
prioritize future changes; it must not be used inside runtime retrieval or
generation.

## Reader Context Variants

The default v1 retrieval log keeps the historical behavior:

```text
index mode = user
context mode = user
```

This indexes only user turns and also sends only user-turn text to the official
reader. For reader-failure analysis, CortexDB also supports a compact
conversation context while keeping the same user-only index:

```bash
make longmemeval-v1-official-retrieval-metrics \
  LONGMEMEVAL_V1_OUTPUT_ROOT=target/longmemeval-v1/cortexdb-compact-context \
  LONGMEMEVAL_V1_CONTEXT_MODE=compact \
  LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT=target/longmemeval-v1/cortexdb-compact-context/official_retrieval_metrics.txt
```

Latest compact-context retrieval-only evidence:

```text
index mode: user
context mode: compact
max turn chars: 900
max session chars: 4000
retrieval log size: 266M
avg top10 context chars: 38,435
official session recall_all@10: 0.9021
official session ndcg_any@10: 0.7873
```

The compact context changes only the text passed to the reader; it does not use
gold labels and does not change the ranking IDs. The next score-improvement
experiment is to run official GPT-4o generation/evaluation against this
compact-context log and compare QA accuracy with the `0.7660` user-context
baseline.

## Unofficial DeepSeek Flash Diagnostic

When GPT-4o quota is unavailable, CortexDB can run a local diagnostic pass with
an OpenAI-compatible DeepSeek endpoint. The default target uses explicit
non-thinking mode for both generation and judging, because DeepSeek thinking mode
can spend the output budget on `reasoning_content` and return empty visible
`content`:

```bash
make longmemeval-v1-deepseek-flash-falsecase-check
```

Default output root:

```text
target/longmemeval-v1/targeted-deepseek-flash-thinking-disabled/
```

Latest local diagnostic on the `117` baseline GPT-4o false cases with explicit
thinking disabled:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 24 / 117
accuracy: 0.2051
empty hypotheses: 0
prompt tokens: 1,009,393
completion tokens: 8,955
```

Breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `5` | `11` | `0.4545` |
| `multi-session` | `11` | `48` | `0.2292` |
| `single-session-assistant` | `0` | `3` | `0.0000` |
| `single-session-preference` | `0` | `22` | `0.0000` |
| `single-session-user` | `1` | `1` | `1.0000` |
| `temporal-reasoning` | `7` | `32` | `0.2188` |

Previous implicit-thinking run for comparison:

```text
model: deepseek-v4-flash
correct by DeepSeek judge: 27 / 117
accuracy: 0.2308
empty hypotheses: 4
completion tokens: 48,555
```

Flash-only diff between the previous implicit-thinking run and the
thinking-disabled run:

```bash
make longmemeval-v1-deepseek-flash-diff
```

```text
both correct: 14
both wrong: 80
new-only correct: 10
old-only correct: 13
old empty hypotheses: 4
new empty hypotheses: 0
old completion tokens: 48,555
new completion tokens: 8,955
```

The thinking-disabled run is cleaner operationally because it produces no empty
hypotheses and uses far fewer completion tokens. It also improves several
knowledge-update cases, but it regresses more temporal-reasoning cases:

| Type | Count | Both correct | Both wrong | New only | Old only |
| --- | ---: | ---: | ---: | ---: | ---: |
| `knowledge-update` | `11` | `2` | `6` | `3` | `0` |
| `multi-session` | `48` | `8` | `32` | `3` | `5` |
| `single-session-assistant` | `3` | `0` | `3` | `0` | `0` |
| `single-session-preference` | `22` | `0` | `22` | `0` | `0` |
| `single-session-user` | `1` | `0` | `0` | `1` | `0` |
| `temporal-reasoning` | `32` | `4` | `17` | `3` | `8` |

This is useful for iteration, but it is not an official LongMemEval result:
generation and judging both use DeepSeek flash instead of the official GPT-4o
judge.

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
