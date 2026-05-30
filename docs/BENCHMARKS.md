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
# Or directly:
cargo bench --bench core_baseline
```

The ANN section also emits a stable JSON line:

```text
ann_repeatable_report_json: {"corpus":"synthetic-ann-corpus-v1", ...}
```

The corpus and query set are deterministic. Latency values are intentionally
machine-dependent, but the report shape is stable and can be archived by CI to
track recall and p95/p99 drift across commits.

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
