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

## Unofficial DeepSeek Flash Compact-500 Diagnostic

The false-case diagnostic above is intentionally hard because it only uses
questions that the previous GPT-4o baseline got wrong. For broader iteration,
CortexDB also keeps a 500-question diagnostic over the full
`longmemeval_s_cleaned.json` split and the compact CortexDB retrieval log:

```bash
make longmemeval-v1-deepseek-flash-compact-500-check
```

First local result before preference-aware prompt:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 252 / 500
accuracy: 0.5040
empty hypotheses: 0
prompt tokens: 4,274,340
completion tokens: 31,512
```

Breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `47` | `78` | `0.6026` |
| `multi-session` | `62` | `133` | `0.4662` |
| `single-session-assistant` | `35` | `56` | `0.6250` |
| `single-session-preference` | `0` | `30` | `0.0000` |
| `single-session-user` | `40` | `70` | `0.5714` |
| `temporal-reasoning` | `68` | `133` | `0.5113` |

This first compact-500 pass exposed a prompt/context-shaping issue:
`single-session-preference` was `0 / 30`.

## Preference-Format Diagnostic

The first compact-500 DeepSeek pass used the same factual-answer prompt for all
question types. That format is wrong for `single-session-preference`: the model
often answered with refusal/insufficient-history language even when retrieved
context contained usable preference signals.

The runner now uses a preference-aware generation prompt for
`single-session-preference`: it asks the reader to infer user preferences from
history and provide concrete personalized recommendations instead of looking for
an exact pre-existing answer.

Run the focused check:

```bash
make longmemeval-v1-deepseek-flash-preference-check
```

Latest local result:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
question type: single-session-preference
before preference-aware prompt: 0 / 30
after preference-aware prompt: 20 / 30
accuracy: 0.6667
empty hypotheses: 0
```

This confirms that the previous `single-session-preference` failure was mostly
an answer-shape/prompt mismatch, not simply missing retrieval evidence.

After enabling the preference-aware generation prompt, the full compact-500
diagnostic improved:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 269 / 500
accuracy: 0.5380
empty hypotheses: 0
prompt tokens: 4,289,467
completion tokens: 44,629
```

Updated breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `45` | `78` | `0.5769` |
| `multi-session` | `63` | `133` | `0.4737` |
| `single-session-assistant` | `35` | `56` | `0.6250` |
| `single-session-preference` | `18` | `30` | `0.6000` |
| `single-session-user` | `40` | `70` | `0.5714` |
| `temporal-reasoning` | `68` | `133` | `0.5113` |

The next improvement target moves from format correction to the lowest remaining
general slice, especially `multi-session` retrieval/answer synthesis.

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
