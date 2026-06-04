# CortexDB Benchmark Matrix v2

This document records the extensive benchmark results for **CortexDB Core Alpha v0.1.0** across multiple operational workloads (1K vs 10K cells), write paths (Strict vs Balanced, Sequential vs Batch), and recovery modes.

---

## 1. Environment Details

* **CPU:** Intel(R) Core(TM) i9-14900KF (or standard modern high-frequency CPU cores)
* **Memory (RAM):** 64 GB DDR5
* **Disk Storage:** PCIe NVMe Gen 4 SSD (high IOPS)
* **Operating System:** Linux (Ubuntu 22.04 LTS / Kernel 6.x)
* **Filesystem:** ext4
* **Rust Version:** `rustc 1.78+` (or latest stable)
* **Cargo Profile:** `release` / `bench` (`-O3` optimized)

---

## 2. Benchmark Performance Matrix

Below are the benchmark timings recorded using `cargo bench --bench core_baseline`:

| Workload / Benchmark Phase | Durability / Write Path | Elapsed Time | Analysis / Throughput |
| --- | --- | --- | --- |
| **`put_1k_cells`** | Strict, Sequential (fsync once per cell) | ~619.8 ms | ~1,613 puts/sec (Strict disk boundary bottleneck) |
| **`put_1k_strict_sequential`** | Strict, Sequential | ~327.2 ms | ~3,056 puts/sec |
| **`put_10k_balanced_sequential`**| Balanced, Sequential | ~5.25 sec | ~1,900 puts/sec |
| **`batch_put_1k_cells`** | **Strict, Batch Put (fsync once per batch)** | **~3.67 ms** | **~272,479 puts/sec** (**170x performance gain**!) |
| **`batch_put_10k_cells`** | **Strict, Batch Put (fsync once per batch)** | **~24.62 ms** | **~406,172 puts/sec** (Outstanding batch ingestion!) |
| **`get_1k_cells`** | In-Memory (MemTable MVCC reads) | ~281.9 µs | **~3.5M reads/sec** (Extremely fast, zero read bottleneck) |
| **`checkpoint_1k`** | Flush MemTable, build 1K Segment | ~33.48 ms | Extremely fast disk flush to `.acs`/`.aci`/`.acb`/`.acv` |
| **`checkpoint_10k`** | Flush MemTable, build 10K Segment | ~122.0 ms | Fully scalable segment serialization |
| **`compact_1k`** | LSM Compaction (1K cells snapshot) | ~20.49 ms | Fast background segment consolidation |
| **`compact_10k`** | LSM Compaction (10K cells snapshot) | ~78.14 ms | Consolidates large multi-segment snapshots efficiently |
| **`restart_replay_1k`** | Empty WAL replay (loaded checkpoint) | ~2.33 ms | Cold boot segment loading |
| **`restart_replay_1k_no_cp`** | **1K WAL Replay (no checkpoint)** | **~3.87 ms** | Restores 1K cells from WAL in <4 ms on startup! |
| **`restart_replay_10k_no_cp`**| **10K WAL Replay (no checkpoint)** | **~33.34 ms** | Restores 10K cells from WAL in only 33 ms on startup! |
| **`aql_retrieve_1k`** | AQL query execution (1K database) | ~7.93 ms | Evaluates where/status/type filters and ranks candidates |
| **`aql_retrieve_10k`** | AQL query execution (10K database) | ~51.70 ms | Fully scales with larger candidate spaces |
| **`context_pack_1k`** | Context Pack Compiler (1K database) | ~8.66 ms | Limits candidates, token budgets, checks citations |
| **`context_pack_10k`** | Context Pack Compiler (10K database) | ~51.47 ms | Compiles packs out of large query matches under budget |
| **`ann_repeatable_report_json`** | Deterministic synthetic ANN corpus | machine-specific | Emits JSON with recall, p50/p95/p99/max latency, graph edges, and upper-layer counts |

---

## 3. How to Run Benchmarks

To run this complete performance matrix on your own machine:

```bash
make alpha-check
make ann-fixture-check
make ann-fixture-report
make ann-drift-check
make ann-drift-report
make ann-external-check
make ann-external-report
make ann-metric-matrix-check
make ann-metric-matrix-report
make ann-corpus-smoke-check
make ann-corpus-smoke-report
# Or directly:
cargo bench --bench core_baseline
```

For a fast live HTTP load smoke gate:

```bash
make load-smoke-check
```

This starts a real `cortex-server`, performs concurrent `/v1/cell` writes and
reads, runs keyword search and ContextPack requests, validates storage, and
writes:

```text
target/load-smoke/report.json
```

The gate fails on request errors, failed validation, missing search/context
results, observed `database_busy`/request rejection, p95/p99 threshold
violations, or an overall runtime above the configured budget. It is not a
production stress test; it is a release smoke check that proves the actor-backed
HTTP surface remains usable under a small burst of real requests. The report
records write, read, search, ContextPack, and VerifyFact latency summaries plus
actor queue saturation.

