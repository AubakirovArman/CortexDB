# ANN Production Tuning

This document defines how CortexDB should tune and gate HNSW/ANN behavior after
Core Alpha. It complements [`ANN_CORPUS_FORMAT.md`](ANN_CORPUS_FORMAT.md), which
defines the external corpus file contract.

The goal is not to make ANN the default for every workload. The goal is to know
when ANN is safe enough to use, when exact scan should remain the fallback, and
which regressions must block a release.

## Why This Exists

Small in-repo fixtures catch contract and obvious recall regressions. They do
not prove production quality. Production tuning needs:

- a representative external corpus;
- exact top-k ground truth for each metric;
- repeatable recall and latency reports;
- repeated recall probes for the same profile/corpus;
- explicit thresholds;
- archived report history across commits;
- a fallback policy when ANN violates a guard.

Without this loop, HNSW tuning becomes anecdotal. With it, every change to graph
construction or search behavior has an objective before/after report.

## Readiness Levels

| Level | Meaning | Required Evidence |
| --- | --- | --- |
| L0 | Contract smoke | `make ann-corpus-run-smoke` passes. |
| L1 | Local fixture gate | synthetic, drift, external, and metric-matrix gates pass. |
| L2 | External corpus baseline | `ann_corpus_check` report archived for a representative corpus. |
| L3 | Regression gate | candidate reports are compared against archived baseline reports. |
| L4 | Production tuning | corpus suite covers size, metric, tenant/domain, and latency classes. |

Core Alpha currently targets L1 plus the tooling for L2/L3. The
`ann-recall-probe-report` target repeats the local domain corpus gate and
records recall, p99 latency, and graph-shape stability as local no-fallback
prerequisite evidence.
The public `siftsmall` full-run baseline attached to the Core Alpha release is
the first L2 evidence package; the hosted workflow can now compare candidate
runs against that published bundle.

## Corpus Selection

Use at least three corpus classes before claiming production ANN behavior:

| Corpus | Purpose |
| --- | --- |
| Small smoke corpus | Fast CI validation of contract and report shape. |
| Domain corpus | Real CortexDB payload vectors from target workloads. |
| Public benchmark corpus | SIFT/GloVe-style reproducible ANN quality reference. |

The checked-in `ann_domain_*_v1.jsonl` fixture is the first lightweight domain
gate. It is not a replacement for a real customer/domain corpus, but it keeps
agent-memory and context-shaped vector regressions visible in every normal CI
run.

`make ann-demo-domain-corpus-run` adds a larger local product-shaped gate by
building vectors from the checked-in example datasets and RAG demo payloads. It
is still deterministic and dependency-free, but it covers more realistic
finance, legal, HR, support, SEC, and world-indicator text than the tiny
checked-in fixture.
Release tags package this run as a separate demo-domain baseline archive, so a
tag carries public benchmark evidence and CortexDB-shaped domain evidence.

For real embedding model output, use `make ann-embedding-domain-corpus-run`
when the source corpus only has text. It calls an external embedding command,
exports fixed-point vectors, and then runs the normal embedded-domain gate. Use
`make ann-embedded-domain-corpus-run` when payload rows and query rows already
contain fixed-point vectors. Both paths fail closed on missing vectors, which
keeps production tuning honest: a real embedding baseline should never silently
fall back to hashed demo vectors.

Production reports should record the embedding command identity and model
version in the run notes or release artifact. The built-in `hash-smoke`
provider is only a plumbing check; it is not evidence for semantic recall.
For OpenAI-compatible or local gateway endpoints, the checked-in
`scripts/ann/embed_text_command.py` can be used as the command wrapper. It reads
endpoint/model/key settings from environment variables and prints only the
numeric vector to stdout, which keeps provider secrets out of committed corpus
artifacts.

Before running an expensive real-embedding benchmark, run the preflight gate:

