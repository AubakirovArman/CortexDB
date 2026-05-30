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
- `report.json`;
- `comparison.json` when a baseline exists;
- git commit SHA;
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
```

Long-term CI should upload these reports to persistent storage so recall and
latency drift can be graphed across commits.

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
- Public benchmark corpus conversion scripts are not yet included.
- Production SLO profiles per workload are not yet formalized.