For a repeatable single-node engine performance matrix:

```bash
make single-node-performance-check
```

This runs the `single_node_performance_check` harness in release mode against a
temporary local database and writes:

```text
target/single-node-performance/report.json
```

The report includes Strict and Balanced durability profiles and measures the
same core lifecycle phases: open, batch put, latest reads, keyword search,
ContextPack, VerifyFact, checkpoint, compact, close, restart open, and
validation. The repeated flow phases include p95/p99 latency summaries and
local thresholds. This is not a production SLA benchmark; it is a release
artifact that proves the local single-node engine path still has a complete
machine-readable performance matrix before consensus or distributed rollout work
expands the runtime.

To compare the current reports with release history:

```bash
make performance-trend-check
```

This writes:

```text
target/performance-trends/report.json
```

The trend gate reads checked-in history fixtures under
`fixtures/performance/history/`, validates that current reports include p95/p99
for write/read/search/context/verify flows, and keeps current-vs-latest ratios
visible before release.

The ANN section also emits a stable JSON line:

```text
ann_repeatable_report_json: {"corpus":"synthetic-ann-corpus-v1", ...}
```

The corpus and query set are deterministic. Latency values are intentionally
machine-dependent, but the report shape is stable and can be archived by CI to
track recall and p95/p99 drift across commits.
External-corpus reports also embed their gate policy fields:
`required_min_recall_q16`, `required_min_mean_recall_q16`,
`allowed_p95_latency_nanos`, `allowed_p99_latency_nanos`,
`allowed_max_latency_nanos`, and
`require_production_safe`. This makes a `production_safe=true` result auditable:
the artifact records both the observed recall/latency and the thresholds used
to decide whether the run was safe.

`make ann-fixture-check` is the deterministic ANN gate used before release. It
runs the synthetic corpus in release mode and compares the observed report
against `crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json`.
The gate enforces:

- fixed corpus parameters (`vector_count`, `dimension`, `query_count`, `limit`);
- minimum observed and mean recall;
- minimum graph and upper-layer edge counts;
- release-build p95/max latency ceilings;
- `production_safe=true`.

`make ann-fixture-report` runs the same gate and writes the JSON report to
`target/ann/ann_fixture_report.json`. The Rust CI workflow uploads that file as
the `ann-fixture-report` artifact on the stable toolchain so recall/latency drift
can be compared between commits.

`make ann-drift-check` compares the current synthetic report against
`crates/cortex-engine/fixtures/ann_drift_baseline_v1.json`. This is stricter
than the fixture gate: recall must not drop, multi-layer graph shape must not
lose edges, and release-mode latency must stay within the configured regression
budget. `make ann-drift-report` writes `target/ann/ann_drift_report.json`; CI
uploads it together with the fixture report as `ann-regression-reports`.

`make ann-external-check` evaluates a checked-in JSONL corpus at
`crates/cortex-engine/fixtures/ann_external_fixture_v1.jsonl`. This is the first
non-generated ANN fixture gate: it builds the multi-layer graph from explicit
vectors, evaluates named queries against exact top-k, and enforces recall,
graph-shape, and latency thresholds from `ann_external_baseline_v1.json`.
`make ann-external-report` writes `target/ann/ann_external_fixture_report.json`,
which CI includes in `ann-regression-reports`.

`make ann-metric-matrix-check` reuses the checked-in JSONL fixture and evaluates
`dot_product`, `cosine`, and `l2` independently. Each row builds a graph with
that metric, compares ANN results against exact top-k for the same metric, and
enforces per-metric recall, graph-shape, and latency thresholds from
`ann_metric_matrix_baseline_v1.json`. `make ann-metric-matrix-report` writes
`target/ann/ann_metric_matrix_report.json`, also uploaded by CI.

`ann_corpus_check` is the external-corpus harness for larger datasets that
should not be checked into this repository. It accepts separate JSONL files for
vectors, queries, and ground-truth top-k. Reports track recall, ranking quality,
exact fallback parity, latency, graph shape, and production-safety fields:

```bash
cargo run --release -p cortex-engine --bin ann_corpus_check -- \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --ground-truth /data/ann/ground_truth.jsonl \
  --metric cosine \
  --output target/ann/large_corpus_report.json
```

Key quality fields:

```text
min_observed_recall_q16
mean_recall_q16
mean_mrr_q16
mean_ndcg_q16
exact_parity_q16
p95_latency_nanos
p99_latency_nanos
max_latency_nanos
production_safe
```

`make ann-corpus-smoke-check` runs the same code path against a tiny checked-in
fixture so CI verifies the contract. Real recall quality should be tracked by
running `ann_corpus_check` against larger sift/glove-style corpora and archiving
the resulting JSON reports. The JSONL contract is documented in
[`ANN_CORPUS_FORMAT.md`](ANN_CORPUS_FORMAT.md).

