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
that cache for all questions in the run. It also uses the benchmark-provided
`source_types` metadata as a runtime source filter before filling any remaining
top-k slots globally. This keeps the 50-question gate focused on retrieval
quality instead of repeatedly decoding the multi-gigabyte `.aci` file for every
question.

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
| retrieval mode | `keyword + source_types top-k=10` |
| average document recall | `56.35%` |
| average invalid extra docs | `9.19` |
| correctness / completeness | `0.0% / 0.0%` |

This is a baseline, not a final EnterpriseRAG-Bench answer score. It proves the
full-corpus local retrieval path works and shows where the next quality work
should focus: semantic and project-related retrieval.

## Embedding Rerank Gate

CortexDB also includes a local embedding rerank harness for the 50-question
gate. It does not re-ingest the corpus. Instead, it asks the existing retrieval
runner for a wider candidate set, embeds the question and candidate documents,
reranks by cosine similarity, and then runs the same official retrieval-only
evaluator over the final top-k documents.

The harness reads embedding credentials from a local env file. Do not commit
that file or paste the key in logs:

```text
CORTEXDB_EMBEDDING_URL=...
CORTEXDB_EMBEDDING_MODEL=...
CORTEXDB_EMBEDDING_API_KEY=...
```

Run a small endpoint smoke first:

```bash
make enterprise-rag-bench-embedding-rerank-existing-50 \
  ENTERPRISE_RAG_BENCH_RERANK_LIMIT=3
```

Run the full 50-question rerank and official retrieval-only metrics:

```bash
make enterprise-rag-bench-official-retrieval-only-metrics-embedding-rerank-existing-50
```

Artifacts:

```text
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_candidates_top50.jsonl
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_embedding_rerank_answers.jsonl
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_embedding_rerank_metrics.json
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_embedding_rerank_report.json
target/enterprise-rag-bench/retrieval/embedding_cache.jsonl
```

This gate is local-only for now. It is intentionally not wired into GitHub
Actions because it requires an external embedding endpoint and a secret key.

Latest local embedding-rerank evidence:

| Field | Value |
| --- | ---: |
| subset questions | `50` |
| candidate retrieval | `keyword + source_types top-k=50` |
| rerank model | `BAAI/bge-m3` |
| final top-k | `10` |
| average document recall | `68.85%` |
| average invalid extra docs | `9.09` |
| correctness / completeness | `0.0% / 0.0%` |

Correctness and completeness are still zero because this is still a
retrieval-only pass with empty answers. The improvement is in supporting
document recall: reranking a wider CortexDB candidate set with embeddings
improved recall by `+12.50` percentage points over the top-10 keyword/source
baseline.

Recall by question type in the latest local rerank:

| Type | Avg recall |
| --- | ---: |
| basic | `94.12%` |
| semantic | `38.46%` |
| intra_document_reasoning | `100.00%` |
| project_related | `24.65%` |
| constrained | `66.67%` |
| conflicting_info | `100.00%` |
| completeness | `68.75%` |
| miscellaneous | `50.00%` |

## Answer Generation Gate

Generate DeepSeek answers from the retrieved documents:

```bash
make enterprise-rag-bench-deepseek-answers-50
```

Generate DeepSeek answers from the embedding-reranked retrieval output:

```bash
make enterprise-rag-bench-deepseek-answers-embedding-rerank-50
```

Then run the official answer evaluator:

```bash
make enterprise-rag-bench-official-answer-metrics-50
```

For embedding-reranked answers:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-50
```

The official answer evaluator requires an LLM provider supported by the upstream
benchmark. The retrieval-only target is local and cheap; the answer metrics
target is the one that becomes comparable to leaderboard-style results.

### Official judge-backed answer metrics

The upstream evaluator reads its judge config from `LLM_PROVIDER`,
`LLM_API_KEY`, and `LLM_MODEL_NAME`. CortexDB keeps those values out of the
repository and maps a local env file into the upstream names only for the
evaluation subprocess:

```bash
make enterprise-rag-bench-deepseek-answers-embedding-rerank-50
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke
```

The judge targets intentionally do not regenerate answers. They require the
existing reranked `answers.jsonl` artifact and fail fast if it is missing.

By default this reads `OPENAI_API_KEY` from
`/mnt/hf_model_weights/arman/3bit/sites/.env`, maps it to `LLM_API_KEY`, and
uses `gpt-4o-mini` as the upstream `openai` judge. Override without editing
tracked files:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke \
  ENTERPRISE_RAG_BENCH_JUDGE_ENV_FILE=/path/to/.env \
  ENTERPRISE_RAG_BENCH_JUDGE_MODEL=gpt-4o-mini
```