```bash
export CORTEXDB_EMBEDDING_URL='https://embedding-gateway.example/v1/embeddings'
export CORTEXDB_EMBEDDING_MODEL='text-embedding-model'
export CORTEXDB_EMBEDDING_API_KEY='...'

make ann-real-embedding-preflight \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_REQUIRE_API_KEY=true
```

The preflight validates JSONL source/query shape, required environment
variables, metric settings, and that the configured command is not a synthetic
`hash-smoke` path. It writes
`target/ann/real-embedding/preflight.json` by default. Once that passes, run:

```bash
make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_REQUIRE_API_KEY=true \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1
```

This target uses `ANN_REAL_EMBEDDING_COMMAND`, which defaults to
`python3 scripts/ann/embed_text_command.py --require-model`, then archives the
normal ANN report and machine profile under
`target/ann/real-embedding/runs/<run-id>/`.

Real embedding runs accept named SLO profiles through
`ANN_REAL_EMBEDDING_SLO_PROFILE`:

| Profile | Intended Use | Recall Policy | Latency Policy | HNSW Shape |
| --- | --- | --- | --- | --- |
| `fast` | low-latency interactive search | 75 percent min/mean recall | strict p95/max budget | small graph |
| `balanced` | default context retrieval | 75 percent min/mean recall | default p95/max budget | balanced graph |
| `semantic` | higher-quality semantic retrieval | higher min/mean recall | wider latency budget | wider graph |
| `audit` | release/audit verification runs | exact top-k recall required | widest latency budget | largest graph |

History gates keep recall, graph shape, HNSW config, and `production_safe`
strict. Real embedding latency is noisier because it depends on the local host
and embedding endpoint path, so `ann-real-embedding-history-regression-check`
uses explicit latency SLO budgets:

```text
ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS=1000000
ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS=5000000
```

Set these to `0` for a strict lab run. Increase them only when the release note
documents the machine/endpoint reason.

## Local Real-Domain Corpus

The first checked-in real-domain corpus lives at:

```text
examples/real_domains/investment_projects/
```

It covers Kazakhstan / Central Asia investment-project retrieval using public
MDB project metadata and short generated benchmark notes. The corpus includes:

- `corpus/documents.jsonl`
- `corpus/chunks.jsonl`
- `queries/queries.jsonl`
- `queries/ground_truth.jsonl`
- source registry and validators

Validate it from the corpus directory:

```bash
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py
```

Then run the readiness gate from the repository root:

```bash
CORTEXDB_EMBEDDING_URL=http://127.0.0.1:11434/v1/embeddings \
CORTEXDB_EMBEDDING_MODEL=bge-m3 \
make ann-real-embedding-readiness \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_READINESS_REPORT=target/ann/real-embedding/investment_projects_readiness.json
```

The first endpoint-backed run for this corpus is `investment-projects-v1` with
`BAAI/bge-m3` embeddings:

```text
vectors: 221
queries: 40
dimension: 1024
metric: cosine
min_recall_q16: 65535
mean_recall_q16: 65535
p95_latency_nanos: 5,280,660
production_safe: true
```

The cached follow-up run `investment-projects-v2-metrics` reuses the same
real-embedding export and records ranking/parity metrics for history tracking:

```text
mean_recall_q16: 65535
mean_mrr_q16: 65535
mean_ndcg_q16: 65535
exact_parity_q16: 65535
p95_latency_nanos: 5,271,976
production_safe: true
```

The package validation command is:

```bash
make ann-real-embedding-validate-baseline-package \
  ANN_REAL_EMBEDDING_BASELINE_ARCHIVE=target/ann/real-embedding/release-baselines/investment-projects-v1.tar.gz
```

This closes the local corpus/query/ground-truth and first endpoint-backed
baseline side of real-domain promotion. The current local history gate for this
run root passes with the explicit p95/max latency budgets above:

```bash
make ann-real-embedding-history-regression-check \
  ANN_REAL_EMBEDDING_RUN_ROOT=target/ann/real-embedding/runs \
  ANN_REAL_EMBEDDING_HISTORY_REPORT=target/ann/real-embedding/runs/history.json
```