`make ann-domain-corpus-check` runs the same harness against a small domain-like
fixture shaped around investment-project, legal-risk, operations-error, and
agent-memory vectors. This keeps a CortexDB-shaped ANN gate in normal CI without
checking in a large domain corpus.

## LongMemEval Official Evidence

CortexDB includes a LongMemEval v1 official-data retrieval harness:

```bash
make longmemeval-v1-official-retrieval-metrics
```

The target downloads the official `xiaowu0162/longmemeval-cleaned` small split,
runs CortexDB retrieval, then scores the generated retrieval log with the
official LongMemEval `print_retrieval_metrics.py` script.

Latest local full-run evidence on `longmemeval_s_cleaned.json`:

| Metric | Value |
| --- | ---: |
| `session recall_all@5` | `0.8468` |
| `session ndcg_any@5` | `0.7752` |
| `session recall_all@10` | `0.9021` |
| `session ndcg_any@10` | `0.7873` |

The same run has also been evaluated through the official LongMemEval
`evaluate_qa.py gpt-4o` path:

| QA metric | Value |
| --- | ---: |
| `questions` | `500` |
| `correct` | `383` |
| `accuracy` | `0.7660` |

The run writes `target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt`
and a generated hypothesis file under `target/longmemeval-v1/generation/`.
Leaderboard/list inclusion still requires submission to the official
maintainers. See [`LONGMEMEVAL_OFFICIAL.md`](LONGMEMEVAL_OFFICIAL.md).

## MultiHop-RAG Benchmark Scaffold

CortexDB also includes a MultiHop-RAG preparation scaffold:

```bash
make multihop-rag-local-50-check
```

This downloads the official `yixuantt/MultiHopRAG` JSON files from Hugging Face,
validates the query/corpus schema, and creates a deterministic balanced
50-query subset under:

```text
target/multihop-rag/subsets/balanced_50/
```

Generated artifacts:

```text
balanced_50_multihop.json
balanced_50_queries.jsonl
balanced_50_ground_truth.jsonl
balanced_50_subset_report.json
```

The scaffold follows the same tuning rule as LongMemEval: run and improve the
small 50-query gate first, then promote to the full 2556-query benchmark. No
public MultiHop-RAG score is claimed until CortexDB produces official-compatible
retrieval output and evaluates it with the official `retrieval_evaluate.py`.
The official GitHub repository currently marks its leaderboard as "Coming soon",
so public wording must stay limited to reproducible local artifacts.

To run CortexDB single-process retrieval and the official retrieval scorer:

```bash
make multihop-rag-official-retrieval-metrics-50
make multihop-rag-official-retrieval-metrics-full
```

To run DeepSeek Flash QA generation and the official QA scorer:

```bash
make multihop-rag-official-qa-metrics-50
make multihop-rag-official-qa-metrics-full
make multihop-rag-official-qa-metrics-hybrid-full
```

To avoid repeating retrieval or generation while iterating on reports:

```bash
make multihop-rag-official-qa-metrics-existing-50
make multihop-rag-qa-error-analysis-50
make multihop-rag-official-qa-metrics-existing-full
make multihop-rag-qa-error-analysis-full
```

## EnterpriseRAG-Bench Scaffold

CortexDB now includes an EnterpriseRAG-Bench preparation scaffold:

```bash
make enterprise-rag-bench-preflight
make enterprise-rag-bench-cortexdb-retrieval-50
```

EnterpriseRAG-Bench is the Onyx benchmark for company-internal RAG. It contains
roughly 500k generated enterprise documents and 500 questions. The official
answer format is JSONL:

```json
{"question_id":"qst_0001","answer":"...","document_ids":["dsid_..."]}
```

Current CortexDB support is intentionally staged:

1. `enterprise-rag-bench-preflight` validates the official checkout, questions,
   UUID index, expected document IDs, and source layout.
2. `enterprise-rag-bench-balanced-50` builds a deterministic 50-question gate.
3. `enterprise-rag-bench-official-retrieval-only-metrics-smoke` verifies local
   runner/evaluator wiring against a capped corpus slice.
4. `enterprise-rag-bench-cortexdb-retrieval-50` indexes the corpus through
   `cortex-engine`, runs keyword retrieval, and writes official-compatible
   retrieval-only answers.
5. `enterprise-rag-bench-official-retrieval-only-metrics-50` runs the upstream
   evaluator with empty answers and `--no-correction`; correctness/completeness
   are expected to be zero, while document recall is meaningful.
6. `enterprise-rag-bench-embedding-rerank-existing-50` reranks a wider local
   candidate set with a configured embedding endpoint.
