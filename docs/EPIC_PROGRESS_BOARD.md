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

`EPIC-B11` — Memory lifecycle: TTL/decay as storage policy.

B11 exit steps:

1. Define TTL, decay, and retention policy as storage behavior.
2. Apply lifecycle during query and maintenance.
3. Add expiration and decay tests.
4. Mark done when memory lifecycle is deterministic and documented; then move to B12/B13.

B11 current state:

- next; start by inventorying current TTL/decay paths in `memory.rs`,
  `memory_accounting.rs`, session memory, and AgentView policy limits.
- Do not reopen B10 temporal filtering unless a lifecycle test fails because of
  temporal validity metadata.

## Active Partial Tail

`EPIC-A19` — scale benchmarks and RAM/latency curves.

A19 split state:

1. Keep reproducible commands for 100K/1M/10M scale targets.
2. Record RAM, open time, put/get/retrieve/verify/checkpoint latency.
3. Store reports in stable target/docs paths.
4. Compare each storage/indexing change against the baseline.
5. Keep 10M lazy evidence and historical optimization labels as the final
   long-running benchmark packet.

A19 progress:

- 100K/1M core lifecycle baselines exist and are documented;
- `scripts/scale_benchmark_inventory.py` writes
  `target/scale-bench/inventory.json` and currently reports 17 scale reports
  with `status=partial`;
- ContextPack p50/p95 exists for 100K and 1M;
- keyword_search and verify_fact p50/p95 now exist for 100K through
  `target/scale-bench/a19-search-verify-100k/report.json`;
- keyword_search and verify_fact p50/p95 now exist for 1M through
  `target/scale-bench/a19-search-verify-1m/report.json`;
- `make scale-bench-trends` writes `target/scale-bench/trends.json` and
  `target/scale-bench/trends.md`; current status is `partial` with 17
  multi-point current scale curves;
- 10M post-lazy RSS/latency, lazy cold outlier analysis, and historical
  before/after A05/A06/A08/A09 optimization curve labels remain;
- use small/medium gates while implementing; do not run 1M/10M unless this
  epic explicitly needs the evidence.

`EPIC-C17` — perf-regressions in CI and continuous benchmarking.

C17 split state:

- local `make continuous-benchmark-gate` exists and passed;
- p95/p99 threshold is `1.2` against latest history fixture
  `v0.2.0-beta.2`;
- hosted scheduled/nightly GitHub Actions wiring is deferred while Actions work
  is out of focus.

## Recently Closed

### EPIC-B10 — Temporal validity columns and temporal queries

Status: `done`

What closed it:

- Added AQL parser/binder support for `REQUIRE valid at "YYYY-MM-DD"` and
  carried it into `QualityThresholds.valid_at`.
- Added strict date validation and documented `valid_from`/`valid_to`
  descriptor semantics in `DATA_MODEL.md` and `AQL_V0_4.md`.
- Added maintained descriptor-backed `TemporalValidityStore`.
- Added physical `TemporalValidityFilter` before candidate budget/rank/payload
  materialization, so stale/future candidates are removed before payload reads.
- Preserved VERIFY stale-guard semantics through descriptor-backed metadata and
  kept the maintained `TemporalFactStore` query-time payload-scan-free.
- Evidence:
  `target/temporal-validity-gate/100k-memory-final/report.json` reports
  `ok=true`, `returned_cells=100`, `valid_expected=100`,
  `query_elapsed_ms=1454`, `segment_loads_after_query=0`.
- Lazy payload evidence:
  `target/temporal-validity-gate/10k-lazy-final/report.json` reports
  `ok=true`, `returned_cells=10`, `valid_expected=10`,
  `query_elapsed_ms=155`, `segment_loads_after_query=10`.
- Gates passed: `cargo fmt --check`, file-size ratchet, descriptor hot-path
  gate, memtable clone gate, targeted AQL/engine temporal tests,
  `cargo test --workspace --all-features`, clippy with `-D warnings`, and
  `make check`.

Remaining follow-up:

- 100K lazy-open still times out through the older payload-derived rebuild of
  conflict/fact stores. Track that as A19/C17/A08-tail performance work, not as
  a B10 correctness blocker.

### EPIC-B09 — Incremental contradiction/conflict index

Status: `done`

What closed it:

- Added maintained `ConflictIndexStore` for inline contradiction markers,
  persisted contradiction relations, descriptor/source facets, and typed
  numeric fact conflicts.
- Wired the store through put/patch/tombstone, replay/open rebuild, lazy reopen,
  and replication/snapshot derived-store rebuild paths.