The latest local history has three runs, `regression_count=0`,
`latest_mean_recall_q16=65535`, `latest_mean_mrr_q16=65535`,
`latest_mean_ndcg_q16=65535`, `latest_exact_parity_q16=65535`, and
`latest_production_safe=true`. Before beta, repeat this on stable
infrastructure and publish the selected baseline package.

## Local Real-Domain Gate

Real-domain embedding benchmarks are intentionally local-only until beta. This
avoids spending embedding quota on every repository run and keeps provider keys
out of GitHub Actions while the corpus and SLOs are still being tuned.

Local environment:

```text
CORTEXDB_EMBEDDING_URL
CORTEXDB_EMBEDDING_API_KEY
CORTEXDB_EMBEDDING_MODEL
```

Local gate:

```bash
make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1
```

GitHub Actions promotion for this gate is deferred to beta. At that point the
workflow should be reintroduced with repository secrets, artifact retention, and
explicit budget controls.

Inspect a profile without running a benchmark:

```bash
make ann-slo-profile ANN_REAL_EMBEDDING_SLO_PROFILE=semantic
```

The database checkpoint path also has profile-aware HNSW construction knobs via
`DatabaseOptions::hnsw_build_config`. This matters because `.ach` graph density
is durable: checkpoint and compact write the graph that persisted ANN search
will later serve. The built-in Rust profiles mirror the benchmark SLO shapes:

| Profile | `max_neighbors` | `ef_search` | `layer_count` |
| --- | ---: | ---: | ---: |
| `Fast` | 8 | 64 | 3 |
| `Balanced` | 16 | 128 | 4 |
| `Semantic` | 24 | 192 | 5 |
| `Audit` | 32 | 256 | 5 |

Use a narrower profile for low-latency interactive indexes and a wider profile
for semantic or audit-heavy collections. Changing the profile affects newly
written checkpoint/compact segments; existing `.ach` files should be rebuilt by
compaction if the collection policy changes.
`Database::validate_storage()` rejects mixed live-segment HNSW profiles, so a
profile migration should compact the collection before the new graph shape is
used for production ANN.

Once a real embedding baseline has been published, candidate runs should use a
report comparison gate:

```bash
make ann-real-embedding-benchmark-and-compare \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_REQUIRE_API_KEY=true \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v2 \
  ANN_REAL_EMBEDDING_SLO_PROFILE=semantic \
  ANN_REAL_EMBEDDING_BASELINE_REPORT=/baselines/my-domain-cosine-v1/report.json \
  ANN_MAX_P95_REGRESSION_NANOS=5000000 \
  ANN_MAX_MAX_REGRESSION_NANOS=10000000
```

This fails on recall regression, corpus/profile mismatch, `production_safe`
loss, HNSW parameter drift, or latency regression beyond the configured budget.
Use `make ann-real-embedding-compare` when the candidate report already exists
and only the comparison needs to be rerun.

Publish a passing real embedding run as a release-ready baseline package:

```bash
make ann-real-embedding-package-baseline \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_BASELINE_ID=my-domain-cosine-v1
```

This writes a baseline directory and `.tar.gz` under
`target/ann/real-embedding/release-baselines/`. The archive includes
`package_manifest.json` with SHA-256 checksums, so it can be attached to a
release and reused as the comparison source for future candidate runs.
Validate the archive before publishing it:

```bash
make ann-real-embedding-validate-baseline-package \
  ANN_REAL_EMBEDDING_BASELINE_ARCHIVE=target/ann/real-embedding/release-baselines/my-domain-cosine-v1.tar.gz
```

For a single local release-evidence command after the corpus paths and embedding
environment are configured, use:

```bash
make ann-real-embedding-release-check \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=/data/cortexdb/text-cells \
  ANN_REAL_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_REAL_EMBEDDING_REQUIRE_API_KEY=true \
  ANN_REAL_EMBEDDING_RUN_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_BASELINE_ID=my-domain-cosine-v1 \
  ANN_REAL_EMBEDDING_SLO_PROFILE=semantic
```

This runs the real-embedding benchmark, optionally compares against
`ANN_REAL_EMBEDDING_BASELINE_REPORT` when it is set, gates history, packages the
baseline, and validates the package with production-safe, history,
ground-truth, and real-embedding metadata requirements.

The validator opens the tarball without extracting it, rejects unsafe archive
paths and links, checks every manifest-listed file size and SHA-256 digest, and
requires `report.json` to be passing and `production_safe=true`. It also
requires the report to carry the gate policy that produced the result:
`required_min_recall_q16`, `required_min_mean_recall_q16`,
`allowed_p95_latency_nanos`, `allowed_p99_latency_nanos`,
`allowed_max_latency_nanos`, and
`require_production_safe=true`. The validator replays those thresholds against
the observed recall/latency fields and rejects single-layer graph evidence
(`hnsw_layer_count <= 1`, `upper_layers == 0`, or `upper_graph_edges == 0`).
Real embedding release packages also require `history.json` and generated
ground truth. The packaged `history.json` must be clean as a standalone
artifact: at least one run, at least one corpus group, zero adjacent
regressions, a matching `source_run_id`, and production-safe latest corpus
evidence. This means a published baseline carries quality evidence, replayable
correctness evidence, the SLO contract used to judge the run, and the history
needed to compare later candidates.

The history gate has checked-in fixtures under
`crates/cortex-engine/fixtures/ann_history_*_v1.json`. Run
`make ann-history-fixture-check` to prove the clean multi-run history passes,
the recall-regression history is rejected for recall loss, and the
latency-regression history is rejected for latency drift. This protects the
release gate from silently accepting broken history summaries.

For each corpus, preserve:

- vector generation version;
- metric (`dot_product`, `cosine`, or `l2`);
- dimension;
- vector count;
- query count;
- top-k limit;
- recall and latency gate policy;
- HNSW graph shape (`max_neighbors`, `ef_search`, `layer_count`);
- machine profile;
- commit SHA;
- exact ground-truth generation command.

## Threshold Policy

Recall uses Q16:

| Q16 | Approximate Recall |
| --- | --- |
| `65535` | 100 percent |
| `62258` | 95 percent |
| `58981` | 90 percent |
| `49151` | 75 percent |

Recommended gates:

| Stage | Min Query Recall | Min Mean Recall | Latency Gate |
| --- | --- | --- | --- |
| Smoke | `49151` | `49151` | CI-friendly loose budget |
| Pre-alpha external | `58981` | `62258` | p95 budget per machine |
| Release candidate | `62258` | `64224` | p95 and max regression budget |
| Critical audit mode | exact scan preferred | exact scan preferred | ANN optional only |

Do not silently lower thresholds to make a run pass. If a corpus exposes low
recall, keep the failing report and tune graph/search parameters.

## Tuning Knobs

Current engine knobs:

| Knob | Effect |
| --- | --- |
| `max_neighbors` | Higher values increase graph density and memory, often improving recall. |
| `ef_search` | Higher values visit more candidates, improving recall at latency cost. |
| `ef_construction` | Higher values expand the build-time neighbor candidate beam, improving graph quality at checkpoint/compact cost. |
| `layer_count` | More layers can reduce traversal cost but require graph-shape validation. |
| `max_visited_candidates` | SLO guard; too low causes visit-budget fallback. |
| `min_recall_q16` | Recall guard; below threshold should trigger exact fallback. |

Future construction knobs:

- corpus-size-specific graph presets;
- background rebuild budget;
- per-tenant SLO profile.

## Tuning Loop

1. Prepare `vectors.jsonl` and `queries.jsonl`.
2. Generate exact ground truth:

```bash
python3 scripts/ann/exact_ground_truth.py \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --metric cosine \
  --output /data/ann/ground_truth.jsonl
```

3. Run a baseline:

```bash
scripts/ann/run_external_corpus.sh \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --ground-truth /data/ann/ground_truth.jsonl \
  --metric cosine \
  --max-neighbors 16 \
  --ef-search 256 \
  --run-id baseline-cosine-100k
```

4. Archive `manifest.json`, `report.json`, and machine profile.
5. Tune one parameter at a time.
6. Compare against baseline:

```bash
scripts/ann/run_external_corpus.sh \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --ground-truth /data/ann/ground_truth.jsonl \
  --metric cosine \
  --baseline-report target/ann/corpus-runs/baseline-cosine-100k/report.json \
  --run-id candidate-cosine-100k
```

7. Reject candidate runs with recall loss unless the change intentionally shifts
   the quality/latency tradeoff and the docs record that decision.

## GitHub Actions Public Corpus Runs

Use the `ANN Public Corpus` workflow when the corpus is too large for normal CI
but should still be evaluated in a reproducible hosted run. Trigger it from
GitHub Actions with:

- `source_url`: public archive URL for a SIFT/GloVe-style corpus;
- `dataset_id`: stable output id;
- `corpus_format`: `fvecs` or `text`;
- `metric`, `normalization`, and `scale` matching the corpus ground truth;
- HNSW knobs: `max_neighbors`, `ef_search`, `layer_count`;
- SLO gates: `min_recall_q16`, `min_mean_recall_q16`,
  `max_p95_latency_nanos`, `max_p99_latency_nanos`,
  `max_max_latency_nanos`.

For a quick shakedown, set `max_vectors` and `max_queries`. For a release-grade
baseline, leave those unset and enable `publish_baseline`; the workflow will
upload the converted JSONL files, run report, history, and optional baseline
tarball as an Actions artifact. Hosted public runs that should be referenced
later are recorded in [`ANN_PUBLIC_CORPUS_RUNS.md`](ANN_PUBLIC_CORPUS_RUNS.md).

To turn a hosted run into a regression gate, pass `baseline_bundle_url` pointing
to a previously published baseline `.tar.gz`. The workflow downloads the
bundle, validates the archive contract before extraction, compares the new
`report.json` against the baseline report, and writes `baseline_comparison.json`
next to the candidate run. It fails on recall regression, corpus-shape changes,
HNSW profile changes, production-safety loss, or latency regression beyond
`max_p95_regression_nanos`, `max_p99_regression_nanos`, and
`max_max_regression_nanos`.

## GitHub Actions Real Embedding Runs

Use the `ANN Real Embedding` workflow when the benchmark should call a real
embedding gateway from a hosted runner. Store endpoint credentials as repository
secrets, not workflow inputs:

- `CORTEXDB_EMBEDDING_URL`
- `CORTEXDB_EMBEDDING_API_KEY`

Trigger the workflow with:

- `source_archive_url`: `.tar`, `.tar.gz`, or `.zip` containing source JSONL
  files and query JSONL;
- `source_archive_sha256`: expected SHA-256 for the downloaded source archive;
  it is required whenever `publish_baseline=true`;
- `source_root_in_archive`: directory inside the archive containing payload
  JSONL files;
- `queries_path_in_archive`: query JSONL path inside the archive;
- `embedding_model`: model id sent to the configured endpoint;
- `slo_profile`: `fast`, `balanced`, `semantic`, or `audit`;
- optional `baseline_bundle_url` for regression gating;
- optional `publish_baseline=true` to upload a release-ready baseline package.

The workflow runs the same local targets:

```text
ann-real-embedding-preflight
ann-real-embedding-benchmark
ann-real-embedding-release-check   # local all-in-one release evidence gate
ann-real-embedding-history-regression-check
ann-real-embedding-compare          # when baseline_bundle_url is set
ann-real-embedding-package-baseline # when publish_baseline is true
ann-real-embedding-validate-baseline-package
```