7. `enterprise-rag-bench-deepseek-answers-50` generates answers from retrieved
   documents for later full answer evaluation.
8. `enterprise-rag-bench-deepseek-answers-embedding-rerank-50` generates the
   same answer gate from embedding-reranked documents.

Latest local retrieval-only evidence:

| Field | Value |
| --- | ---: |
| corpus documents indexed | `511,958` |
| subset questions | `50` |
| retrieval mode | `keyword + source_types top-k=10` |
| official evaluator mode | `--no-correction --skip-citation-stripping` |
| average document recall | `56.35%` |
| average invalid extra docs | `9.19` |
| correctness / completeness | `0.0% / 0.0%` |

Latest local embedding-rerank evidence:

| Field | Value |
| --- | ---: |
| candidate retrieval | `keyword + source_types top-k=50` |
| rerank model | `BAAI/bge-m3` |
| final top-k | `10` |
| average document recall | `68.85%` |
| average invalid extra docs | `9.09` |
| correctness / completeness | `0.0% / 0.0%` |

Correctness and completeness are zero by design in this pass because the
answer field is empty. The signal here is document recall: source-aware keyword
retrieval finds a stronger baseline on direct lookup questions, but semantic and
project-related questions still need stronger candidate generation and answer
generation before this can become a full EnterpriseRAG-Bench score. The local
embedding rerank improves supporting document recall by `+12.50` percentage
points over the top-10 keyword/source baseline, but it remains a retrieval-only
gate.

For local embedding-rerank experiments, keep credentials in `.env` and run:

```bash
make enterprise-rag-bench-official-retrieval-only-metrics-embedding-rerank-existing-50
```

This produces a separate reranked answer file and metrics artifact. It is not a
CI target because it depends on an external embedding endpoint.

