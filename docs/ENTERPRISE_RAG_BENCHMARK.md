# EnterpriseRAG-Bench

EnterpriseRAG-Bench is the next public benchmark track for CortexDB after
LongMemEval and MultiHop-RAG. It tests retrieval and answer generation over a
large synthetic enterprise corpus from Onyx: roughly 500k internal documents and
500 questions.

Official project:

```text
https://github.com/onyx-dot-app/EnterpriseRAG-Bench
```

## What This Measures

The benchmark is useful because it looks like a company knowledge base, not a
small public QA corpus. It covers Slack, Gmail, Linear, Google Drive, HubSpot,
Fireflies, GitHub, Jira, and Confluence-like data. Question types include basic
lookup, semantic lookup, intra-document reasoning, project-related aggregation,
constraints, conflicts, completeness, high-level questions, and unavailable
information.

For CortexDB this specifically checks:

- can the engine ingest a large enterprise-shaped corpus;
- can keyword/ContextPack retrieval return the right supporting documents;
- can answer generation use only retrieved evidence;
- can the output be scored by the official evaluator.

## Local Setup

Clone or update the official repo:

```bash
make enterprise-rag-bench-official-repo
```

Run input validation:

```bash
make enterprise-rag-bench-preflight
```

This writes:

```text
target/enterprise-rag-bench/preflight_report.json
```

## First 50-Question Gate

Build a deterministic balanced subset:

```bash
make enterprise-rag-bench-balanced-50
```

Run a cheap end-to-end smoke check first:

```bash
make enterprise-rag-bench-official-retrieval-only-metrics-smoke
```

This indexes only `ENTERPRISE_RAG_BENCH_SMOKE_MAX_DOCUMENTS` documents, so recall
is not a quality signal. It only proves the local CortexDB runner, JSONL answer
format, and official evaluator wiring are working.

Run CortexDB retrieval over that subset:

```bash
make enterprise-rag-bench-cortexdb-retrieval-50
```

This writes an official-compatible JSONL answer file:

```text
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_answers.jsonl
```

The rows contain `question_id`, empty `answer`, and retrieved `document_ids`.
That is intentionally retrieval-only. It validates document recall without
spending LLM tokens on answer generation.

Corpus ingest uses `Database::put_cells` with
`ENTERPRISE_RAG_BENCH_INGEST_BATCH_SIZE` (default `1000`) so large runs still go
through CortexDB's WAL/engine path without doing one sync boundary per document.
After checkpoint, the runner loads the persisted lexical index once and reuses
that cache for all questions in the run. This keeps the 50-question gate focused
on retrieval quality instead of repeatedly decoding the multi-gigabyte `.aci`
file for every question.

Run retrieval-only official metrics:

```bash
make enterprise-rag-bench-official-retrieval-only-metrics-50
```

If the retrieval JSONL already exists and you only want to re-run the official
evaluator, use:

```bash
make enterprise-rag-bench-official-retrieval-only-metrics-existing-50
```

Because answers are empty, correctness and completeness are expected to be zero.
The useful field in this pass is document recall.

Latest local retrieval-only result:

| Field | Value |
| --- | ---: |
| corpus documents indexed | `511,958` |
| subset questions | `50` |
| retrieval mode | `keyword top-k=10` |
| average document recall | `43.56%` |
| average invalid extra docs | `9.43` |
| correctness / completeness | `0.0% / 0.0%` |

This is a baseline, not a final EnterpriseRAG-Bench answer score. It proves the
full-corpus local retrieval path works and shows where the next quality work
should focus: semantic and project-related retrieval.

## Answer Generation Gate

Generate DeepSeek answers from the retrieved documents:

```bash
make enterprise-rag-bench-deepseek-answers-50
```

Then run the official answer evaluator:

```bash
make enterprise-rag-bench-official-answer-metrics-50
```

The official answer evaluator requires an LLM provider supported by the upstream
benchmark. The retrieval-only target is local and cheap; the answer metrics
target is the one that becomes comparable to leaderboard-style results.

## Full Run

The full retrieval target is:

```bash
make enterprise-rag-bench-cortexdb-retrieval-full
```

Do not start with the full run. Use the 50-question gate first, inspect document
recall, then decide whether to run full answer generation.

## Official Evaluator Environment

The upstream evaluator has its own Python dependencies (`pydantic`, `openai`,
`anthropic`, and others). Keep them isolated from CortexDB:

```bash
make enterprise-rag-bench-official-env
```

This creates:

```text
target/enterprise-rag-bench/.venv/
```

The official metrics targets use that virtualenv automatically.

## Current Scope

Current harness scope:

- local official repo clone under `target/external-benchmarks/EnterpriseRAG-Bench`;
- `preflight.py` for input validation;
- `build_balanced_subset.py` for the first 50-question gate;
- `enterprise_rag_bench_retrieval` Rust binary using `cortex-engine`;
- DeepSeek answer generation helper;
- Make targets for retrieval-only and answer metrics.

Not yet optimized:

- embedding retrieval;
- source-aware metadata filtering;
- reranking;
- ContextPack-specific enterprise prompt tuning;
- leaderboard submission package.
