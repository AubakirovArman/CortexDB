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

`EPIC-B03` — token-budget pushdown and early termination.

B03 exit steps:

1. Done: make PackOp signal when the token budget is filled.
2. Stop upstream operators as early as safely possible.
3. Done: clamp retrieve `LimitOp` to the budget-derived cost-model
   `recommended_candidate_limit`.
4. Move lazy payload reads behind permission/rank and bounded pack selection.
5. Preserve ContextPack fixture quality.
6. Publish a small/medium payload-read count or latency gate; large 1M/10M
   evidence remains A19/C17.

B03 progress:

- done: physical `LimitOp` now uses
  `min(cost_model.recommended_candidate_limit, plan.context_policy.candidate_limit)`;
- done: non-analyze `EXPLAIN RETRIEVE` reports the same effective returned
  limit;
- done: small tests cover `BUDGET 320 TOKENS LIMIT 10 CANDIDATES` producing
  2 budget-derived candidates from 5 quality-filtered candidates;
- done: `PackOp`/`PackExecution` expose a budget-filled signal covered by
  `pack_operator_reports_budget_filled_signal`;
- remaining: upstream early stop and bounded lazy payload fetch after cheap
  candidate/rank metadata.

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

Done count in roadmap snapshot: `37`.

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

Work on B03 only:

1. add a budget-full signal to PackOp/ContextPackBuilder;
2. stop upstream candidate/payload work when the pack is complete;
3. prove ContextPack fixture parity;
4. update `docs/DATABASE_GRADE_EXECUTION_PLAN.md` and this board;
5. move pointer only after B03 is `done` or explicitly split with accepted
   follow-up scope.