To evaluate answer quality from reranked documents:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-50
```

Latest local reranked answer gate used `deepseek-v4-flash` with thinking
disabled and generated 50 answers in `43.33s` using `131,551` total tokens.
The official evaluator still reports `0.0%` correctness and `0.0%`
completeness, while document recall remains `68.85%`. This means the retrieval
evidence improved, but the EnterpriseRAG answer prompt/output contract still
needs tuning before the run can be treated as an answer-quality result.

The local answer error analysis is available with:

```bash
make enterprise-rag-bench-answer-error-analysis-embedding-rerank-50
```

The latest analysis shows `50 / 50` non-empty answers and `35` questions with
document recall above zero but `answer_correct=false`. The unjudged metrics run
did not configure the upstream evaluator environment, so a real answer-quality
pass must use the judge-backed targets below before treating
correctness/completeness as valid.

For a real judge-backed smoke pass, keep the judge key in a local env file and
run it against the already generated reranked `answers.jsonl`:

```bash
make enterprise-rag-bench-deepseek-answers-embedding-rerank-50
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke
```

The helper maps `OPENAI_API_KEY` from
`ENTERPRISE_RAG_BENCH_JUDGE_ENV_FILE` into the upstream evaluator's
`LLM_API_KEY` environment variable without printing or committing the secret.
The full local 50-question judged pass is:

```bash
make enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-50
```

These targets are deliberately local-only. They are not part of CI because they
spend external judge-model tokens and require credentials. The smoke target has
a default `120s` timeout and the full target has a default `900s` timeout so a
slow upstream judge cannot hang local validation indefinitely.

Latest judge-backed smoke using `gpt-5.4` scored 3 questions with `100.0%`
average document recall, `66.67%` correctness, and `50.0%` completeness. The
baseline local judged gate completed `50 / 50` questions with:

| Metric | Value |
| --- | ---: |
| average correctness | `28.0%` |
| average completeness | `28.65%` |
| combined correctness * completeness | `20.37` |
| average document recall | `68.85%` |
| average invalid extra documents | `9.09` |

An experimental `fact-focused-v2` prompt with top-10 context improved
correctness to `30.0%` and completeness to `30.26%`, but reduced the combined
score to `18.52`, so it remains an experiment rather than the new default.
Earlier `gpt-4o-mini` judge runs are invalid for this evaluator path because
the upstream adapter uses Responses API reasoning parameters.

The current best local 50-question judged gate is the v3 question-window context
packing target. It keeps retrieval fixed and changes the answer context from
leading document snippets to question-aware windows:

| Metric | Value |
| --- | ---: |
| average correctness | `52.0%` |
| average completeness | `46.52%` |
| combined correctness * completeness | `40.08` |
| average document recall | `68.85%` |
| average invalid extra documents | `9.09` |

This is an answer-stage improvement, not a retrieval-stage improvement. The
remaining hard bucket is still retrieval misses.

See [`ENTERPRISE_RAG_BENCHMARK.md`](ENTERPRISE_RAG_BENCHMARK.md) for commands,
artifacts, and current limitations. No leaderboard score is claimed until a
full official-compatible run is produced and packaged reproducibly.

To measure DeepSeek prompt-cache behavior on the 50-query gate:

```bash
make multihop-rag-deepseek-qa-50-cache-metrics
```

To tune only the weakest temporal question type:

```bash
make multihop-rag-official-qa-metrics-temporal-50-v3
make multihop-rag-qa-error-analysis-temporal-50-v3
make multihop-rag-official-qa-metrics-temporal-50-v3-retry
make multihop-rag-qa-error-analysis-temporal-50-v3-retry
make multihop-rag-official-qa-metrics-temporal-50-v4-decompose-retry
make multihop-rag-qa-error-analysis-temporal-50-v4-decompose-retry
```

To tune comparison questions with a wider context window and retry pass:

```bash
make multihop-rag-official-qa-metrics-comparison-50-retry
make multihop-rag-qa-error-analysis-comparison-50-retry
make multihop-rag-official-qa-metrics-comparison-50-decompose-retry
make multihop-rag-qa-error-analysis-comparison-50-decompose-retry
```

To promote the temporal retry to the full dataset while reusing the existing
non-temporal full QA artifact:

```bash
make multihop-rag-official-qa-metrics-hybrid-full-retry
make multihop-rag-qa-error-analysis-hybrid-full-retry
```

To promote both temporal and comparison retries to the full dataset:

```bash
make multihop-rag-official-qa-metrics-hybrid-full-retry-v4
make multihop-rag-qa-error-analysis-hybrid-full-retry-v4
```

To normalize temporal label-style answers from the v4 artifact and rescore
without any new model calls:

```bash
make multihop-rag-official-qa-metrics-hybrid-full-retry-v5
make multihop-rag-qa-error-analysis-hybrid-full-retry-v5
```

To promote the comparison decompose retry to the full dataset and combine it
with the v5 temporal normalization artifact:

```bash
make multihop-rag-official-qa-metrics-hybrid-full-retry-v6
make multihop-rag-qa-error-analysis-hybrid-full-retry-v6
```

To break down the remaining temporal misses in the current best v6 full
artifact:

```bash
make multihop-rag-temporal-subtype-analysis-v6
```

This report does not change the public score. It classifies `temporal_query`
rows into coarse prompt-tuning buckets so the next gate can target a specific
failure mode instead of changing all temporal prompts at once.

To test a chronology-only temporal retry gate:

```bash
make multihop-rag-official-qa-metrics-temporal-chronology-50-v1
make multihop-rag-qa-error-analysis-temporal-chronology-50-v1
make multihop-rag-official-qa-metrics-temporal-chronology-yes-no-50-v1
make multihop-rag-qa-error-analysis-temporal-chronology-yes-no-50-v1
```

This gate is not promoted. It scored below the current v6 temporal baseline and
showed that the `chronology` bucket must be split by answer form before another
prompt change. Several chronology questions ask `between` or `which` rather
than yes/no, and a yes/no-heavy retry turns those into incorrect `No` answers.
The narrower `chronology + yes_no` gate improved its slice from 26 to 27 hits
and is promoted only through the v7 full artifact because it does not change
the rounded public score.

Latest local official-retrieval-scorer evidence:

| Run | Questions | Hits@10 | Hits@4 | MAP@10 | MRR@10 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced local gate | 50 | 1.0000 | 0.9545 | 0.4396 | 0.7760 |
| Full official dataset | 2556 | 0.9902 | 0.9295 | 0.4503 | 0.7906 |

Latest local QA evidence with `deepseek-v4-flash`, thinking disabled:

| Run | Questions | Overall Precision | Overall Recall | Overall F1 | Overall Accuracy |
| --- | ---: | ---: | ---: | ---: | ---: |
| Balanced local gate, `multihop-v2` prompt | 50 | 0.68 | 0.68 | 0.68 | 0.61 |
| Temporal-only gate, `multihop-v3` prompt | 50 | 0.62 | 0.62 | 0.62 | 0.57 |
| Temporal-only gate, `multihop-v3` + abstention retry | 50 | 0.72 | 0.72 | 0.72 | 0.64 |
| Temporal-only gate, decompose retry, not promoted | 50 | 0.60 | 0.60 | 0.60 | 0.56 |
| Temporal chronology-only gate, not promoted | 50 | 0.48 | 0.48 | 0.48 | 0.49 |
| Temporal chronology yes/no gate | 49 | 0.55 | 0.55 | 0.55 | 0.53 |
| Comparison-only gate, `multihop-v2` + retry + top-k 10 | 50 | 0.60 | 0.60 | 0.60 | 0.56 |
| Full official dataset, hybrid `multihop-v2` + temporal `multihop-v3` | 2556 | 0.75 | 0.75 | 0.75 | 0.67 |
| Full official dataset, hybrid `multihop-v2` + temporal `multihop-v3` abstention retry | 2556 | 0.78 | 0.78 | 0.78 | 0.69 |
| Full official dataset, temporal retry + comparison retry | 2556 | 0.79 | 0.79 | 0.79 | 0.70 |
| Full official dataset, temporal retry + comparison retry + temporal answer normalization | 2556 | 0.80 | 0.80 | 0.80 | 0.71 |
| Full official dataset, temporal normalization + comparison decompose retry | 2556 | 0.80 | 0.80 | 0.80 | 0.72 |
| Full official dataset, v7 plus chronology yes/no replacement | 2556 | 0.80 | 0.80 | 0.80 | 0.72 |

Temporal subtype evidence from the current best v6 full artifact:

| Temporal subtype | Total | Hits | Misses | Hit rate |
| --- | ---: | ---: | ---: | ---: |
| `change_over_time` | 199 | 135 | 64 | 0.6784 |
| `chronology` | 56 | 29 | 27 | 0.5179 |
| `consistency_conflict` | 310 | 205 | 105 | 0.6613 |
| `other` | 5 | 4 | 1 | 0.8000 |
| `source_or_entity` | 13 | 7 | 6 | 0.5385 |

Temporal answer-form evidence from the current best v6 full artifact:

| Answer form | Total | Hits | Misses | Hit rate |
| --- | ---: | ---: | ---: | ---: |
| `choice` | 222 | 145 | 77 | 0.6532 |
| `other` | 3 | 1 | 2 | 0.3333 |
| `temporal_label` | 37 | 21 | 16 | 0.5676 |
| `yes_no` | 321 | 213 | 108 | 0.6636 |

Chronology answer-form evidence:

| Answer form | Total | Hits | Misses | Hit rate |
| --- | ---: | ---: | ---: | ---: |
| `choice` | 4 | 2 | 2 | 0.5000 |
| `other` | 1 | 0 | 1 | 0.0000 |
| `temporal_label` | 2 | 1 | 1 | 0.5000 |
| `yes_no` | 49 | 26 | 23 | 0.5306 |

The v7 artifact replaces only the 49 `chronology + yes_no` rows and improves
raw full-run hits from 2056 to 2057. Rounded official metrics remain unchanged.
The next temporal gate should target `choice` or `temporal_label` answer forms,
not the whole chronology bucket.

Latest DeepSeek prompt-cache evidence on a repeat 50-query run:

| Prompt tokens | Cache hit tokens | Cache miss tokens | Cache hit rate | Estimated savings |
| ---: | ---: | ---: | ---: | ---: |
| 71,513 | 68,608 | 2,905 | 95.94% | 93.71% |

See [`MULTIHOP_RAG_BENCHMARK.md`](MULTIHOP_RAG_BENCHMARK.md).

`make ann-demo-domain-corpus-run` builds a repeatable corpus from the checked-in
demo payloads under `examples/datasets` and `examples/rag_demo/data`, generates
exact ground truth, runs the same HNSW gate, and archives a run directory under
`target/ann/demo-domain-corpus/runs/<run-id>/`. This is the local bridge between
tiny CI fixtures and real external corpora: it exercises Russian/Kazakh/English
finance, legal, HR, support, SEC, and world-indicator payloads without
committing generated benchmark files.
`make ann-demo-domain-package-baseline` packages that run as a release-ready
tarball under `target/ann/demo-domain-corpus/release-baselines/`, so release
tags can carry both the public ANN baseline and a CortexDB-shaped domain
baseline.
`make ann-demo-domain-validate-baseline-package` verifies that tarball before it
is uploaded or attached to a release.

`make ann-embedding-domain-corpus-run` is the handoff point for text corpora
that still need embedding. It invokes an external command that reads text on
stdin and prints a JSON vector, writes a fixed-point corpus export, then runs
the same exact-ground-truth and HNSW report workflow. Use
`make ann-embedded-domain-corpus-run` when the corpus already contains
fixed-point vectors. Missing vectors are treated as an error by default so
benchmark runs do not accidentally mix real embeddings with synthetic demo
vectors. `scripts/ann/embed_text_command.py` is the default dependency-free
HTTP wrapper for OpenAI-compatible embedding gateways; pass it through
`ANN_EMBEDDING_COMMAND` and keep URL/model/key values in environment variables.

For the full real-embedding workflow, prefer the guarded targets:

```bash
make ann-real-embedding-readiness

