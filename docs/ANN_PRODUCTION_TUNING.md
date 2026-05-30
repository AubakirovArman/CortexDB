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

Core Alpha currently targets L1 plus the tooling for L2/L3.

## Corpus Selection

Use at least three corpus classes before claiming production ANN behavior:

| Corpus | Purpose |
| --- | --- |
| Small smoke corpus | Fast CI validation of contract and report shape. |
| Domain corpus | Real CortexDB payload vectors from target workloads. |
| Public benchmark corpus | SIFT/GloVe-style reproducible ANN quality reference. |

For each corpus, preserve:

- vector generation version;
- metric (`dot_product`, `cosine`, or `l2`);
- dimension;
- vector count;
- query count;
- top-k limit;
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
| `layer_count` | More layers can reduce traversal cost but require graph-shape validation. |
| `max_visited_candidates` | SLO guard; too low causes visit-budget fallback. |
| `min_recall_q16` | Recall guard; below threshold should trigger exact fallback. |

Future construction knobs:

- `ef_construction`;
- collection-level metric metadata;
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

CI runs this packaging step on the stable toolchain and uploads the tarball as
the `ann-release-baseline-package` artifact. That artifact is the preferred
input for GitHub Releases because it carries both the benchmark report and the
checksum manifest in one file.

Candidate runs can be gated against a published bundle with:

```bash
make ann-compare-baseline-bundle \
  ANN_BASELINE_BUNDLE=target/ann/release-baselines/v0.1.0-core-alpha-smoke \
  ANN_CANDIDATE_RUN_ID=candidate-cosine-100k
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

- No archived large external corpus baseline in the repo.
- HNSW construction parameters are not yet collection-profile aware.
- Report history is not stored outside CI artifacts.
- Public benchmark conversion and one-command public-corpus runs are available,
  but no SIFT/GloVe-style golden report is checked in or attached to a release
  yet.
- Production SLO profiles per workload are not yet formalized.
