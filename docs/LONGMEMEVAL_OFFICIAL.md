# LongMemEval Official Score Path

Status: CortexDB has an official LongMemEval v1 retrieval harness. The final
QA leaderboard score is still gated on official hypothesis generation and
official GPT-4o evaluation.

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

## Current Local Evidence

Local full-run evidence on the official `longmemeval_s_cleaned.json` split:

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

## Submission Gap

To appear in an official LongMemEval list or leaderboard, the remaining work is:

1. Generate official QA hypotheses with the official generation script.
2. Score those hypotheses with the official evaluator and GPT-4o-compatible
   judge key.
3. Package the run according to the official benchmark or LongMemEval-V2
   leaderboard requirements.
4. Submit the package to the official maintainers.

Until those four steps are done, CortexDB has official-script retrieval
evidence, but not an official published LongMemEval leaderboard entry.
