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
# CORTEXDB_EMBEDDING_API_KEY must be set locally; do not commit it.
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

Then run the local answer metrics path:

```bash
make enterprise-rag-bench-official-answer-metrics-50
```

For embedding-reranked answers:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-50
```

The local answer metrics path now defaults to the CortexDB DeepSeek judge. The
retrieval-only target is local and cheap; the answer metrics target spends
external model tokens and must be reported as a local DeepSeek-judged gate
unless a separate upstream submission package is produced.

### DeepSeek judge-backed answer metrics

CortexDB's local EnterpriseRAG answer-quality gates now use DeepSeek for both
answer generation and answer judging. The judge script writes the same metrics
shape consumed by `analyze_answer_errors.py`, but it is a local CortexDB judge
path rather than an upstream official leaderboard judge.

```bash
make enterprise-rag-bench-deepseek-answers-embedding-rerank-50
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke
```

The judge targets intentionally do not regenerate answers. They require the
existing reranked `answers.jsonl` artifact and fail fast if it is missing.

By default this uses the same local DeepSeek key file as answer generation and
the `deepseek-v4-flash` model:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke \
  ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE=/path/to/deepseek-key \
  ENTERPRISE_RAG_BENCH_JUDGE_MODEL=deepseek-v4-flash
```

After the smoke pass succeeds, run the full 50-question judged gate:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-50
```

The smoke target defaults to a `120s` timeout and the full target defaults to a
`900s` timeout. Override these locally with
`ENTERPRISE_RAG_BENCH_JUDGE_SMOKE_TIMEOUT_SECONDS` or
`ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS` if the DeepSeek endpoint is slow.

This path is intentionally local-only and not wired into GitHub Actions. It
spends judge-model tokens and depends on external credentials. For upstream
official leaderboard-style claims, rerun with the benchmark maintainers'
required evaluator contract and package those artifacts separately.

Embedding-reranked answer artifacts are written separately:

```text
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/answers.jsonl
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/answer_generation_report.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics_judge_smoke.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank/official_metrics_judge.json
```

Baseline local embedding-reranked answer gate:

| Field | Value |
| --- | ---: |
| model | `deepseek-v4-flash` |
| thinking | `disabled` |
| questions | `50` |
| prompt tokens | `128,507` |
| completion tokens | `3,044` |
| total tokens | `131,551` |
| generation wall time | `43.33s` |
| judge model | `deepseek-v4-flash` |
| average correctness | `28.0%` |
| average completeness | `28.65%` |
| combined correctness * completeness | `20.37` |
| average document recall | `68.85%` |

The generated answers are non-empty and now have a real judge-backed score.
Treat this as a local 50-question gate, not a leaderboard claim.

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
| doc recall > 0 but answer_correct=false | `23` |
| blank correctness reasoning rows | `0` |
| likely judge/format issue bucket | `3` |
| answer missing gold facts bucket | `17` |
| abstained despite evidence bucket | `15` |
| retrieval miss bucket | `15` |

If the judge is not configured, correctness and completeness can collapse to
`0.0%` even when candidate answers are non-empty and retrieved documents include
gold evidence. Judge-backed answer-quality runs must configure the local
DeepSeek key file before claiming a CortexDB local answer score.

Latest judge-backed smoke:

| Field | Value |
| --- | ---: |
| questions | `3` |
| judge model | `deepseek-v4-flash` |
| average correctness | `66.67%` |
| average completeness | `50.0%` |
| average document recall | `100.0%` |
| average invalid extra | `9.0` |

The smoke proves the local DeepSeek judge bridge works.

Baseline judge-backed 50-question gate:

| Field | Value |
| --- | ---: |
| questions scored | `50 / 50` |
| judge model | `deepseek-v4-flash` |
| average correctness | `28.0%` |
| average completeness | `28.65%` |
| combined correctness * completeness | `20.37` |
| average document recall | `68.85%` |
| average invalid extra | `9.09` |

The judged full gate confirms that the benchmark bridge is operational and that
answer quality is now measurable.

Experimental v2 answer prompt:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-v2-judge-50
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-v2-judge-50
```

Latest v2 result:

