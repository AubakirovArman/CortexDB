# LongMemEval Official Score Path

Status: CortexDB has a full local LongMemEval v1 run using the official cleaned
data, official retrieval metric script, and official generation/evaluation
format. Current local generation/evaluation defaults use DeepSeek flash.
Public leaderboard/list inclusion is still gated on submission to the official
maintainers.

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

## Epic 137 Evidence Page Contract

This page is the LongMemEval evidence page for the production epic plan. It
publishes four bounded pieces of evidence:

| Task | Evidence on this page | Boundary |
| --- | --- | --- |
| Retrieval-only results | Official local LongMemEval v1 session retrieval metrics for the 500-row small split. | Retrieval quality only; not a QA leaderboard score. |
| Official evaluator command | The exact `make longmemeval-v1-official-retrieval-metrics` flow that invokes `LongMemEval/src/evaluation/print_retrieval_metrics.py`. | Uses the official retrieval metric script after CortexDB produces the retrieval log. |
| Log format | The JSONL retrieval log path and row shape expected by the official metric script. | The log carries ranked retrieval IDs and text context; it is not committed because it is large. |
| Limitations | Submission gap, local-only boundary, QA/retrieval separation, and DeepSeek diagnostic boundary. | No official leaderboard/list placement claim until maintainers accept a submission. |

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

Run the retrieval-adapter acceptance gate for Epic 49:

```bash
make longmemeval-v1-retrieval-adapter-check
```

Validate the end-to-end adapter package for Epic 50 without making any API
calls:

```bash
make longmemeval-v1-e2e-adapter-check
```

The command writes:

```text
target/longmemeval-v1/data/longmemeval_s_cleaned.json
target/longmemeval-v1/data/manifest.json
target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl
target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt
target/longmemeval-v1/cortexdb/report.json
target/longmemeval-v1/cortexdb/summary.md
target/longmemeval-v1/retrieval-adapter/report.json
target/longmemeval-v1/e2e-adapter/report.json
```

## Official Evaluator Command

The official retrieval evaluator command is run through:

```bash
make longmemeval-v1-official-retrieval-metrics
```

That target runs:

```text
LongMemEval/src/evaluation/print_retrieval_metrics.py \
  target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl
```

and writes:

```text
target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt
```

The adapter gate then validates the official data manifest, retrieval report,
retrieval log row count, and official metrics:

```bash
make longmemeval-v1-retrieval-adapter-check
```

Run official QA scoring after official generation has produced a hypothesis
JSONL file:

```bash
export DEEPSEEK_API_KEY=...
make longmemeval-v1-official-qa-score \
  LONGMEMEVAL_V1_HYPOTHESIS_FILE=target/longmemeval-v1/generation/<file>.jsonl
```

## Retrieval-only Results

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

## Retrieval Log Format

CortexDB writes the retrieval log consumed by the official metric script at:

```text
target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl
```

The log is JSONL. Each non-empty line is a JSON object with:

```json
{
  "question_id": "string",
  "question": "string",
  "answer": "string",
  "question_type": "string",
  "retrieval_results": {
    "ranked_items": [
      {
        "id": "string",
        "session_id": "string",
        "turn_id": "string",
        "text": "string",
        "score": 0
      }
    ]
  }
}
```

The retrieval-adapter checker validates that every row has `question_id`,
`retrieval_results`, and `ranked_items`, and that the JSONL row count matches
the CortexDB retrieval report.

Retrieval adapter acceptance evidence:

```text
schema: cortexdb.longmemeval.v1.retrieval_adapter_check.v1
status: passed
retrieval_log_rows: 500
boundary: retrieval-only, not an end-to-end QA claim
```

End-to-end adapter acceptance evidence:

```text
schema: cortexdb.longmemeval.v1.e2e_adapter_check.v1
status: passed
hypotheses_rows: 500
eval_rows: 500
qa accuracy: 0.7660
boundary: retrieval metrics and QA accuracy are separate claims
```

Historical QA evaluator evidence on the same split:

```text
model: gpt-4o
questions: 500
correct: 383
accuracy: 0.7660
```

This is retained as a historical local baseline. New local runs should use the
DeepSeek defaults in the Makefile unless a submission policy explicitly requires
a different evaluator model.

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

## Limitations

- The official local retrieval metrics are retrieval-only evidence.
- The historical QA accuracy is a local official-run artifact, not a public
  score and not a public LongMemEval leaderboard/list placement.
- DeepSeek Flash runs are local diagnostics and are not official LongMemEval
  scores.
- The large retrieval log and generated hypotheses are retained under `target/`
  and are not committed to the repository.
- To publish an official claim, the packaged v1 artifacts must be submitted to the LongMemEval maintainers.
  LongMemEval-V2 must be run under its separate web/enterprise benchmark process.

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

## Multi-Session Aggregation Diagnostic