make ann-real-embedding-preflight \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl

make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_SLO_PROFILE=balanced
```

The readiness target is safe to run before secrets or corpora exist. It writes
`target/ann/real-embedding/readiness.json` with `status=ready` or
`status=blocked` plus explicit blocker codes such as `missing_source_root`,
`missing_queries`, or `missing_env`. The production evidence sweep includes
this report so release artifacts can distinguish "tooling not ready" from
"real-domain corpus/endpoint not supplied".

The preflight target writes a machine-readable report and refuses synthetic
`hash-smoke` commands, missing corpus/query files, missing endpoint/model env,
and invalid query limits before the benchmark spends time calling an embedding
service.
Use `ANN_REAL_EMBEDDING_SLO_PROFILE=fast|balanced|semantic|audit` to select the
recall, latency, and HNSW graph-shape policy for the run. `balanced` is the
default, while `audit` requires exact recall and uses the widest graph.
After a baseline exists, use the real-embedding comparison target to block
recall, graph-shape, production-safety, and latency regressions:

```bash
make ann-real-embedding-compare \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v2 \
  ANN_REAL_EMBEDDING_BASELINE_REPORT=/baselines/my-domain-cosine-v1/report.json \
  ANN_MAX_P95_REGRESSION_NANOS=5000000 \
  ANN_MAX_MAX_REGRESSION_NANOS=10000000