- `Database::conflict_index` now delegates to the maintained store instead of
  scanning visible payloads at query time.
- Added `visible_conflict` ContextPack anomaly and typed
  `GET /v1/conflicts?scope=...` API/OpenAPI/SDK response shape.
- Preserved lazy read-path SLO accounting by using uncached maintenance payload
  reads for conflict-index lazy-open rebuild.
- Gates passed: `cargo fmt --check`, B09 targeted engine/server tests,
  `make openapi-contract-check`, file-size ratchet, full workspace tests,
  clippy with `-D warnings`, and `make check`.

Remaining follow-up:

- None for B09. If write amplification becomes measurable, track performance
  evidence under A19/C17 rather than reopening B09 behavior.

### EPIC-B08 — VerifyOp as a planned verification operator

Status: `done`

What closed it:

- `verify_fact_aql` now delegates to `Database::execute_verify_fact_plan`,
  preserving the public `VerificationReport`.
- VERIFY execution is traceable as `VerificationCandidateScan`,
  `VerificationPermissionFilter`, `VerificationMaterializeOp`, `VerifyOp`,
  `SourceSupportExpandOp`, and `VerdictAggregateOp`.
- Candidate/materialize/source-support loops are isolated under
  `verification/operator/candidates.rs`, leaving the main VERIFY path as an
  explicit operator pipeline.
- `BoundVerifyFactPlan` carries policy-clamped `max_candidates` and
  `max_evidence`; VERIFY uses those plan limits instead of hard-coded depth.
- Engine-level `explain_verify_aql` and `explain_analyze_verify_aql` expose
  logical policy rewrite and physical trace stages.
- Gates passed: `cargo fmt --check`, file-size ratchet,
  descriptor hot-path gate, `cargo test -p cortex-aql --all-features`,
  B08 targeted engine tests, `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Remaining follow-up:

- Server/CLI formatting for VERIFY explain is B15/API-surface work; B08 only
  closes the engine execution and explain boundary.

### EPIC-B07 — Fact/claim store with typed numeric values

Status: `done`

What closed it:

- Added `FactClaimStore` and `NumericFactRecord` backed by `NumericValue`.
- Conservative extraction requires a metric, materializes exactly one numeric
  value, and rejects ambiguous multi-value claims.
- Added `database::stores::DerivedStores` so the fact store rebuilds on open,
  updates on put/patch/tombstone, and rebuilds after replication snapshot
  install with the other maintained derived stores.
- `verify_fact_aql` now consults typed claims for numeric support/conflict
  evidence before sorting, while retaining the parser fallback for non-typed
  evidence and temporal guard paths.
- `scripts/fact_claim_store_inventory.py` reports `status=complete` with
  6 pass, 0 partial, 0 fail.
- Targeted gates passed: `cargo fmt --check`, file-size ratchet,
  fact-claim inventory, `cargo test -p cortex-engine verification::numeric::fact_claim --all-features`,
  `cargo test -p cortex-engine --test verification_guards --all-features`, and
  `cargo test -p cortex-engine --test verification_tests --all-features`.

Remaining follow-up:

- The metric-sorted numeric index and 1M p95 proof are not B07; they remain in
  `EPIC-C13`/`EPIC-A19`.

### EPIC-B06 — Typed provenance model

Status: `done`

What closed it:

- `make provenance-model-inventory` writes
  `target/provenance-model/inventory.json` and reports `status=complete` with
  9 passing checks.
- `CellDescriptor` carries descriptor-backed `source_id`, `source_url`,
  `document_id`, `page`, `row`, `cell_range`, `json_path`, `confidence_q16`,
  `citation`, `content_hash`, source/trust, and temporal fields through the
  existing WAL descriptor section.
- Metadata WAL writes preserve payload-derived provenance by merging metadata
  overlay fields with the payload descriptor instead of replacing it.
- ContextPack source_ref/citation export is covered by no-payload-header
  regression tests, and `DATA_MODEL.md` documents the model.

Next:

- Continue with B07. Do not reopen B06 unless provenance inventory regresses or
  a migration-specific issue appears.

### EPIC-D05 — SDK publication audit follow-up

Status: `partial`

What advanced it:

- `v0.2.0-beta.2` removed the previous version/tag blocker.
- The `SDK Release` workflow failure on the beta.2 tag was traced to Rust
  publish order: `cortex-sdk` was dry-run before `cortex-api-types` existed on
  crates.io.
- The workflow, release manifest, registry gate, and SDK release docs now model
  the correct order: `cortex-api-types` first, then `cortex-sdk`.
- Local gates pass again: `make sdk-release-contract-check`,
  `make sdk-registry-gate-check`, `make sdk-e2e-release-check`, and
  `make sdk-check`.
- GitHub repo state currently shows no `sdk-release` environment and no
  repo-level `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN`; public registry publication
  remains external.

Next:

- Do not claim public SDK packages until registry credentials/trusted
  publishing are configured and the manual tag-gated workflow succeeds.
- Continue with A19.

### EPIC-D15 — v0.2.0-beta.2 release/tag

Status: `done`

What closed it:

- Workspace, Rust crates, SDK packages, OpenAPI docs, migration fixtures, and
  release checker defaults target `0.2.0-beta.2` / `0.2.0b2`.
- `make beta-release-check` passed on committed SHA
  `bbd3b6c35a77a1d9c6d3845e9dd2b2ef91b16dc8` and wrote
  `target/beta-release/report.json` plus
  `target/beta-release/evidence.tar.gz`.
- The annotated tag `v0.2.0-beta.2` peels to the same verified commit.
- GitHub release
  `https://github.com/AubakirovArman/CortexDB/releases/tag/v0.2.0-beta.2`
  is a prerelease and includes the beta.2 local binary archive, checksum, and
  evidence bundle.
