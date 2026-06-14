# CortexDB Scale Benchmarks

This page records database-scale benchmark runs that are larger than the
historical 1K/10K microbenchmarks. The goal is to keep capacity claims honest:
publish the expensive phases, not only the fast paths.

## Runner

The scale runner creates an isolated local database under `target/scale-bench`.
It does not reuse EnterpriseRAG benchmark artifacts and does not touch any
existing user database.

```bash
make scale-bench-100k
make scale-bench-1m
make scale-bench-10m-lazy
```

Default targets run a safe core profile:

```text
put batches -> memory after put -> checkpoint -> memory after checkpoint
-> sampled get_latest -> close -> restart open
```

Heavy phases are opt-in because the current 100K broad search/context/verify
paths expose unresolved bottlenecks and can take a long time:

```bash
make scale-bench-100k SCALE_BENCH_SEARCH_SAMPLES=1
make scale-bench-100k SCALE_BENCH_CONTEXT_SAMPLES=1
make scale-bench-100k SCALE_BENCH_VERIFY_SAMPLES=1
```

The runner writes machine-readable JSON reports:

```text
target/scale-bench/100k/report.json
target/scale-bench/1m/report.json
target/scale-bench/10m-lazy/report.json
```

`make scale-bench-10m-lazy` is a controlled post-A08 lazy-residency packet. It
uses a direct prepared checkpoint with fixed 128-byte payloads, disables the
benchmark-only lazy payload-derived index rebuild, and skips storage estimates
and full validation so the 10M run measures open RSS, sampled read latency, and
restart latency without triggering a full AQL-index rebuild from `storage_stats`.
The 100K/1M targets remain the strict validation targets.

To inventory already captured A19 evidence without running a new benchmark:

```bash
python3 scripts/scale_benchmark_inventory.py \
  --root target/scale-bench \
  --report target/scale-bench/inventory.json
```

This writes:

```text
target/scale-bench/inventory.json
```

To build current scale trend curves from existing reports without running a new
benchmark:

```bash
make scale-bench-trends
```

This writes:

```text
target/scale-bench/trends.json
target/scale-bench/trends.md
```

## Payload Profile

The generated corpus uses realistic text payloads between roughly 0.5KB and
4KB, with mixed scopes and operational terms:

```text
scope=scale
scope=scale:team-a
scope=scale:team-b
scope=scale:archive
```

This profile is intentionally heavier than tiny synthetic payloads.

## 100K Core Baseline

Run date: 2026-06-12

Source state: A19 scale-benchmark runner commit `94fbf92`.

Command:

```bash
make scale-bench-100k
```

Report:

```text
target/scale-bench/100k/report.json
```

Summary:

| Phase | Result |
| --- | ---: |
| cells | 100000 |
| total duration | 71185.262 ms |
| put_batches | 960.891 ms |
| checkpoint | 38416.460 ms |
| get_latest p95 | 0.003 ms |
| restart_open | 219.172 ms |
| after_put RSS | 872755200 bytes |
| after_checkpoint RSS | 890494976 bytes |
| peak RSS | 1123278848 bytes |
| estimated total memory | 894553484 bytes |

Validation status: `ok=true`, no storage validation errors.

## 1M Core Baseline

Run date: 2026-06-12

Source state: A19 scale-benchmark runner commit `94fbf92`.

Command:

```bash
make scale-bench-1m
```

Report:

```text
target/scale-bench/1m/report.json
```

Summary:

| Phase | Result |
| --- | ---: |
| cells | 1000000 |
| total duration | 704416.326 ms |
| put_batches | 10892.097 ms |
| checkpoint | 378042.066 ms |
| get_latest p95 | 1.165 ms |
| restart_open | 2879.535 ms |
| after_put RSS | 8606552064 bytes |
| after_checkpoint RSS | 8748335104 bytes |
| peak RSS | 11147628544 bytes |
| estimated total memory | 8946879838 bytes |

Validation status: `ok=true`, no storage validation errors.

## Current Interpretation

The core single-node write/read/restart path is reproducible at 100K and 1M
cells with strict validation. Controlled direct-checkpoint runs publish the
current 100K/1M search and VerifyFact bottlenecks instead of hiding them behind
the default target.

The 10M lazy packet is intentionally narrower: it proves the post-A08 lazy-open
RSS/read/restart shape after disabling benchmark-only eager payload-derived
index rebuilds. It does not claim full 10M broad search, ContextPack, VerifyFact,
storage-estimate, or validation coverage.

## A19 10M Lazy RSS/Latency Packet

Run date: 2026-06-14

This run is the post-A08 10M lazy-residency packet. It is a controlled direct
checkpoint with fixed 128-byte payloads and 20 prepared segments. The benchmark
skips storage estimates and full validation because those paths intentionally
force full index/segment scans and are covered by the smaller strict targets.

Command:

```bash
make scale-bench-10m-lazy
```

Report:

```text
target/scale-bench/10m-lazy/report.json
```

Summary:

| Phase | Result |
| --- | ---: |
| cells | 10000000 |
| total duration | 372088.640 ms |
| direct_checkpoint | 317386.510 ms |
| open_prepared | 26570.692 ms |
| after_open_prepared RSS | 24504950784 bytes |
| after_open_prepared peak RSS | 24742612992 bytes |
| get_latest p50/p95 | 117.489 / 120.598 ms |
| close | 3218.116 ms |
| restart_open | 19500.937 ms |

Validation status: `validation_skipped=true`,
`storage_estimates_skipped=true`, `lazy_payload_index_rebuild=false`.

## A19 Optimization History Labels

The trend generator reads `fixtures/scale_bench/optimization_history.json` and
publishes the required before/after labels for A05, A06, A08, and A09 in
`target/scale-bench/trends.md`.

| Epic | After label |
| --- | --- |
| A05 | Indexed candidate path; 1M fixed-payload direct run `verify_fact.p95_ms=32531.547`. |
| A06 | Query-adjacent scans=0; 1M direct indexed ContextPack `p95_ms=11633.309`. |
| A08 | Lazy residency; 100K after-open RSS `818954240` bytes and ContextPack `p95_ms=1113.399`. |
| A09 | Cached persisted-index append delta merge with structural no-full-remerge proof. |

## A19 Evidence Inventory

Run date: 2026-06-14

Command:

```bash
python3 scripts/scale_benchmark_inventory.py \
  --root target/scale-bench \
  --report target/scale-bench/inventory.json
```

Report:

```text
target/scale-bench/inventory.json
```

Summary:

| Cells | Core lifecycle | ContextPack p50/p95 | Keyword p50/p95 | Verify p50/p95 |
| ---: | --- | --- | --- | --- |
| 1K | yes | no | yes | yes |
| 10K | yes | yes | no | no |
| 100K | yes | yes | yes | yes |
| 1M | yes | yes | yes | yes |
| 10M | yes | no | no | no |

Status: `complete`, 19 reports found, no missing A19 acceptance items.

## A19 Current Scale Trend Report

Run date: 2026-06-14

Command:

```bash
make scale-bench-trends
```

Reports:

```text
target/scale-bench/trends.json
target/scale-bench/trends.md
```

Summary:

| Item | Result |
| --- | ---: |
| trend status | complete |
| multi-point curves | 38 |
| git revision | recorded in `target/scale-bench/trends.json` |

The current trend report has the 10M lazy point and before/after optimization
labels for A05, A06, A08, and A09 from
`fixtures/scale_bench/optimization_history.json`.

## A19 100K Search/Verify Instrumentation

Run date: 2026-06-13

This run is a controlled instrumentation run, not the full 100K lifecycle
baseline. It uses `--direct-checkpoint` and fixed 128-byte payloads so search
and VerifyFact latency can be measured without rerunning the expensive WAL
ingest path.

Command:

```bash
cargo run --release -p cortex-engine --bin scale_benchmark_check -- \
  --root target/scale-bench/a19-search-verify-100k \
  --report target/scale-bench/a19-search-verify-100k/report.json \
  --cells 100000 \
  --samples 0 \
  --search-samples 3 \
  --context-samples 0 \
  --verify-samples 3 \
  --batch-size 5000 \
  --payload-bytes 128 \
  --direct-checkpoint
```

Report:

```text
target/scale-bench/a19-search-verify-100k/report.json
```

Summary:

| Phase | Result |
| --- | ---: |
| cells | 100000 |
| total duration | 26613.438 ms |
| direct_checkpoint | 2539.002 ms |
| open_prepared | 1984.402 ms |
| keyword_search p50/p95 | 1307.151 / 1307.151 ms |
| VerifyFact p50/p95 | 4077.174 / 4077.174 ms |
| restart_open | 1749.477 ms |

Validation status: `ok=true`, no storage validation errors.

## A19 1M Search/Verify Instrumentation

Run date: 2026-06-13

This run uses the same controlled profile as the 100K search/verify
instrumentation run: direct prepared checkpoint, fixed 128-byte payloads, and
one search/verify sample. It is intended to close the 1M metric-shape gap and
surface the current bottleneck, not to claim optimized latency.

Command:

```bash
cargo run --release -p cortex-engine --bin scale_benchmark_check -- \
  --root target/scale-bench/a19-search-verify-1m \
  --report target/scale-bench/a19-search-verify-1m/report.json \
  --cells 1000000 \
  --samples 0 \
  --search-samples 1 \
  --context-samples 0 \
  --verify-samples 1 \
  --batch-size 50000 \
  --payload-bytes 128 \
  --direct-checkpoint
```

Report:

```text
target/scale-bench/a19-search-verify-1m/report.json
```

Summary:

| Phase | Result |
| --- | ---: |
| cells | 1000000 |
| total duration | 276196.626 ms |
| direct_checkpoint | 28375.848 ms |
| open_prepared | 20446.025 ms |
| after_open_prepared RSS | 12299739136 bytes |
| after_open_prepared estimated total memory | 4901708052 bytes |
| keyword_search p50/p95 | 133779.048 / 133779.048 ms |
| VerifyFact p50/p95 | 32531.547 / 32531.547 ms |
| restart_open | 17520.731 ms |

Validation status: `ok=true`, no storage validation errors.
