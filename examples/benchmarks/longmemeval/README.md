# LongMemEval Official Benchmark

This directory documents the official LongMemEval path for CortexDB.

## What Counts As Official

For LongMemEval v1, CortexDB must use:

- the official repository: `https://github.com/xiaowu0162/LongMemEval`;
- the official cleaned dataset: `xiaowu0162/longmemeval-cleaned`;
- the official QA evaluator: `src/evaluation/evaluate_qa.py gpt-4o`.

Local retrieval-only reports are useful release evidence, but the final QA
score is not official unless the generated hypotheses are evaluated by the
official evaluator.

For leaderboard submission, prefer LongMemEval-V2. Its official repository
contains `leaderboard/` packaging utilities and a submission form.

## Local Commands

Download the official v1 data:

```bash
make longmemeval-v1-official-data
```

Run CortexDB retrieval on the official v1 data:

```bash
make longmemeval-v1-cortexdb-retrieval
```

Print official retrieval metrics from the generated retrieval log:

```bash
make longmemeval-v1-official-retrieval-metrics
```

Generate QA hypotheses through the official generation script:

```bash
export LONGMEMEVAL_V1_READER_OPENAI_KEY="$OPENAI_API_KEY"
make longmemeval-v1-official-generate
```

Evaluate QA hypotheses through the official evaluator:

```bash
export OPENAI_API_KEY=...
make longmemeval-v1-official-qa-score \
  LONGMEMEVAL_V1_HYPOTHESIS_FILE=target/longmemeval-v1/generation/<file>
```

Analyze false cases after the official evaluator finishes:

```bash
make longmemeval-v1-error-analysis
```

This writes `target/longmemeval-v1/analysis/error_report.md`,
`error_report.json`, `false_cases.jsonl`, and `retrieval_diagnostics.jsonl`.
The analysis is post-hoc only and is meant to guide improvements without using
gold labels inside runtime retrieval.

Try the compact reader-context variant without changing retrieval ranking:

```bash
make longmemeval-v1-official-retrieval-metrics \
  LONGMEMEVAL_V1_OUTPUT_ROOT=target/longmemeval-v1/cortexdb-compact-context \
  LONGMEMEVAL_V1_CONTEXT_MODE=compact \
  LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT=target/longmemeval-v1/cortexdb-compact-context/official_retrieval_metrics.txt
```

This keeps `index-mode=user` and changes only the text passed to the reader.
Run official generation/evaluation on that retrieval log before treating it as
a score improvement.

Run the unofficial DeepSeek flash false-case diagnostic:

```bash
make longmemeval-v1-deepseek-flash-falsecase-check
```

This uses `deepseek-v4-flash` for both generation and judging on the baseline
GPT-4o false-case subset. The target explicitly disables DeepSeek thinking mode
for generation and judging so the output budget goes to visible answer content
instead of `reasoning_content`. It is useful for local iteration, but it is not
an official LongMemEval score.

Latest local diagnostic:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
correct by DeepSeek judge: 24 / 117
accuracy: 0.2051
empty hypotheses: 0
```

Compare the latest thinking-disabled flash run against the previous
implicit-thinking flash run:

```bash
make longmemeval-v1-deepseek-flash-diff
```

Current diff summary: `10` questions became correct only in the
thinking-disabled run, while `13` were correct only in the previous implicit
run. The thinking-disabled run removed empty hypotheses (`4 -> 0`) and reduced
completion tokens (`48,555 -> 8,955`).

Run the broader 500-question DeepSeek flash diagnostic over the compact
CortexDB retrieval log:

```bash
make longmemeval-v1-deepseek-flash-compact-500-check
```

First local result before preference-aware prompt:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
correct by DeepSeek judge: 252 / 500
accuracy: 0.5040
empty hypotheses: 0
```

This first 500-question run exposed a format issue: `single-session-preference`
was `0 / 30` because the reader prompt treated preference questions like factual
lookup questions.

Run the focused preference-format check:

```bash
make longmemeval-v1-deepseek-flash-preference-check
```

Latest local result with the preference-aware generation prompt:

```text
model: deepseek-v4-flash
question type: single-session-preference
before preference-aware prompt: 0 / 30
after preference-aware prompt: 20 / 30
accuracy: 0.6667
empty hypotheses: 0
```

After applying that prompt fix to the full compact-500 run:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
correct by DeepSeek judge: 269 / 500
accuracy: 0.5380
empty hypotheses: 0
single-session-preference: 18 / 30
```

Run the focused multi-session aggregation check:

```bash
make longmemeval-v1-deepseek-flash-multi-session-check
```

Latest local result with the multi-session-aware generation prompt:

```text
model: deepseek-v4-flash
question type: multi-session
before multi-session-aware prompt: 63 / 133
after multi-session-aware prompt: 77 / 133
accuracy: 0.5789
empty hypotheses: 0
```

After applying both the preference-aware and multi-session-aware prompt fixes to
the full compact-500 run:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
correct by DeepSeek judge: 283 / 500
accuracy: 0.5660
empty hypotheses: 0
multi-session: 77 / 133
single-session-preference: 19 / 30
temporal-reasoning: 65 / 133
```

Run the focused temporal-reasoning check:

```bash
make longmemeval-v1-deepseek-flash-temporal-check
```

Latest local result with the temporal-aware generation prompt:

```text
model: deepseek-v4-flash
question type: temporal-reasoning
before temporal-aware prompt: 65 / 133
after temporal-aware prompt: 89 / 133
accuracy: 0.6692
empty hypotheses: 0
```

After applying the preference-aware, multi-session-aware, and temporal-aware
prompt fixes to the full compact-500 run:

```text
model: deepseek-v4-flash
generation thinking: disabled
judge thinking: disabled
correct by DeepSeek judge: 309 / 500
accuracy: 0.6180
empty hypotheses: 0
multi-session: 80 / 133
single-session-preference: 20 / 30
temporal-reasoning: 88 / 133
```

## Current Boundary

- CortexDB owns the retrieval log generation.
- The official LongMemEval repository owns QA generation/evaluation format.
- Official QA scoring needs a GPT-4o-compatible OpenAI key.
- LongMemEval-V2 leaderboard submission needs both web and enterprise runs plus
  the official leaderboard package builder.

## Latest Official Local Evidence

On the official v1 `longmemeval_s_cleaned.json` split, the local CortexDB
retrieval run has been accepted by the official retrieval metric script with:

```text
session recall_all@10 = 0.9021
session ndcg_any@10 = 0.7873
```

The same run was evaluated with official `evaluate_qa.py gpt-4o`:

```text
questions = 500
correct = 383
accuracy = 0.7660
```

This is a full official local score. It is not yet a published leaderboard
entry until the package is submitted to the official maintainers.

## Package For Submission

```bash
make longmemeval-v1-package-submission
```

Output:

```text
target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o.tar.gz
```

The package includes hypotheses, official QA labels, retrieval metrics, dataset
manifest, retrieval report, and a package manifest with checksums. The full
retrieval log is omitted by default because it is about 255 MB; rebuild with
`scripts/longmemeval/package_v1_submission.py --include-retrieval-log` if a
reviewer requests it.
