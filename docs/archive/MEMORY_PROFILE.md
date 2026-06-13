# CortexDB Memory Profile

This document describes the portable memory profiling gate used for database-grade
work until allocator-specific profilers are explicitly enabled.

## Command

```bash
make memory-profile
```

Useful overrides:

```bash
make memory-profile \
  MEMORY_PROFILE_ROOT=target/memory-profile/10k \
  MEMORY_PROFILE_REPORT=target/memory-profile/10k/report.json \
  MEMORY_PROFILE_CELLS=10000 \
  MEMORY_PROFILE_BATCH_SIZE=5000 \
  MEMORY_PROFILE_PAYLOAD_BYTES=0 \
  MEMORY_PROFILE_PAYLOAD_RESIDENCY=memory
```

Use `MEMORY_PROFILE_PAYLOAD_RESIDENCY=lazy` to measure checkpoint reopen with
segment-backed payloads left on disk.

Use `MEMORY_PROFILE_PAYLOAD_BYTES=N` to pad every generated payload to at least
`N` bytes. Use `MEMORY_PROFILE_REOPEN_ONLY=--reopen-only` to measure a fresh
process opening an existing `MEMORY_PROFILE_ROOT/db` without rebuilding the
database first. Fresh-process reopen is the correct RSS comparison for lazy
payload residency because allocator memory from the ingestion/checkpoint phase
can otherwise dominate the process RSS.

Use `MEMORY_PROFILE_BATCH_SIZE=N` to control ingestion memory. Large runs such
as 1M cells should use bounded batches instead of constructing a single
`Vec<(CellId, payload)>` for the whole corpus.

Use `MEMORY_PROFILE_DIRECT_CHECKPOINT=--direct-checkpoint` to prepare a
checkpoint residency fixture directly from segment files instead of measuring
the WAL/write path. This mode is intended for large lazy-open RSS evidence: it
writes descriptor-backed checkpoint segments and empty secondary index files,
then the real measurement should be a fresh-process `--reopen-only` run against
that prepared root. Do not use direct-checkpoint reports as a search-quality
benchmark.

## Report

The report schema is `cortexdb.memory_profile.v1` and includes:

- process RSS and peak RSS from `/proc/self/status`;
- `storage_stats()` memory estimates after put, checkpoint, and reopen;
- estimated-to-RSS ratios;
- a static payload clone gate covering A04/A05 hot clone regressions;
- allocator observers for `dhat` and `jemalloc`.

The default portable profile does not enable `dhat` or `jemalloc` because the
workspace does not add new profiling dependencies by default. Those observers are
reported as unavailable until an explicit profiling dependency/runtime decision is
made.

## Current Local Evidence

Latest local 10K profile:

```text
report: target/memory-profile/10k/report.json
cells: 10000
ok: true
rss_bytes: 38936576
peak_rss_bytes: 40894464
estimated_total_memory_bytes: 28795568
rss_to_estimated_total_ratio: 1.352
peak_rss_to_estimated_total_ratio: 1.420
payload_clone_gate_passed: true
```

A08 lazy smoke profile:

```text
memory resident payload after reopen: 11184 bytes
lazy resident payload after reopen: 0 bytes
```

A08 fresh-process 10K x 4KB reopen profile:

```text
prepared root: target/memory-profile/a08-memory-10k-4kb
memory report: target/memory-profile/a08-reopen-memory-10k-4kb/report.json
lazy report: target/memory-profile/a08-reopen-lazy-10k-4kb/report.json
cells: 10000
payload_bytes: 4096
memory RSS: 924790784
lazy RSS: 110858240
RSS memory/lazy ratio: 8.342
memory peak RSS: 943800320
lazy peak RSS: 129933312
peak RSS memory/lazy ratio: 7.264
memory resident payload after reopen: 40960000 bytes
lazy resident payload after reopen: 0 bytes
memory estimated total: 128993526
lazy estimated total: 47073526
estimated memory/lazy ratio: 2.740
memory reopen duration: 6298.253 ms
lazy reopen duration: 450.410 ms
```

A08 fresh-process 10K x 4KB reopen + 50 `get_latest` samples:

```text
memory report: target/memory-profile/a08-reopen-memory-10k-4kb-read50/report.json
lazy report: target/memory-profile/a08-reopen-lazy-10k-4kb-read50/report.json
memory RSS: 926367744
lazy RSS: 111054848
RSS memory/lazy ratio: 8.342
memory resident payload after reopen: 40960000 bytes
lazy resident payload after reopen: 0 bytes
memory get_latest p95: 0.002 ms
lazy get_latest p95: 1.423 ms
lazy/memory p95 ratio: 711.500
```

A08 fresh-process 1M x 512B direct-checkpoint reopen + 50 `get_latest` samples:

```text
prepared root: target/memory-profile/a08-direct-1m-512b
memory report: target/memory-profile/a08-direct-1m-512b/reopen-memory-read50-tailstore.json
lazy report: target/memory-profile/a08-direct-1m-512b/reopen-lazy-read50-tailstore.json
cells: 1000000
payload_bytes: 512
batch_size: 10000
prepare mode: direct_checkpoint fixture, then fresh-process reopen
memory after_open RSS: 13973860352
lazy after_open RSS: 1665662976
after_open RSS memory/lazy ratio: 8.389
memory after_reopen_stats RSS: 14041493504
lazy after_reopen_stats RSS: 1733615616
after_reopen_stats RSS memory/lazy ratio: 8.100
memory peak RSS: 14368604160
lazy peak RSS: 1733615616
peak RSS memory/lazy ratio: 8.288
memory resident payload after reopen: 512000000 bytes
lazy resident payload after reopen: 0 bytes
logical payload in both modes: 512000000 bytes
memory estimated total: 3804000456
lazy estimated total: 2780000456
memory get_latest p95: 0.002 ms
lazy get_latest p95: 1.328 ms
```

Interpretation:

- The estimate is lower than process RSS because RSS includes allocator/runtime
  overhead and non-database process memory.
- Small smoke runs can have identical RSS in memory and lazy mode because process
  overhead dominates the payload bytes, and same-process build/reopen runs can
  retain allocator memory from ingestion/checkpoint. Use `--reopen-only` for
  RSS comparisons.
- The gap is now visible and reproducible instead of being an undocumented claim.
- Lazy payload residency trades RAM for on-demand disk reads: the local 10K x
  4KB read sample and 1M x 512B sample show a large relative p95 increase, but
  the absolute lazy `get_latest` p95 remains low at these scales.
- The static clone gate is not an allocator profiler; it prevents known
  full-payload clone regressions from returning to checkpoint and VERIFY paths.