| Field | Baseline | v2 |
| --- | ---: | ---: |
| average correctness | `28.0%` | `30.0%` |
| average completeness | `28.65%` | `30.26%` |
| combined correctness * completeness | `20.37` | `18.52` |
| abstained-with-evidence bucket | `15` | `4` |
| answer-missing-gold-facts bucket | `17` | `30` |

v2 reduces unnecessary abstention but is not the new default because its
combined score regressed.

Experimental v3 question-window context packing:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-v3-windowed-judge-50
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-v3-windowed-judge-50
```

v3 keeps the same embedding-reranked retrieval output but changes answer
context construction from "first N characters of each document" to
question-aware document windows. This is a context-packing improvement, not a
retrieval improvement: it helps when the correct document was retrieved but the
answer fact was below the leading snippet.

Latest v3 result:

| Field | Baseline | v2 | v3 windowed |
| --- | ---: | ---: | ---: |
| average correctness | `28.0%` | `30.0%` | `52.0%` |
| average completeness | `28.65%` | `30.26%` | `46.52%` |
| combined correctness * completeness | `20.37` | `18.52` | `40.08` |
| average document recall | `68.85%` | `68.85%` | `68.85%` |
| answer generation prompt tokens | `128,507` | `237,057` | `463,245` |
| answer generation completion tokens | `3,044` | `2,204` | `4,611` |
| answer generation wall time | `43.33s` | `51.73s` | `69.73s` |
| abstained-with-evidence bucket | `15` | `4` | `4` |
| answer-missing-gold-facts bucket | `17` | `30` | `26` |
| retrieval-miss bucket | `15` | `15` | `15` |

v3 was the first strong local 50-question judged gate. It is intentionally kept
as a separate target because it spends more generation tokens than the baseline
without changing retrieval. Its remaining hard limit is retrieval recall: the
15 retrieval-miss bucket cannot be fixed by answer prompt or context packing
alone.

Experimental v4 fused retrieval plus question-window context packing:

```bash
make enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates-wide
make enterprise-rag-bench-embedding-rerank-wide-existing-50
make enterprise-rag-bench-embedding-rerank-fused-existing-50
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v4-windowed-judge-50
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v4-windowed-judge-50
```

v4 keeps the v3 question-aware answer windows and changes retrieval. It builds
two embedding-reranked retrieval lists, one from the normal top-50 candidate
pool and one from a wider top-500 candidate pool, then fuses them with
reciprocal-rank fusion. This catches documents that were present only deep in
the keyword/source candidate list while preserving documents that the narrower
rerank already placed well.

Artifacts:

```text
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_candidates_top500.jsonl
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_embedding_rerank_top500_answers.jsonl
target/enterprise-rag-bench/retrieval/cortexdb_balanced_50_embedding_rerank_fused_answers.jsonl
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank-fused-v4-windowed/answers.jsonl
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank-fused-v4-windowed/official_metrics_judge.json
target/enterprise-rag-bench/qa/deepseek-balanced-50-embedding-rerank-fused-v4-windowed/answer_error_analysis_judge.json
```

Latest v4 result with the local DeepSeek judge:

| Field | Baseline | v3 windowed | v4 fused |
| --- | ---: | ---: | ---: |
| average correctness | `28.0%` | `52.0%` | `52.0%` |
| average completeness | `28.65%` | `46.52%` | `61.42%` |
| combined correctness * completeness | `20.37` | `40.08` | `31.94` |
| average document recall | `68.85%` | `68.85%` | `79.23%` |
| average invalid extra documents | `9.09` | `9.09` | `8.98` |
| answer generation prompt tokens | `128,507` | `463,245` | `465,749` |
| answer generation completion tokens | `3,044` | `4,611` | `4,913` |
| answer generation wall time | `43.33s` | `69.73s` | `73.39s` |
| judge total tokens | n/a | n/a | `30,124` |
| retrieval-miss bucket | `15` | `15` | `10` |
| answer-missing-gold-facts bucket | `17` | `26` | `29` |

Local gold-presence checks over the final top-10 document lists showed:

| Retrieval output | Gold docs found in top-10 |
| --- | ---: |
| top-50 embedding rerank | `35 / 47` (`74.47%`) |
| top-500 embedding rerank | `38 / 47` (`80.85%`) |
| fused top-50 + top-500 rerank | `40 / 47` (`85.11%`) |

The first wide rerank is expensive because it embeds many more candidate
documents. In the local v4 run it embedded `18,544` new texts and grew the
embedding cache to `21,028` vectors. Repeated runs are cheaper if the cache is
kept, but this remains a local benchmark path rather than a CI gate.

v4 improves document recall, but it is no longer the best local DeepSeek-judged
answer gate. Its main remaining quality issue is answer selection over
conflicting or similar evidence.

Experimental v5 evidence-selection prompt:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v5-windowed-judge-50
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v5-windowed-judge-50
```

