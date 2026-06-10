# EnterpriseRAG-Bench Official-Clean Mode

`official-clean` is the only mode intended for fair product-quality tracking.
It simulates a real user asking CortexDB a question over an already indexed
enterprise corpus.

## Inference Contract

Allowed at inference time:

```text
question_id
question
retrieved document_ids
document text loaded from generated_data/sources
document metadata that is part of the corpus, such as title/path/source folder
```

Forbidden at inference time:

```text
question_type
source_types
expected_doc_ids
answer_facts
gold_answer
per-question routed selectors based on those fields
direct company_overview.md prompt injection unless it is ingested/retrieved as corpus
```

Gold fields are allowed only in the judge/scoring stage.

The runner passes `--official-clean` into the Rust retrieval binary. In that
mode the binary rejects any question row containing oracle fields, disables
`source_types` filtering, and emits only clean retrieval rows.

## Commands

Run 50 questions:

```bash
make enterprise-rag-bench-official-clean-50
```

Run all 500 questions:

```bash
make enterprise-rag-bench-official-clean-500
```

Run the 100-question held-out/extra split:

```bash
make enterprise-rag-bench-official-clean-heldout
```

Run a cheap held-out contract check without retrieval, answers, or judge calls:

```bash
make enterprise-rag-bench-official-clean-heldout-smoke-check
```

The primary targets record `split_name=primary` in `official_clean_status.json`
and `official_clean_run_report.json`. The held-out target records
`split_name=heldout` and uses the upstream `extra_questions.jsonl` file. This
keeps dev/primary and locked/held-out evidence machine-readable instead of
depending on the shell command name.

Current held-out baseline evidence:

```text
target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/
```

This run reused the full-corpus CortexDB root from the EPIC-16 retrieval SLO
run, stripped oracle fields before inference, generated answers with
`deepseek-v4-flash`, and judged with `deepseek-v4-flash`. The current held-out
score is intentionally recorded as a low baseline, not a product claim:

| Metric | Value |
| --- | ---: |
| Overall | 6.0 |
| Answer correctness | 6.0% |
| Answer completeness | 7.63% |
| Document recall | 33.0% |
| Invalid extra docs | 9.67 |
| Answer tokens | 544,774 |
| Judge tokens | 34,788 |

The held-out retrieval artifact is protected by a local regression gate:

```bash
make enterprise-rag-bench-official-clean-heldout-retrieval-quality-check
```

The gate writes:

```text
target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate_report.json
```

It evaluates only after retrieval has completed, so gold `expected_doc_ids` are
never available to the retrieval/answer inference stages.

A lightweight CI-safe fixture gate exercises the same metric code without the
external 500k benchmark checkout:

```bash
make enterprise-rag-bench-retrieval-quality-fixture-check
```

It uses committed files under
`fixtures/enterprise_rag_bench/retrieval_quality_gate/` and is wired into the
stable Rust GitHub Actions job. This catches broken recall/MRR/nDCG/invalid-doc
accounting in PRs; the full held-out gate above remains the real benchmark
evidence when local EnterpriseRAG artifacts are available.

A second CI-safe fixture gate protects the EPIC-01 hybrid parity contract:

```bash
make enterprise-rag-bench-hybrid-parity-fixture-check
```

It compares a committed `reference.python_fusion.jsonl` retrieval artifact
against a committed `candidate.engine_hybrid.jsonl` artifact and fails if native
engine-hybrid recall/hit/full-recall regresses at top-k, or if any question
regresses. This is a mechanics gate, not a public benchmark claim. Real parity
evidence still comes from comparing full/held-out EnterpriseRAG artifacts
produced by `engine-hybrid` against the historical Python fusion run.

Model-specific aliases:

```bash
make enterprise-rag-bench-official-clean-50-gemma
make enterprise-rag-bench-official-clean-50-gemini
make enterprise-rag-bench-official-clean-50-deepseek

make enterprise-rag-bench-official-clean-500-gemma
make enterprise-rag-bench-official-clean-500-gemini
make enterprise-rag-bench-official-clean-500-deepseek

make enterprise-rag-bench-official-clean-heldout-gemma
make enterprise-rag-bench-official-clean-heldout-gemini
make enterprise-rag-bench-official-clean-heldout-deepseek
```

Mixed answer/judge providers:

```bash
make enterprise-rag-bench-official-clean-50 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemma \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemini
```

Retrieval mode selection:

```bash
make enterprise-rag-bench-official-clean-50-deepseek \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-keyword
```

Supported modes:

```text
cached-lexical  existing cached `.aci` BM25-like benchmark path
engine-keyword  native Database::search_keyword path
engine-hybrid   native Database::search_cells(SearchMode::Hybrid) path
```

`engine-hybrid` requires query vectors generated from the clean question text,
not from gold fields. It also needs document vectors at ingest time unless the
reused database was already ingested with `vector=...` payload metadata:

```bash
make enterprise-rag-bench-official-clean-50-deepseek \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-hybrid \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS=target/question_vectors.jsonl \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS=target/document_vectors.jsonl
```

Build official-clean vectors locally from question text and corpus documents:

```bash
make enterprise-rag-bench-official-clean-vectors-50
make enterprise-rag-bench-official-clean-vectors-500
```

The vector builder reads `CORTEXDB_EMBEDDING_URL`,
`CORTEXDB_EMBEDDING_MODEL`, and `CORTEXDB_EMBEDDING_API_KEY` from `.env` by
default, calls an OpenAI-compatible embeddings endpoint, unit-normalizes the
float vectors, and writes `i16` vectors accepted by the engine payload format.
It does not read `question_type`, `source_types`, `expected_doc_ids`,
`answer_facts`, or `gold_answer` for inference decisions.

For a cheap smoke run, limit the number of document vectors explicitly:

```bash
make enterprise-rag-bench-official-clean-vectors-50 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT=50
```

For an honest `engine-hybrid` benchmark over the full EnterpriseRAG corpus,
do not set `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT`.

Run 50 questions with generated vectors:

```bash
make enterprise-rag-bench-official-clean-50-deepseek \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-hybrid \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-hybrid \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS=target/enterprise-rag-bench/official-clean/50/query_vectors.jsonl \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS=target/enterprise-rag-bench/official-clean/50/document_vectors.jsonl
```

## Mode Comparison

Use `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL` to keep artifacts from
different retrieval modes side by side:

```bash
make enterprise-rag-bench-official-clean-retrieval-50-cached
make enterprise-rag-bench-official-clean-retrieval-50-engine-keyword
make enterprise-rag-bench-official-clean-retrieval-50-engine-hybrid
```

These aliases run `stage=retrieval`, which means `prepare + retrieve` only. They
do not call answer generation or judging, so they do not spend LLM tokens.

For a fast mechanics check, run a smoke retrieval over only the first 50 corpus
documents:

```bash
make enterprise-rag-bench-official-clean-retrieval-smoke-50
```

This target sets `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS=50`. Do not
use it as benchmark evidence; honest retrieval scores must omit
`ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS` and index/reuse the full
corpus.

Then compare retrieval recall. This comparison reads gold fields only after
retrieval, as evaluation evidence, not during inference:

```bash
make enterprise-rag-bench-official-clean-compare-retrieval
```

Outputs:

```text
target/enterprise-rag-bench/official-clean/retrieval_comparison.json
target/enterprise-rag-bench/official-clean/retrieval_comparison.jsonl
target/enterprise-rag-bench/official-clean/retrieval_comparison.md
```

Reuse an already indexed local database for faster repeated checks:

```bash
make enterprise-rag-bench-official-clean-50-deepseek \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB=1
```

Fresh runs remain the default. Use reuse only after a successful clean run has
created `target/enterprise-rag-bench/official-clean/<size>/cortexdb`.

You can also point reuse at an existing full-corpus CortexDB root:

```bash
make enterprise-rag-bench-official-clean-50-deepseek \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB=1 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT=target/enterprise-rag-bench/cortexdb-full
```

## Outputs

For 50 questions:

```text
target/enterprise-rag-bench/official-clean/50/questions.clean.jsonl
target/enterprise-rag-bench/official-clean/50/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/50/answer-<provider>/answers.jsonl
target/enterprise-rag-bench/official-clean/50/answer-<provider>/answer_generation_report.json
target/enterprise-rag-bench/official-clean/50/answer-<provider>/judge-<provider>/results.json
```

The held-out split writes the same layout under:

```text
target/enterprise-rag-bench/official-clean/100/
```

The answer generation report includes prompt/completion/total token counts.
The judge results use the official combined formula:

```text
overall = mean(completeness_pct if answer_correct else 0)
```

## Logging

Long runs print timestamped stage logs:

```text
[official-clean ...] step 1/4 prepare: start
[official-clean ...] begin prepare clean questions
[official-clean ...] step 2/4 retrieve: start
[official-clean ...] begin build retrieval binary
[official-clean ...] begin retrieve with CortexDB
[enterprise-rag-retrieval +  12.3s] begin corpus ingest
[enterprise-rag-retrieval + 210.5s] begin checkpoint
[enterprise-rag-retrieval + 330.1s] load retrieval index from checkpoint segments
[official-clean-answer ...] begin answer generation
[answer-runner ...] answer_generation: 50/500 questions (10.0%) elapsed=2m03s rate=0.41/s eta=18m27s total_tokens=123456 last_question_id=qst_0050
[official-clean-judge ...] begin judging
[judge-runner ...] judging: 50/500 questions (10.0%) elapsed=1m38s rate=0.51/s eta=14m43s total_tokens=23456 last_question_id=qst_0050
```

This makes it visible whether the run is indexing, checkpointing, retrieving,
answering, or judging. Answer and judge progress lines include
`completed/total`, percent, elapsed time, rate, ETA, token totals, and the last
completed `question_id`.

The orchestrator also writes a live log and status file:

```text
target/enterprise-rag-bench/official-clean/<size>/official_clean_run.log
target/enterprise-rag-bench/official-clean/<size>/official_clean_status.json
target/enterprise-rag-bench/official-clean/<size>/prepare_status.json
target/enterprise-rag-bench/official-clean/<size>/retrieval_progress.log
target/enterprise-rag-bench/official-clean/<size>/retrieval_status.json
target/enterprise-rag-bench/official-clean/<size>/retrieval_report.json
target/enterprise-rag-bench/official-clean/<size>/answer-<provider>/answer_status.json
target/enterprise-rag-bench/official-clean/<size>/answer-<provider>/judge-<provider>/judge_status.json
```

`official_clean_run_report.json` also embeds `progress_artifacts` and a compact
`summary` block. Use it after a run to see the final retrieval report path,
answer token totals, judge token totals, and final score without manually
opening the child reports.

`official_clean_status.json` embeds the active child status when a stage has
one. During retrieval that child status shows whether the Rust runner is
loading questions, ingesting documents, checkpointing, loading the retrieval
index, or retrieving question rows.

`retrieval_report.json` also records full-corpus performance evidence under
`performance`: total duration, ingest duration and documents/sec, checkpoint
duration, retrieval duration and questions/sec, plus process RSS / peak RSS.
Use this report when checking whether a 500-question run used the full
EnterpriseRAG corpus within the local single-node SLO envelope.

Use these while a long run is active:

```bash
tail -f target/enterprise-rag-bench/official-clean/500/official_clean_run.log
cat target/enterprise-rag-bench/official-clean/500/official_clean_status.json
cat target/enterprise-rag-bench/official-clean/500/retrieval_status.json
cat target/enterprise-rag-bench/official-clean/500/answer-deepseek/answer_status.json
cat target/enterprise-rag-bench/official-clean/500/answer-deepseek/judge-deepseek/judge_status.json
```

Or use the status viewer, which reads the same status JSON and renders the
active stage, step, current operation, PID, model, child status, token counters,
current/last question id, log file, and artifact paths:

```bash
make enterprise-rag-bench-official-clean-status
```

For a 50-question run:

```bash
make enterprise-rag-bench-official-clean-status \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_SIZE=50
```

For a labeled run:

```bash
make enterprise-rag-bench-official-clean-status \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_SIZE=50 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_RUN_LABEL=engine-hybrid
```

For live polling:

```bash
make enterprise-rag-bench-official-clean-status \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_WATCH=1 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_INTERVAL_SECONDS=10
```

To include the latest log lines in each status render:

```bash
make enterprise-rag-bench-official-clean-status \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_TAIL_LINES=20
```

Vector generation writes its own progress artifacts:

```text
target/enterprise-rag-bench/official-clean/vector_progress.log
target/enterprise-rag-bench/official-clean/vector_status.json
target/enterprise-rag-bench/official-clean/2/heldout-smoke-check/official_clean_gate.log
target/enterprise-rag-bench/official-clean/2/heldout-smoke-check/official_clean_gate_status.json
target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate.log
target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate_status.json
```

The gate status files are useful when a check is run directly from `make`:
they show whether the script is loading inputs, validating clean artifacts,
scoring retrieval rows, writing markdown, or failing on a threshold.

Progress frequency is configurable:

```bash
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY=50000
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY=10
```

Use the retrieval interval for corpus indexing progress and the general interval
for answer/judge progress.

## Diagnostic Runs

Older `routed-v*`, `type-aware-*`, and selector scripts are diagnostic tools.
They may read labels for analysis and ablation. Do not use them as public
benchmark evidence unless the result is clearly marked diagnostic.
