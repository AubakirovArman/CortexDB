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
| **`ann_repeatable_report_json`** | Deterministic synthetic ANN corpus | machine-specific | Emits JSON with recall, p50/p95/max latency, graph edges, and upper-layer counts |

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
results, or an overall runtime above the configured budget. It is not a
production stress test; it is a release smoke check that proves the actor-backed
HTTP surface remains usable under a small burst of real requests.

The ANN section also emits a stable JSON line:

```text
ann_repeatable_report_json: {"corpus":"synthetic-ann-corpus-v1", ...}
```

The corpus and query set are deterministic. Latency values are intentionally
machine-dependent, but the report shape is stable and can be archived by CI to
track recall and p95/p99 drift across commits.
External-corpus reports also embed their gate policy fields:
`required_min_recall_q16`, `required_min_mean_recall_q16`,
`allowed_p95_latency_nanos`, `allowed_max_latency_nanos`, and
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
vectors, queries, and ground-truth top-k:

```bash
cargo run --release -p cortex-engine --bin ann_corpus_check -- \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --ground-truth /data/ann/ground_truth.jsonl \
  --metric cosine \
  --output target/ann/large_corpus_report.json
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
make ann-real-embedding-preflight \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl

make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_SLO_PROFILE=balanced
```

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
