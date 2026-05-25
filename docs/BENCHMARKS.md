# CortexDB Core Alpha Benchmark Baseline

This document lists the baseline performance metrics of **CortexDB Core Alpha** (`v0.1.0-core-alpha`) for a standard workload of **1,000 cells**.

---

## 1. Baseline Performance Metrics

The following numbers were recorded on a standard Linux environment using the optimized Cargo bench profile:

| Benchmark Phase | Operations | Elapsed Time | Description |
| --- | --- | --- | --- |
| `put_1k_cells` | 1,000 | ~542 ms | Linear put of 1,000 cells (appending to active WAL, indexing in MemTable). |
| `get_1k_cells` | 1,000 | ~257 µs | Read of 1,000 cells from MemTable using MVCC isolation. |
| `checkpoint_1k` | 1 | ~27.7 ms | Serialization of 1,000 cells to disk segments (`.acs`, `.acb`, `.aci`, `.acv`), truncation of WAL. |
| `restart_replay_1k` | 1 | ~1.18 ms | Complete cold startup, scanning of segments, replaying of non-empty WAL. |
| `compact_1k` | 1 | ~13.2 ms | LSM compaction merging delta segments into a single consolidated snapshot segment. |
| `aql_retrieve_1k` | 1 | ~4.29 ms | Evaluation of full AQL select query with where/status filters and ranking. |
| `context_pack_1k` | 1 | ~4.57 ms | Full compilation of AQL retrieve output, token budget clamping, citation checks, and anomalies. |

---

## 2. How to Run Benchmarks

To reproduce these metrics on your local machine, run the following command from the repository root:

```bash
cargo bench --bench core_baseline
```

This will compile the database engine in release mode (`--profile bench`) and execute the sequential 1K cells workflow, outputting the exact timings of each phase.
