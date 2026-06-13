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
- Use small/medium gates inside implementation epics. Large 1M/10M runs are
  accumulated in A19/C17 benchmark packets unless the current epic explicitly
  needs them for safety.
- Update this file whenever an epic moves between `current`, `done`,
  `partial`, or `next`.

## Current Pointer

`EPIC-B04` — AgentView as an index invariant before payload reads.

B04 exit steps:

1. Inventory read surfaces that still authorize after payload materialization.
2. Route readable candidate filtering through AgentView/descriptor indexes before
   payload fetch where possible.
3. Ensure `/get` and server read paths authorize from descriptor scope, not
   payload-parsed scope.
4. Extend the structural permission gate so no scan/read surface silently skips
   the permission predicate.

B04 progress:

- done: added descriptor-only `Database::get_latest_cell_descriptor` for
  pre-payload authorization;
- done: `/get`/`/v1/cell`, tombstone/delete, batch tombstone authorization, and
  `/v1/forget` now check durable descriptor scope before fetching payload;
- done: regression
  `denied_cell_routes_authorize_descriptor_before_lazy_payload_read` proves
  denied lazy GET, DELETE, and forget routes leave segment payload loads at 0;
- done: B04 progress gates passed: file-size ratchet, targeted server security
  tests, `cargo fmt --check`, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`;
- done: `descriptor_hot_path_gate_check.py` now requires descriptor-only lookup
  in server core/memory routes and forbids pre-auth
  `get_latest_cell_with_descriptor` in those auth paths;
- done: verification source-support enrichment now checks relation descriptor
  scope before lazy relation payload reads, with a regression proving unreadable
  persisted source-support relation payload is not materialized;
- done: the structural descriptor hot-path gate covers the verification
  source-support scan and persisted graph-edge enrichment paths;
- remaining: finish the broader non-AQL read-surface inventory and decide
  whether B04 can close on descriptor-index authorization or needs a separate
  permission-bitmap follow-up.

## Recently Closed

### EPIC-B03 — Token-budget pushdown and early termination

Status: `done`

What closed it:

- physical `LimitOp` now uses
  `min(cost_model.recommended_candidate_limit, plan.context_policy.candidate_limit)`;
- non-analyze `EXPLAIN RETRIEVE` reports the same effective returned limit;
- `PackOp`/`PackExecution` expose a budget-filled signal covered by full and
  non-full budget unit tests;
- `CheapRankBudgetOp` uses the AQL lexical index to rank candidate IDs without
  fetching payloads and bounds the input to `QualityFilter`;
- the small explain fixture shows `CheapRankBudgetOp` 5 -> 4 before payload
  materialization, `QualityFilter` 4 -> 4, and `LimitOp` 4 -> 2;
- lazy payload-read counter gate exposes `PayloadCacheStats::segment_loads` and
  proves the same bounded plan performs only 4 segment payload loads before
  returning the budget-derived 2 cells;
- ContextPack fixture quality stayed stable.
- final B03 gates passed: file-size ratchet, `cargo fmt --check`,
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Important follow-up:

- Large 1M/10M lazy ContextPack p95 evidence remains A19/C17 benchmark work, not
  a reason to reopen B03.

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

### EPIC-A08 — Lazy payload residency follow-up

Status: `done`

What closed it:

- explicit lazy restart-tail parity covers checkpoint+patch tail,
  checkpoint+tombstone tail, compact+patch tail, and compact+tombstone tail;
- explicit lazy corruption parity covers `.acs`, `.acm`, `.acb`, `.aci`,
  `.acv`, and `.ach` corruption as fail-closed or validation-error behavior;
- `cargo test -p cortex-engine --test lazy_payload_parity --all-features`
  passed;
- `make crash-fault-check` passed and wrote `target/crash-fault/report.json`;
- `scale_benchmark_check` supports `--payload-residency memory|lazy`;
- 100K prepared indexed fixture passed in both modes:
  - memory: after-open RSS `1236881408`, ContextPack p95 `1626.458ms`;
  - lazy: after-open RSS `818954240`, ContextPack p95 `1113.399ms`,
    max `59070.219ms`.

Important follow-up:

- the lazy cold max outlier and canceled 1M lazy ContextPack run are
  performance debt for A19/C17, not A08 blockers.

### EPIC-B02 — ContextPackBuilder as a physical operator

Status: `done`

What closed it:

- added `ContextPackBuilder` as the stateful internal boundary for pack
  construction;
- `PackOp` now implements `PhysicalOp<Item = ContextPack>` and remains
  compatible through `PackOp::execute`;
- `EXPLAIN ANALYZE RETRIEVE CONTEXT` now appends `PackOp` after `LimitOp`;
- ContextPack output parity stayed stable on targeted golden/quality fixtures.

Important follow-up:

- upstream early termination and avoiding full upstream candidate/payload
  materialization are B03 scope.

## Done Snapshot

Done count in roadmap snapshot: `38`.

High-signal done epics:

- A01 clean repo and reproducibility;
- A02 typed cell descriptor;
- A03 data model docs;
- A04 MemTable no-clone iterators;
- A05 indexed VERIFY FACT;
- A06 indexed-only retrieve/ContextPack evidence;
- A07 segment footer/random payload access;
- A08 lazy payload residency small/medium functional gate;
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
- B02 ContextPackBuilder physical operator;
- B03 token-budget pushdown and early termination;
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

Partial count in roadmap snapshot: `4`.

- A19 scale benchmarks:
  long-running load evidence, lazy cold outlier analysis, and later larger-scale
  evidence remain.
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

Work on B04 only:

1. inventory remaining read surfaces and current permission gates;
2. decide whether remaining engine/search/verification reads are already
   covered by descriptor indexes or need new B04 work;
3. run the E09/property/security subset plus full required gates when closing;
4. move pointer only after B04 is `done` or explicitly split with accepted
   follow-up scope.
