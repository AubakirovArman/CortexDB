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

## Current Boundary

- CortexDB owns the retrieval log generation.
- The official LongMemEval repository owns QA generation/evaluation format.
- Official QA scoring needs a GPT-4o-compatible OpenAI key.
- LongMemEval-V2 leaderboard submission needs both web and enterprise runs plus
  the official leaderboard package builder.

## Latest Official-Script Retrieval Evidence

On the official v1 `longmemeval_s_cleaned.json` split, the local CortexDB
retrieval run has been accepted by the official retrieval metric script with:

```text
session recall_all@10 = 0.9021
session ndcg_any@10 = 0.7873
```

This is not the final QA leaderboard score. The QA score still requires
official generation output plus `evaluate_qa.py`.