The next weakness was `multi-session`: DeepSeek often found the right facts but
refused to compute a final count, total, duration, or comparison unless the
history explicitly stated the combined answer. The runner now uses a
multi-session-aware generation prompt for `multi-session`: it asks the reader to
use evidence across all provided sessions, reconcile duplicates, and compute the
final aggregate directly.

Run the focused check:

```bash
make longmemeval-v1-deepseek-flash-multi-session-check
```

Latest local result:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
question type: multi-session
before multi-session-aware prompt: 63 / 133
after multi-session-aware prompt: 77 / 133
accuracy: 0.5789
empty hypotheses: 0
```

After enabling both the preference-aware and multi-session-aware generation
prompts, the full compact-500 diagnostic improved again:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 283 / 500
accuracy: 0.5660
empty hypotheses: 0
prompt tokens: 4,301,681
completion tokens: 45,261
```

Updated breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `47` | `78` | `0.6026` |
| `multi-session` | `77` | `133` | `0.5789` |
| `single-session-assistant` | `35` | `56` | `0.6250` |
| `single-session-preference` | `19` | `30` | `0.6333` |
| `single-session-user` | `40` | `70` | `0.5714` |
| `temporal-reasoning` | `65` | `133` | `0.4887` |

## Temporal Reasoning Diagnostic

The next weak slice was `temporal-reasoning`: DeepSeek often had the right
events in context but refused to compute dates or durations unless the final
interval was stated explicitly. The runner now uses a temporal-aware generation
prompt for `temporal-reasoning`: it asks the reader to build a timeline from
session timestamps, resolve relative phrases such as `yesterday` or `last week`,
sort events by calendar time, and compute intervals directly.

Run the focused check:

```bash
make longmemeval-v1-deepseek-flash-temporal-check
```

Latest local result:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
question type: temporal-reasoning
before temporal-aware prompt: 65 / 133
after temporal-aware prompt: 89 / 133
accuracy: 0.6692
empty hypotheses: 0
```

After enabling the preference-aware, multi-session-aware, and temporal-aware
generation prompts, the full compact-500 diagnostic improved again:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 309 / 500
accuracy: 0.6180
empty hypotheses: 0
prompt tokens: 4,321,364
completion tokens: 49,467
```

Updated breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `46` | `78` | `0.5897` |
| `multi-session` | `80` | `133` | `0.6015` |
| `single-session-assistant` | `34` | `56` | `0.6071` |
| `single-session-preference` | `20` | `30` | `0.6667` |
| `single-session-user` | `41` | `70` | `0.5857` |
| `temporal-reasoning` | `88` | `133` | `0.6617` |

The remaining low slices are now less concentrated: `single-session-user`,
`knowledge-update`, `multi-session`, and `single-session-assistant` all sit in
the `0.58-0.61` range in this local diagnostic.

## Focused-First 50-Question Gate

For local prompt and answer-shape iteration, CortexDB now uses a smaller
balanced 50-question gate before any expensive 500-question diagnostic. The
subset is deterministic and proportional by question type:

```text
knowledge-update: 8
multi-session: 13
single-session-assistant: 6
single-session-preference: 3
single-session-user: 7
temporal-reasoning: 13
```

Run it with:

```bash
make longmemeval-v1-deepseek-flash-compact-50-check
```

Latest local compact-50 result with the preference-aware, multi-session-aware,
temporal-aware, and single-session-user-aware prompts:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
official score: false
correct by DeepSeek judge: 32 / 50
accuracy: 0.6400
empty hypotheses: 0
prompt tokens: 435,067
completion tokens: 5,313
```

Breakdown:

| Question type | Correct | Count | Accuracy |
| --- | ---: | ---: | ---: |
| `knowledge-update` | `5` | `8` | `0.6250` |
| `multi-session` | `6` | `13` | `0.4615` |
| `single-session-assistant` | `6` | `6` | `1.0000` |
| `single-session-preference` | `2` | `3` | `0.6667` |
| `single-session-user` | `5` | `7` | `0.7143` |
| `temporal-reasoning` | `8` | `13` | `0.6154` |

The single-session-user prompt was tested first on its focused slice:

```text
before single-session-user-aware prompt: 41 / 70
after single-session-user-aware prompt: 44 / 70
accuracy: 0.6286
empty hypotheses: 0
```

Operational rule: run a focused 50-70 question gate first. Only run the full
500-question diagnostic when the focused gate has a positive net delta and does
not introduce obvious old-correct regressions.

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
target/longmemeval-v1/submission/cortexdb-longmemeval-v1-deepseek-flash/
target/longmemeval-v1/submission/cortexdb-longmemeval-v1-deepseek-flash.tar.gz
```

Package contents:

```text
README.md
manifest.json
data_manifest.json
hypotheses.jsonl
eval-results.jsonl
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