Artifacts include the preflight report, exported fixed-point corpus, ANN run
directory, explicit `history.json`, and optional validated baseline package.
The real-embedding baseline package requires `embedding_preflight.json` and
`embedding_export_manifest.json`; hosted published baselines also carry a
`source_archive_manifest.json` with the downloaded archive SHA-256. This is the
preferred path for production-style evidence because the run is reproducible
from a GitHub Actions URL, records model/provider provenance, and does not
expose embedding credentials in command lines.

## Fallback Policy

Exact fallback is a correctness boundary, not an implementation detail.

ANN should fall back to exact scan when:

- graph is empty;
- graph integrity fails;
- graph returns insufficient candidates;
- visit budget is exceeded;
- observed recall is below `min_recall_q16`;
- persisted snapshot is unavailable or stale.

If `require_slo=true`, fallback should be visible in reports as not
`production_safe`. For critical audit workloads, exact scan remains the safest
default until external corpus evidence proves ANN behavior for that domain.

## Report History

Every external run should keep:

- `manifest.json`;
- `machine_profile.json`;
- `report.json`;
- `comparison.json` when a baseline exists;
- `history.json` at the corpus-run root;
- git commit SHA;
- HNSW parameters (`max_neighbors`, `ef_search`, `layer_count`);
- CPU, memory, disk, OS, Rust version;
- command line used to generate ground truth and run the check.

Recommended artifact layout:

```text
target/ann/corpus-runs/
  baseline-cosine-100k/
    manifest.json
    ground_truth.jsonl
    report.json
  candidate-cosine-100k/
    manifest.json
    report.json
    comparison.json
  history.json
```

`run_external_corpus.sh` refreshes `history.json` after successful runs. The
history summary groups runs by corpus key and records adjacent recall, latency,
graph-shape, and production-safety regressions. Long-term CI should upload
these reports to persistent storage so recall and latency drift can be graphed
across commits.

The Rust GitHub Actions workflow uploads checked-in ANN reports and
`target/ann/corpus-runs/**` as the `ann-regression-reports` artifact on the
stable toolchain. This keeps `history.json`, per-run manifests, reports, and
generated smoke ground truth available even when a regression causes the ANN
report step to fail.

The real-embedding workflow runs `ann-real-embedding-history-regression-check`
after each benchmark and uploads `target/ann/real-embedding/runs/**`, including
the explicit real-embedding `history.json`. Use the same target locally when a
real embedding run directory already exists and you only need to regenerate or
gate the history summary. The regression-check targets fail closed when the run
root has no archived run or corpus group, so an empty directory cannot count as
production latency evidence.

For local release hygiene, `make ann-scripts-check` also runs
`history_fixture_check.py --self-test` and `make ann-history-fixture-check`.
That means script self-tests, generated ground truth checks, report contracts,
package contracts, and history-fixture contracts all fail before release
evidence can be packaged.

For release checkpoints, publish the selected run into
`target/ann/release-baselines/<baseline-id>/`:

```bash
make ann-publish-baseline \
  ANN_BASELINE_RUN_ID=smoke \
  ANN_BASELINE_ID=v0.1.0-core-alpha-smoke
```

That bundle includes `baseline_manifest.json`, the selected report, the run
manifest, `history.json`, and generated ground truth when available. GitHub
Actions uploads `target/ann/release-baselines/**` with the same
`ann-regression-reports` artifact, so releases can attach a stable baseline
package without checking large corpus files into git.

For GitHub Releases, package the selected bundle as a tarball:

```bash
make ann-package-baseline \
  ANN_BASELINE_ID=v0.1.0-core-alpha-smoke \
  ANN_BASELINE_ARCHIVE=target/ann/release-baselines/v0.1.0-core-alpha-smoke.tar.gz
```

The tarball includes `package_manifest.json` with SHA-256 checksums for each
included report, manifest, machine profile, and ground-truth file.