- The old published `v0.2.0-beta.1` tag was not force-moved.

Next:

- Continue with D05 because SDK publication was waiting on the beta tag.

### EPIC-E14 — Upgrade/rollback drill

Status: `done`

What closed it:

- `make upgrade-rollback-cli-flow-check` now validates current modular CLI
  paths and runs a real runtime drill.
- The drill creates a database, writes and flushes data, runs
  `cortexdb upgrade prepare`, verifies the immutable backup, validates the
  candidate database, restores rollback, validates rollback, and reads the
  payload from the rollback directory.
- The gate writes `target/upgrade-rollback-cli-flow/report.json`.
- `release-check` now invokes the upgrade/rollback flow gate.
- `docs/archive/UPGRADE_ROLLBACK.md` names the executable gate and evidence
  path.
- `make deployment-upgrade-check` passes after its modular CLI/Makefile
  checks were updated.

Next:

- D15 is now closed; continue with D05 as directed by `EPIC_EXIT_STEPS.md`.

### EPIC-E02 — Backup UX happy path + verify

Status: `done`

What closed it:

- Backup now writes `backup_manifest.tsv` with relative path, file size, and
  CRC32C for copied files.
- Added engine `Database::verify_backup_path` and CLI
  `cortexdb backup-verify <backup_path>` for verifying a backup without
  creating a restore target.
- `restore --dry-run` reports checksum manifest presence and verified file
  count when present.
- Docs now show the two-command happy path: `cortexdb backup ...` followed by
  `cortexdb backup-verify ...`.
- Regression test corrupts a backed-up segment and proves `backup-verify`
  rejects it through the checksum manifest.

Next:

- Continue with E14 as directed by `EPIC_EXIT_STEPS.md`.

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

Done count in roadmap snapshot: `46`.

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
- B06 typed provenance model;
- D02 init/doctor;
- D06 Python SDK;
- D07 TypeScript SDK;
- D08 async Rust SDK/shared API types;
- D09 Docker quickstart;
- D10 OpenAPI/codegen control;
- D11 MCP adapter;
- D15 beta.2 release/tag;
- E01 WAL writer error surfacing;
- E10 decode fuzzing gate;
- E08 tenant isolation test suite;
- E09 AgentView rights property suite;
- E11 chaos/graceful shutdown;
- E12 migration framework;
- C16 memory profiling harness.

## Partial Snapshot

Partial count in roadmap snapshot: `3`.

- A19 scale benchmarks:
  10M lazy evidence, cold outlier analysis, and historical before/after
  optimization labels remain.
- C17 perf-regressions:
  local gate exists; hosted scheduled/nightly CI wiring remains deferred.
- D05 SDK publish:
  local gates exist; public registry publication remains externally blocked.

## Frozen Snapshot

- F02 replication;
- F03 consensus;
- F09 cloud/service.

Frozen means do not implement unless the plan explicitly thaws the epic.

## Next Exit Step

Work on B07 only:

1. audit current numeric fact extraction and scan-based VERIFY conflict paths;
2. identify the smallest typed fact record/index that preserves current
   verdicts;
3. add fixture tests before replacing any VERIFY path;
4. keep extraction conservative and deterministic.