v5 keeps the same fused retrieval output as v4 and changes only the answer
prompt. The prompt explicitly asks the model to choose exact evidence by entity,
date, path, header, number, version, and other anchors before writing the final
answer.

Latest v5 result with the local DeepSeek judge:

| Field | v4 fused | v5 evidence-selection |
| --- | ---: | ---: |
| average correctness | `52.0%` | `58.0%` |
| average completeness | `61.42%` | `63.62%` |
| combined correctness * completeness | `31.94` | `36.90` |
| average document recall | `79.23%` | `79.23%` |
| average invalid extra documents | `8.98` | `8.98` |
| answer generation prompt tokens | `465,749` | `476,499` |
| answer generation completion tokens | `4,913` | `2,354` |
| answer generation total tokens | `470,662` | `478,853` |
| answer generation wall time | `73.39s` | `59.70s` |
| judge total tokens | `30,124` | `27,574` |

v5 is the current best local 50-question DeepSeek-judged gate. The improvement
is answer-stage only: retrieval is unchanged from v4. The remaining hard cases
are mostly questions where the correct document is present but the model still
selects a nearby conflicting value or omits required details.

Experimental v6 lexical-anchor fusion:

```bash
make enterprise-rag-bench-embedding-rerank-fused-v6-lexical-existing-50
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v6-lexical-windowed-judge-50
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v6-lexical-windowed-judge-50
```

v6 adds the original top-50 keyword/source candidate list as a low-weight RRF
input (`weights=[1.0, 1.0, 0.75]`, `rrf_k=5`) alongside the top-50 and top-500
embedding rerank lists. The goal is to keep exact lexical/source hits from
falling out of the final top-10 when embedding rerank prefers semantically
similar but conflicting documents.

Latest v6 result with the local DeepSeek judge:

| Field | v5 evidence-selection | v6 lexical-anchor |
| --- | ---: | ---: |
| average correctness | `58.0%` | `56.0%` |
| average completeness | `63.62%` | `62.32%` |
| combined correctness * completeness | `36.90` | `34.90` |
| average document recall | `79.23%` | `81.89%` |
| average invalid extra documents | `8.98` | `8.91` |
| answer generation prompt tokens | `476,499` | `472,645` |
| answer generation completion tokens | `2,354` | `2,465` |
| answer generation total tokens | `478,853` | `475,110` |
| answer generation wall time | `59.70s` | `77.40s` |
| judge total tokens | `27,574` | `27,648` |
| retrieval-miss bucket | `10` | `9` |

v6 is not promoted as the current best answer gate. It proves that lexical
anchoring can improve document recall (`48/75 -> 51/75` gold docs found and
`7 -> 6` full retrieval misses), but the extra lexical evidence can also expose
nearby conflicting documents that reduce answer correctness. The next
retrieval-stage improvement should be selective rather than applying the
lexical anchor to every question type.

Experimental v7 selective lexical routing:

```bash
make enterprise-rag-bench-routed-v7-selective-lexical-judge-50
make enterprise-rag-bench-answer-error-analysis-routed-v7-selective-lexical-judge-50
```

v7 is a deterministic routing artifact over already generated and already
DeepSeek-judged rows. It does not call an LLM. The route uses v5 as the default
answer path and uses v6 lexical-anchor rows only for `basic`, `completeness`,
`conflicting_info`, `constrained`, and `project_related` question types.

Latest v7 routed result:

| Field | v5 evidence-selection | v6 lexical-anchor | v7 selective routing |
| --- | ---: | ---: | ---: |
| average correctness | `58.0%` | `56.0%` | `58.0%` |
| average completeness | `63.62%` | `62.32%` | `64.72%` |
| combined correctness * completeness | `36.90` | `34.90` | `37.54` |
| average document recall | `79.23%` | `81.89%` | `81.89%` |
| average invalid extra documents | `8.98` | `8.91` | `8.91` |
| route counts | n/a | n/a | `22 default / 28 routed` |