After the smoke pass succeeds, run the full 50-question judged gate:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-50
```

The smoke target defaults to a `120s` timeout and the full target defaults to a
`900s` timeout. Override these locally with
`ENTERPRISE_RAG_BENCH_JUDGE_SMOKE_TIMEOUT_SECONDS` or
`ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS` if the upstream judge endpoint is
slow.

This path is intentionally local-only and not wired into GitHub Actions. It
spends judge-model tokens and depends on external credentials. DeepSeek answer
generation remains separate; the official judge itself uses the upstream
EnterpriseRAG-Bench provider contract, which currently supports OpenAI or
Anthropic SDK clients rather than arbitrary chat-completions endpoints.

Embedding-reranked answer artifacts are written separately:

```text
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/answers.jsonl
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/answer_generation_report.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics_judge_smoke.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics_judge.json
```

Latest local embedding-reranked answer gate:

| Field | Value |
| --- | ---: |
| model | `deepseek-v4-flash` |
| thinking | `disabled` |
| questions | `50` |
| prompt tokens | `128,507` |
| completion tokens | `3,044` |
| total tokens | `131,551` |
| generation wall time | `43.33s` |
| official evaluator mode | `--no-correction --skip-citation-stripping` |
| average correctness | `0.0%` |
| average completeness | `0.0%` |
| average document recall | `68.85%` |

The generated answers are non-empty, but this gate does not yet produce an
official answer-quality score. The current prompt/output contract still needs
EnterpriseRAG-specific tuning. Treat the reranked run as validated retrieval
evidence, not as a leaderboard-ready answer result.

Run answer error analysis:

```bash
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-50
```

Latest local analysis artifact:

```text
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/answer_error_analysis.json
```

Latest local answer analysis:

| Field | Value |
| --- | ---: |
| non-empty answers | `50 / 50` |
| doc recall > 0 but answer_correct=false | `35` |
| blank correctness reasoning rows | `50` |
| likely judge/format issue bucket | `3` |
| answer missing gold facts bucket | `17` |
| abstained despite evidence bucket | `15` |
| retrieval miss bucket | `15` |

The upstream metrics script uses its own LLM judge for fact validation and
wholistic correctness. If that judge is not configured, correctness and
completeness can collapse to `0.0%` even when candidate answers are non-empty
and retrieved documents include gold evidence. Judge-backed answer-quality runs
must configure the upstream judge environment (`LLM_PROVIDER`, `LLM_API_KEY`,
and `LLM_MODEL_NAME`) before claiming an official answer score.

Latest judge-backed smoke:

| Field | Value |
| --- | ---: |
| questions | `3` |
| average correctness | `0.0%` |
| average completeness | `0.0%` |
| average document recall | `100.0%` |
| average invalid extra | `9.0` |

The smoke proves the upstream judge environment bridge works. The failing answer
score means the next tuning target is the EnterpriseRAG answer prompt/output
contract, not the retrieval path or local env wiring.

Latest judge-backed 50-question gate:

| Field | Value |
| --- | ---: |
| questions scored | `50 / 50` |
| average correctness | `0.0%` |
| average completeness | `0.0%` |
| combined correctness * completeness | `0.0` |
| average document recall | `68.85%` |
| average invalid extra | `9.09` |

The judged full gate confirms that the benchmark bridge is operational, but
answer quality is not yet competitive. The next work item is to reduce
over-broad evidence (`invalid_extra_docs`) and tune generated answers to match
EnterpriseRAG gold facts without unnecessary abstention.

Judge-backed answer error analysis:

```bash
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-judge-50
```

Latest judge-backed failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `17` |
| `abstained_with_evidence` | `15` |
| `retrieval_miss` | `15` |
| `likely_judge_or_format_issue` | `3` |

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
- embedding rerank helper for local source-aware candidates;
- DeepSeek answer generation helper;
- Make targets for retrieval-only and answer metrics.

Not yet optimized:

- embedding retrieval;
- production-grade embedding retrieval;
- answer-aware reranking;
- ContextPack-specific enterprise prompt tuning;
- leaderboard submission package.
