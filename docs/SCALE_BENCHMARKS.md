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
```

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
cells. The runs also confirm that realistic payloads produce high RSS and a
costly checkpoint phase before lazy payload work lands.

Search, ContextPack, and VerifyFact at 100K are deliberately kept outside the
default target. A low-sample exploratory run reached those phases but did not
finish in a practical window, so those measurements remain open work for the
next optimization pass rather than hidden behind a passing default.

## A19 Evidence Inventory

Run date: 2026-06-13

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
| 1K | yes | no | no | no |
| 10K | yes | yes | no | no |
| 100K | yes | yes | no | no |
| 1M | yes | yes | no | no |

Open A19 items from the inventory:

- 100K keyword search p50/p95.
- 100K VerifyFact p50/p95.
- 1M keyword search p50/p95.
- 1M VerifyFact p50/p95.
- 10M post-lazy RSS/latency report.
- Multi-point before/after optimization trend curves.
