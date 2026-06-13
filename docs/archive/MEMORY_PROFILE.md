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
  MEMORY_PROFILE_PAYLOAD_RESIDENCY=memory
```

Use `MEMORY_PROFILE_PAYLOAD_RESIDENCY=lazy` to measure checkpoint reopen with
segment-backed payloads left on disk.

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

Interpretation:

- The estimate is lower than process RSS because RSS includes allocator/runtime
  overhead and non-database process memory.
- Small smoke runs can have identical RSS in memory and lazy mode because process
  overhead dominates the payload bytes; the resident payload counter is the
  precise signal for this scale.
- The gap is now visible and reproducible instead of being an undocumented claim.
- The static clone gate is not an allocator profiler; it prevents known
  full-payload clone regressions from returning to checkpoint and VERIFY paths.
