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

`EPIC-E02` — Backup UX happy path + verify.

E02 exit steps:

1. Make one documented backup command create a validated offline copy.
2. Verify backup manifest, WAL, segments, and indexes.
3. Add happy-path restore verification.
4. Mark done when a user can backup and verify with one documented flow.

E02 progress:

- next: inventory current backup/restore commands, docs, and tests before
  choosing the smallest missing UX/evidence slice.

## Recently Closed

### EPIC-E04 — Corruption handling

Status: `done`

What closed it:

- Added typed validation issues with recovery actions for manifest, WAL,
  segment, bitmap, lexical, vector, HNSW, candidate mapping, and manifest
  reference problems.
- Added path-level `Database::validate_storage_path_report`, so manifest and
  segment corruption can produce actionable reports even when normal
  `Database::open` fails.
- `cortexdb validate`, `doctor`, and `repair --dry-run` now expose issue kind,
  recovery action, restore requirement, and a recommended command.
- Added `docs/CORRUPTION_HANDLING.md` with the Core Alpha quarantine policy:
  no unsafe in-place quarantine for live manifest/segment/bitmap/lexical
  artifacts; restore into a separate verified path unless a class has an
  explicit safe repair/rebuild path.
- Corruption matrix tests now assert typed recovery actions for manifest,
  live segment, bitmap, lexical, vector, and HNSW corruption.

Next:

- Continue with E02 as directed by `EPIC_EXIT_STEPS.md`.

### EPIC-E10 — Fuzzing decode paths

Status: `done`

What closed it:

- Added a no-new-deps deterministic decode fuzz gate over real seed files built
  by the normal WAL/storage writers.
- Covered WAL records and WAL files; segment record, descriptor, candidate, and
  payload lookup paths; bitmap, lexical, vector, and HNSW indexes; manifest
  load; and AQL parser diagnostics.
- Added byte truncation, byte flip, appended-noise, pure-noise, and optional
  deterministic extra mutation rounds.
- Added `make decode-fuzz-check` and included it in `make check`.
- Documented local and longer soak commands in `docs/DECODE_FUZZING.md`.
- Gates passed: `cargo test -p cortex-engine --test decode_fuzz --all-features`,
  `CORTEXDB_DECODE_FUZZ_EXTRA_CASES=10 cargo test -p cortex-engine --test decode_fuzz --all-features`,
  `make decode-fuzz-check`, file-size ratchet, `cargo fmt --check`,
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Next:

- Continue with E04 as directed by `EPIC_EXIT_STEPS.md`.

### EPIC-E08 — Tenant isolation test suite

Status: `done`

What closed it:

- Tenant route matrix covers `/v1/cell`, `/v1/search`, `/v1/context`,
  `/v1/aql`, `/v1/verify`, `/v1/stats`, `/v1/validate`, and `/v1/metrics`
  for cross-tenant payload isolation.
- Same numeric AgentView id is loaded from the requested tenant realm, proving
  one tenant's scope grants do not authorize another tenant's realm.
- Generated invalid tenant values cover traversal, separators, whitespace,
  reserved characters, and percent-encoded variants; rejected tenants do not
  create `realms/`.
- Gates passed: `cargo test -p cortex-server tenancy --all-features`,
  file-size ratchet, `cargo fmt --check`,
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Next:

- E09 was already closed, so continue with E10.

### EPIC-B05 — AgentView lifecycle API v1

Status: `done`

What closed it:

- Stable AgentView lifecycle surface now includes server create/list/show routes
  and CLI create/list/show/grant/revoke commands.
- CLI supports human and JSON output for AgentView lifecycle commands.
- Admin authz tests cover `/v1/agents` create/list/show and data-role denial.
- CLI tests cover lifecycle mutation behavior and a two-agent scope isolation
  scenario using AgentViews created through CLI commands.
- `docs/AUTH.md` documents the durable file-backed `agent_views/*.view`
  compatibility bridge; system-cell migration remains future work.

Next:

- E08 extends this security surface into a tenant isolation route matrix.

### EPIC-B04 — AgentView as an index invariant before payload reads

Status: `done`

What closed it:

- AQL retrieval already builds `agent_allowed` from maintained
  `EngineAqlIndex` scope bitmaps before bitmap evaluation.
- Search uses bitmap-backed `allowed_candidates` before persisted search result
  materialization.
- Direct server cell routes now authorize durable descriptors before payload
  reads for `/get`, `/v1/cell`, tombstone/delete, batch tombstone, and
  `/v1/forget`.
- Verification source-support enrichment checks relation descriptor scope before
  lazy persisted relation payload reads.
- Memory and session ID allocation use descriptor-only existence checks instead
  of loading payloads to detect collisions.
- Remaining `get_latest_cell*` surfaces are classified as public payload API,
  post-verification response shaping, validation/reporting, tests, or benchmark
  binaries rather than pre-auth runtime reads.
- Structural gates cover server, verification, memory/session allocation,
  descriptor-first indexed paths, and query scan inventory.
- Final B04 gates passed: file-size ratchet, `cargo fmt --check`,
  descriptor hot-path gate, targeted server/security/verification/session/memory
  tests, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Important follow-up:

- B05 productizes AgentView lifecycle management. Any future non-AQL read
  surface must be added to the descriptor hot-path gate before it can read
  payload bytes.

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

Done count in roadmap snapshot: `40`.

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
- E10 decode fuzzing gate;
- E08 tenant isolation test suite;
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

Work on E04 only:

1. inventory current validation, repair, and corruption-report paths;
2. define quarantine/report behavior for corrupt WAL, segments, and indexes;
3. add operator-facing diagnostics and regression tests by corruption class;
4. move pointer only after E04 is `done` or explicitly split with accepted
   follow-up scope.