v7 is the current best local 50-question routed gate. v5 remains the best
single-generation path because v7 is a row-level policy that reuses prior v5
and v6 answer/judge artifacts. Treat v7 as routing evidence for the next real
generation experiment, not as a fresh model run.

Experimental v8 selective lexical fresh generation:

```bash
make enterprise-rag-bench-routed-v8-selective-lexical-retrieval-50
make enterprise-rag-bench-official-answer-metrics-routed-v8-selective-lexical-windowed-judge-50
make enterprise-rag-bench-answer-error-analysis-routed-v8-selective-lexical-windowed-judge-50
```

v8 turns the v7 routing evidence into a real answer-generation run. It routes
retrieval rows before prompting DeepSeek: v5 fused retrieval remains the default
context, and v6 lexical-anchor retrieval is used for `basic`, `completeness`,
`conflicting_info`, `constrained`, and `project_related` question types. The
answers are then freshly generated with the v5 evidence-selection prompt.

Latest v8 fresh-generation result:

| Field | v5 evidence-selection | v7 routed reuse | v8 routed fresh generation |
| --- | ---: | ---: | ---: |
| average correctness | `58.0%` | `58.0%` | `58.0%` |
| average completeness | `63.62%` | `64.72%` | `65.12%` |
| combined correctness * completeness | `36.90` | `37.54` | `37.77` |
| average document recall | `79.23%` | `81.89%` | `81.89%` |
| average invalid extra documents | `8.98` | `8.91` | `8.91` |
| answer generation total tokens | `478,853` | n/a | `476,796` |
| answer generation wall time | `59.70s` | n/a | `61.84s` |
| judge total tokens | `27,574` | n/a | `27,618` |

v8 is the current best local 50-question fresh-generation gate. The gain is
small but real under the same local DeepSeek judge: it keeps v7's retrieval
recall while improving answer completeness over v5 and v7. The remaining hard
cases are still selection failures when the correct document is present beside
similar conflicting evidence, plus `project_related` and `semantic` questions
with many required facts.

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

Latest v3 judge-backed failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `26` |
| `retrieval_miss` | `15` |
| `likely_judge_or_format_issue` | `5` |
| `abstained_with_evidence` | `4` |

Latest v4 DeepSeek-judged failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `29` |
| `retrieval_miss` | `10` |
| `likely_judge_or_format_issue` | `7` |
| `abstained_with_evidence` | `4` |

Latest v5 DeepSeek-judged failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `37` |
| `retrieval_miss` | `10` |
| `likely_judge_or_format_issue` | `3` |

Latest v6 DeepSeek-judged failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `37` |
| `retrieval_miss` | `9` |
| `likely_judge_or_format_issue` | `4` |

Latest v7 routed failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `37` |
| `retrieval_miss` | `9` |
| `likely_judge_or_format_issue` | `4` |

Latest v8 fresh-generation failure buckets:

| Bucket | Count |
| --- | ---: |
| `answer_missing_gold_facts` | `37` |
| `retrieval_miss` | `9` |
| `likely_judge_or_format_issue` | `4` |

## Full Run

The full retrieval target is:

```bash
make enterprise-rag-bench-cortexdb-retrieval-full
```

Do not start with the full run. Use the 50-question gate first, inspect document
recall, then decide whether to run full answer generation.

## Upstream Evaluator Environment

The upstream benchmark repository has its own Python dependencies and evaluator
contract. Keep that environment isolated from CortexDB:

```bash
make enterprise-rag-bench-official-env
```

This creates:

```text
target/enterprise-rag-bench/.venv/
```

The retrieval-only official metrics targets use that virtualenv automatically.
The current answer-quality targets use the local DeepSeek judge script instead
of the upstream benchmark evaluator.

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

- production-grade embedding retrieval;
- answer-aware reranking;
- exact fact selection when retrieved documents contain similar conflicting
  evidence;
- ContextPack-specific enterprise prompt tuning;
- leaderboard submission package.
