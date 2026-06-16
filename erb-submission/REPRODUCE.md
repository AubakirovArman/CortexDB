# CortexDB EnterpriseRAG-Bench Reproduction Guide

This package records one current EnterpriseRAG-Bench result. It is designed so a
third party can re-score the exact answers with the official judge later.

Contact for submission: joachim@onyx.app. Repo:
https://github.com/AubakirovArman/CortexDB.

## 1. Current Result

Full 500 questions, no-oracle inference, official-clean prompt shape:

| Metric | Value |
| --- | ---: |
| Overall combined correctness/completeness | **47.74** |
| Correctness | 50.0% |
| Completeness | 53.7% |
| Document recall | 55.71% |
| Invalid extra docs | 9.23 |

Configuration:

| Stage | Setting |
| --- | --- |
| answerer | `google/gemma-4-31B-it` |
| judge | `gemini-3.5-flash` |
| questions | 500 |
| prompt | `official-clean-v1` |
| context mode | `question-window-digest-ranked` |
| active context | up to 8 docs, 8000 chars per doc |
| answer tokens | 6,150,627 total |
| judge tokens | 348,992 total |

Honesty note: this is the single official interim number for this package. It is
not a leaderboard claim because the official `gpt-5.4` judge has not been run on
these Gemma answers yet. Re-score `answers.jsonl` with the official evaluator
and `LLM_MODEL_NAME=gpt-5.4` before publishing a leaderboard-comparable number.

## 2. System Under Test

The inference path uses only:

- `question_id`
- question text
- CortexDB retrieval output
- retrieved document text

The benchmark oracle fields are stripped before retrieval or answer generation:

- `answer_facts`
- `expected_doc_ids`
- `gold_answer`
- `question_type`
- `source_types`

Gold metadata is judge-only. It is not available to the database, retriever, or
Gemma answerer.

## 3. Package Files

| File | Purpose |
| --- | --- |
| `answers.jsonl` | Exact 500 Gemma answers used for the current result. |
| `official_results.json` | Gemini-judge results, Overall 47.74. |
| `config_answer_report.json` | Gemma answer-generation provenance. |
| `official_clean_gate_report.json` | Clean question/retrieval validation. |
| `oracle_usage_audit.json` | Artifact and script oracle-field audit. |

## 4. Re-score Current Answers

Use the EnterpriseRAG-Bench evaluator against this package's `answers.jsonl`.
For the final official number, use `gpt-5.4`:

```bash
cd target/external-benchmarks/EnterpriseRAG-Bench
ANS=/mnt/hf_model_weights/arman/3bit/sites/CortexDB/erb-submission/answers.jsonl

LLM_API_KEY=<openai key> \
LLM_MODEL_NAME=gpt-5.4 \
CHEAP_LLM_MODEL_NAME=gpt-5-mini \
python -m src.scripts.answer_evaluation.metrics_based_eval \
  --answers-file "$ANS" \
  --questions-file questions.jsonl \
  --results-file results.json \
  --updated-questions-file questions_updated.jsonl \
  --parallelism 8
```

Until that re-judge exists, report only the interim Gemini-judge number above.

## 5. Recreate The Local Run

The selected run artifact came from:

```text
target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/
```

It contains:

```text
questions.clean.jsonl
retrieval.clean.jsonl
answer-gemma/answers.jsonl
answer-gemma/judge-gemini/results.json
answer-gemma/official_clean_run_report.json
oracle_usage_audit.json
```

The core command shape is:

```bash
make enterprise-rag-bench-official-clean-500 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=real-cortexdb-gemma500-20260615T194239Z \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemma \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemini \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CONTEXT_MODE=question-window-digest-ranked
```

Provider environment must point at the same Gemma answer endpoint and Gemini
judge key used by the local run.

## 6. No-Oracle Checks

Artifact audit:

```bash
python3 scripts/enterprise_rag_bench/oracle_usage_audit.py \
  --clean-questions target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/questions.clean.jsonl \
  --clean-retrieval target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/retrieval.clean.jsonl \
  --answers-file target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/answer-gemma/answers.jsonl \
  --report target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/oracle_usage_audit.json
```

Inference-path guard:

```bash
make erb-oracle-audit
```

Clean gate:

```bash
python3 scripts/enterprise_rag_bench/official_clean_gate.py \
  --run-report target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/answer-gemma/official_clean_run_report.json \
  --report target/enterprise-rag-bench/official-clean/500/real-cortexdb-gemma500-20260615T194239Z/official_clean_gate_report.json \
  --require-retrieval
```

All three checks must pass before the package is refreshed.