Validate the release archive before attaching it:

```bash
make ann-validate-baseline-package \
  ANN_BASELINE_ARCHIVE=target/ann/release-baselines/v0.1.0-core-alpha-smoke.tar.gz
```

This gate enforces the package contract: one safe archive root, no links, a
matching `package_manifest.json`, exact file sizes, exact SHA-256 checksums,
`history.json`, generated ground truth, and a passing production-safe report.

CI runs this packaging step on the stable toolchain and uploads the tarball as
the `ann-release-baseline-package` artifact. That artifact is the preferred
input for GitHub Releases because it carries both the benchmark report and the
checksum manifest in one file.
The Rust, public-corpus, real-embedding, and Release workflows all run the
package validator before uploading or attaching baseline archives.

For local release readiness, use the aggregate gate:

```bash
make ann-release-evidence-check
```

It writes under `target/ann/release-evidence/` rather than the normal
developer `target/ann/corpus-runs/` directory. That keeps release evidence
isolated from old local experiments, so a stale latency run cannot make a
candidate fail or pass for the wrong reason. The target runs the smoke corpus,
fails on report-history regressions, publishes the baseline directory, packages
the archive, validates the archive contract, then does the same
package/validation pass for the deterministic demo-domain corpus. `make
release-check` depends on this target, so release candidates cannot skip the
ANN baseline package validation step.

When a `v*` tag is pushed, the `Release` workflow builds the same package and
uploads it directly to the GitHub Release. That makes the ANN baseline durable:
release consumers can download the exact recall/latency evidence used for the
tag even after normal CI artifacts expire.

Candidate runs can be gated against a published bundle with:

```bash
make ann-compare-baseline-bundle \
  ANN_BASELINE_BUNDLE=target/ann/release-baselines/v0.1.0-core-alpha-smoke \
  ANN_CANDIDATE_RUN_ID=candidate-cosine-100k \
  ANN_MAX_P95_REGRESSION_NANOS=5000000 \
  ANN_MAX_MAX_REGRESSION_NANOS=10000000
```

This writes `baseline_comparison.json` next to the candidate run and fails on
recall regression, corpus-shape mismatch, production-safety loss, or latency
regression beyond the configured budgets.

## Release Blockers

Block a release when any guarded corpus shows:

- `passed=false`;
- `production_safe=false`;
- min or mean recall regression;
- p95/max latency regression beyond budget;
- graph-shape collapse (`graph_nodes`, `graph_edges`, upper-layer edges);
- corpus-shape mismatch between baseline and candidate reports.

Warnings, not blockers:

- small latency variation inside the configured budget;
- improved recall with expected latency increase, if documented;
- smoke-corpus-only changes that do not affect external corpora.

## Current Gaps

- Full hosted `siftsmall` public-corpus baseline is published as a release
  asset and candidate hosted runs can be gated against that baseline bundle.
- HNSW construction profiles are available for newly written checkpoint/compact
  graphs. The manifest also stores the intended profile independently of `.ach`,
  and validation rejects live graphs that drift from that manifest policy.
- Vector collection metadata is stored in the manifest as `vector_profile`
  (`dimension`, `metric`) and validation rejects live `.acv` / `.ach` bundles
  that drift from that collection profile.
- `ef_construction` is now part of HNSW build profiles, `.ach` graph metadata,
  manifest `hnsw_profile`, and ANN corpus/run manifests, so baseline sweeps can
  compare build-time graph quality knobs explicitly.
- Report history is not stored outside CI artifacts and release baseline
  bundles.
- Demo-domain corpus generation is available, but no large real customer/domain
  baseline is published yet.
- Embedded-vector and real-embedding corpus tooling exists, including preflight,
  history gating, report comparison, provenance capture, source-archive
  integrity checks, and baseline packaging gates, but it still needs a real
  model export and published baseline bundle.
- Workload SLO profiles exist, but their thresholds still need calibration on
  real customer/domain corpora.