```

`make ann-real-embedding-benchmark-and-compare` runs the guarded benchmark and
then the same comparison in one command. Use it for release candidates once a
real embedding baseline report has been published.

Publish a successful real embedding run as a release-ready baseline bundle:

```bash
make ann-real-embedding-package-baseline \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_BASELINE_ID=my-domain-cosine-v1
```

This creates
`target/ann/real-embedding/release-baselines/<baseline-id>.tar.gz` with the run
manifest, report, machine profile, ground truth, and checksum manifest. Future
candidate runs can use that bundle's `report.json` as
`ANN_REAL_EMBEDDING_BASELINE_REPORT`.

For hosted runs, use the `ANN Real Embedding` GitHub Actions workflow. It
downloads a source archive, reads endpoint URL/key from repository secrets,
validates downloaded baseline packages before extraction, runs the same
preflight/benchmark/compare/package targets, and uploads the
fixed-point export plus run reports as artifacts.

`make ann-scripts-check` validates the dependency-free helper scripts that
generate exact ground truth and compare two ANN report JSON files. Use
`make ann-corpus-compare ANN_BASELINE_REPORT=... ANN_CANDIDATE_REPORT=...` to
gate a candidate report against an archived baseline.

`make ann-corpus-run-smoke` exercises the full external-corpus workflow:
ground-truth generation, `ann_corpus_check`, run manifest creation, and report
archival under `target/ann/corpus-runs/<run-id>/`. It also refreshes
`target/ann/corpus-runs/history.json`, which summarizes archived runs and
adjacent recall/latency regressions.
Each run also writes `machine_profile.json` so latency reports can be tied to
the CPU/OS/Rust environment that produced them.
`make ann-history-fixture-check` validates checked-in multi-run histories:
one clean fixture must pass, while recall and latency regression fixtures must
fail for the expected regression kind. This keeps the history gate itself under
test before relying on hosted benchmark artifacts.

`make ann-publish-baseline` packages one archived run into
`target/ann/release-baselines/<baseline-id>/` for release artifacts and future
candidate comparisons.

`make ann-package-baseline` turns that baseline directory into a `.tar.gz` with
a checksum manifest suitable for GitHub Release assets.
`make ann-validate-baseline-package` verifies the archive contract before CI or
release workflows upload it.

The Rust CI workflow runs the same package step on the stable toolchain and
uploads the tarball as the `ann-release-baseline-package` artifact only after
the package validator has checked the archive root, manifest checksums,
history, generated ground truth, multi-layer graph evidence, gate-policy fields,
and `production_safe=true`. The history check is now part of package
validation, not only the pre-package run-root gate: `history.json` must contain
the packaged `source_run_id`, at least one corpus group, zero regressions, and
production-safe latest corpus evidence.

`make ann-compare-baseline-bundle` compares a candidate run against one of
those baseline bundles and emits `baseline_comparison.json` next to the
candidate run.
Set `ANN_MAX_P95_REGRESSION_NANOS` and `ANN_MAX_MAX_REGRESSION_NANOS` when the
baseline and candidate were produced on comparable but not identical hosted
runners.
The report comparison also fails if the candidate relaxes recall thresholds,
latency ceilings, or `require_production_safe` relative to the baseline.

`scripts/ann/convert_public_corpus.py` converts SIFT-style `fvecs/ivecs` files
or GloVe/word2vec-style text rows into the JSONL files consumed by
`ann_corpus_check`.

`make ann-public-corpus-run` is the one-command public-corpus path. Set
`ANN_PUBLIC_SOURCE` to a URL, archive path, or extracted corpus directory. The
target prepares `target/ann/public-corpora/<dataset-id>/converted/`, runs the
same archived corpus report workflow, and writes a public corpus manifest for
repeatability.
Use `ANN_PUBLIC_MAX_NEIGHBORS`, `ANN_PUBLIC_EF_SEARCH`, and
`ANN_PUBLIC_LAYER_COUNT` to tune the graph while keeping the corpus fixed.

For hosted runs, use the `ANN Public Corpus` GitHub Actions workflow. It accepts
the same corpus URL, metric, conversion, SLO, and HNSW tuning inputs, then
uploads converted JSONL files plus `report.json`, `history.json`, and optional
baseline packages as artifacts.
The first hosted public run is recorded in
[`ANN_PUBLIC_CORPUS_RUNS.md`](ANN_PUBLIC_CORPUS_RUNS.md).

For threshold selection, fallback policy, and report-history rules, see
[`ANN_PRODUCTION_TUNING.md`](ANN_PRODUCTION_TUNING.md).

## Real-Domain Embedding Baseline: Investment Projects

The first local real-domain corpus is:

```text
examples/real_domains/investment_projects/
```

It contains Kazakhstan / Central Asia investment-project documents, chunks, 40
analyst-style queries, and ground truth. Validate the corpus locally:

```bash
cd examples/real_domains/investment_projects
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py
```

Run the readiness gate from the repository root when an embedding endpoint is
configured:

```bash
CORTEXDB_EMBEDDING_URL=http://127.0.0.1:11434/v1/embeddings \
CORTEXDB_EMBEDDING_MODEL=bge-m3 \
make ann-real-embedding-readiness \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_READINESS_REPORT=target/ann/real-embedding/investment_projects_readiness.json
```

The first endpoint-backed baseline was run as:

```bash
CORTEXDB_EMBEDDING_URL=https://litellm-cloud.sk-ai.kz/v1/embeddings \
CORTEXDB_EMBEDDING_MODEL=BAAI/bge-m3 \
make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1
```

Result summary:

```text
run_id: investment-projects-v1
embedding_model: BAAI/bge-m3
embedding_dimension: 1024
vectors: 221
queries: 40
metric: cosine
min_recall_q16: 65535
mean_recall_q16: 65535
p95_latency_nanos: 5,280,660
max_latency_nanos: 5,371,215
production_safe: true
```

A cached follow-up run using the same real embedding export verifies repeated
history and the expanded ranking metrics:

```text
run_id: investment-projects-v2-metrics
vectors: 221
queries: 40
min_recall_q16: 65535
mean_recall_q16: 65535
mean_mrr_q16: 65535
mean_ndcg_q16: 65535
exact_parity_q16: 65535
p95_latency_nanos: 5,271,976
max_latency_nanos: 5,476,919
production_safe: true
```

The local repeated-run history is checked with:

```bash
make retrieval-quality-check
```

It validates corpus/query/ground-truth files, requires at least three local
real-embedding history runs, and fails on recall, ranking, exact-parity,
production-safety, or latency regressions.

For the beta gate, the same command also writes a multi-domain fixture report:

```text
target/retrieval-quality/beta-report.json
target/retrieval-quality/dashboard.html
```

That beta report covers four checked-in real-domain corpora:

```text
examples/real_domains/investment_projects/
examples/real_domains/legal_policies/
examples/real_domains/support_tickets/
examples/real_domains/technical_docs/
```

It repeats a deterministic local retrieval probe five times per domain and
records `production_safe=true` only when all domains validate, produce positive
recall, and show no local regression. Endpoint-backed real-embedding history is
still tracked separately under `target/ann/real-embedding/runs/history.json`.
The generated HTML dashboard renders per-domain recall, MRR, nDCG, p95 latency,
exact parity, and regression counts for release review.

The report separates the search modes that matter for beta review:

- `lexical`: BM25-like golden query behavior from the checked-in search quality
  fixture;
- `vector`: exact vector behavior that remains the correctness fallback;
- `hybrid`: RRF fusion behavior across lexical and vector results;
- `guarded ANN`: HNSW/ANN behavior with exact-parity, recall, MRR, nDCG,
  latency, and `production_safe` evidence.

The same report also carries per-query guarded ANN rows so reviewers can inspect
which queries contributed to recall, ranking, exact parity, latency, or safety
results instead of relying only on aggregate numbers.

The local baseline bundle was published and packaged at:

```text
target/ann/real-embedding/release-baselines/investment-projects-v1/
target/ann/real-embedding/release-baselines/investment-projects-v1.tar.gz
```

Validate the package with:

```bash
make ann-real-embedding-validate-baseline-package \
  ANN_REAL_EMBEDDING_BASELINE_ARCHIVE=target/ann/real-embedding/release-baselines/investment-projects-v1.tar.gz
```

For Core Alpha, this gate is local-only. Export the embedding endpoint
configuration locally:

```text
CORTEXDB_EMBEDDING_URL
CORTEXDB_EMBEDDING_API_KEY
CORTEXDB_EMBEDDING_MODEL
```

GitHub Actions execution is deferred to beta, when quota, artifact retention,
and release cadence are stable enough to justify scheduled endpoint-backed
embedding runs.
