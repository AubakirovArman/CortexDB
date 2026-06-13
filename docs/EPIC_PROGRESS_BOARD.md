# CortexDB Epic Progress Board

Last updated: 2026-06-13

Purpose: short operational board for active epic execution. The detailed source
of truth remains `docs/DATABASE_GRADE_EXECUTION_PLAN.md`; this file shows what
is done, what is partial, and what must be done next so we do not reopen closed
work by accident.

## Rule

- Move by the ordered execution queue, not by random topic hopping.
- A `partial` epic can be revisited only for its listed remaining exit steps.
- Do not redo accepted work unless a new failing test or explicit user request
  changes the scope.
- Update this file whenever an epic moves between `current`, `done`,
  `partial`, or `next`.

## Current Pointer

`EPIC-A08` — lazy payload residency follow-up.

Already accepted:

- lazy payload residency path exists;
- 1M RSS evidence accepted;
- checkpoint-backed lazy payload materialization works across many read paths.

Remaining A08 exit steps:

- broader lazy crash/corruption/restart parity;
- full AQL/ContextPack p95 beside memory-mode p95;
- decide whether remaining latency/RSS work stays in A08 or moves to a
  dedicated performance epic.

## Recently Closed

### EPIC-A06 — Indexed-only retrieve/ContextPack path

Status: `done`

What closed it:

- query-adjacent scan inventory stays at zero:
  `query_adjacent=0 maintenance_or_backfill=7 non_runtime_gates=2`;
- `scale_benchmark_check` gained direct prepared indexed fixtures:
  `--direct-checkpoint` and `--reopen-only`;
- 10K direct checkpoint smoke passed:
  `target/scale-bench/a06-direct-smoke-10k/report.json`;
- 1M direct indexed benchmark passed:
  `target/scale-bench/a06-direct-1m-context/report.json`;
- 1M validation passed:
  `cells_checked=1000000`, `live_segments_checked=20`,
  `manifest_ok=true`, `wal_ok=true`, `errors=[]`;
- 1M ContextPack p95 published:
  `p95_ms=11633.309`, `p50_ms=11064.372`, `p99_ms=12034.731`;
- correctness fixture stayed stable:
  `cargo test -p cortex-engine --test context_verify_quality --all-features`.

Important follow-up:

- A06 is closed for indexed-path evidence, but 1M ContextPack latency is high.
  The high p95 and after-open RSS are now performance work for A08/C-track, not
  a reason to reopen A06 from scratch.

## Done Snapshot

Done count in roadmap snapshot: `35`.

High-signal done epics:

- A01 clean repo and reproducibility;
- A02 typed cell descriptor;
- A03 data model docs;
- A04 MemTable no-clone iterators;
- A05 indexed VERIFY FACT;
- A06 indexed-only retrieve/ContextPack evidence;
- A07 segment footer/random payload access;
- A09 disk-resident persisted-index incremental merge;
- A10 logical plan;
- A11 operator executor;
- A12 storage statistics;
- A13 cost model v0;
- A14 snapshot pinning and GC barrier;
- A15 transactional WriteBatch API;
- A16 concurrent read path;
- A17 checkpoint without stop-the-world;
- A18 background incremental compaction;
- A20 property tests for MVCC/WAL/recovery;
- B01 ContextPack JSON Schema v1;
- D02 init/doctor;
- D06 Python SDK;
- D07 TypeScript SDK;
- D08 async Rust SDK/shared API types;
- D09 Docker quickstart;
- D10 OpenAPI/codegen control;
- D11 MCP adapter;
- E01 WAL writer error surfacing;
- E09 AgentView rights property suite;
- E11 chaos/graceful shutdown;
- E12 migration framework.

## Partial Snapshot

Partial count in roadmap snapshot: `5`.

- A08 lazy payload residency follow-up:
  crash parity and full AQL/ContextPack p95 remain.
- A19 scale benchmarks:
  long-running load evidence and later larger-scale evidence remain.
- C16 memory profiling:
  estimated-vs-real verification and gate integration remain.
- D05 SDK publish:
  local gates exist; public registry publication remains.
- D15 beta tag:
  release-management decision remains.

## Frozen Snapshot

- F02 replication;
- F03 consensus;
- F09 cloud/service.

Frozen means do not implement unless the plan explicitly thaws the epic.

## Next Exit Step

Work on A08 only for its remaining tail:

1. identify which lazy crash/corruption/restart scenarios are not yet mirrored;
2. run or extend the lazy parity matrix;
3. publish memory-mode vs lazy-mode full AQL/ContextPack p95;
4. update `docs/DATABASE_GRADE_EXECUTION_PLAN.md` and this board;
5. move pointer to the next ordered epic only after A08 is `done` or explicitly
   split with accepted follow-up scope.
