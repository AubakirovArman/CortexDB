# CortexDB Epic Progress Board

Last updated: 2026-06-14

Purpose: short operational board for active epic execution. The detailed source
of truth remains `docs/DATABASE_GRADE_EXECUTION_PLAN.md`; this file shows what
is done, what is partial, and what must be done next so we do not reopen closed
work by accident.

## Rule

- Move by the ordered execution queue, not by random topic hopping.
- A `partial` epic can be revisited only for its listed remaining exit steps.
- Do not redo accepted work unless a new failing test or explicit user request
  changes the scope.
- Use small/medium gates inside implementation epics. Large 1M/10M runs remain
  explicit benchmark packets unless the current epic needs them for safety.
- Update this file whenever an epic moves between `current`, `done`,
  `partial`, or `next`.

## Current Pointer

`EPIC-B14` — Explainability contract.

B14 exit steps:

1. Inventory existing explain-v2 surfaces.
2. Freeze the formal explainability contract in docs/schema/golden tests.
3. Add missing cell-specific/golden-test coverage without rewriting accepted explain code.
4. Publish focused evidence, then move to B15.

B14 current state:

- next; formal-tail only, not greenfield.
- Do not reopen C14 unless temporal interval index, AQL `valid at`, or VERIFY
  stale-guard contracts regress.

## Active Partial Tail

`EPIC-D05` — SDK publish.

D05 split state:

- local package gates exist;
- publication remains externally blocked on public registry credentials/trusted
  publishing;
- do not block kernel/database epics on D05.

## Recently Closed

### EPIC-C14 — Temporal index

Status: `done`

What closed it:

- Added incremental temporal validity indexes over `valid_from` and `valid_to`.
- Added sorted `valid_from` zone cache for AQL `REQUIRE valid at` candidate filtering before lazy payload reads.
- Routed VERIFY stale/future guard reasons through `VerificationTemporalIndexLookup`.
- Added `make temporal-validity-index-check`; 10K lazy fixture passed with `query_elapsed_ms=152`, `returned_cells=10`, `segment_loads_after_query=10`.

### EPIC-C13 — Fact/numeric index

Status: `done`

What closed it:

- Added metric/scope/project -> sorted normalized value -> cell maintenance to `FactClaimStore`.
- Routed numeric VERIFY candidate selection through the typed numeric index, with lexical fallback when the index has no typed hits.
- Batched conflict-index numeric rebuild by metric/scope/project groups.
- Added `make numeric-verify-index-check`; local 1M report passed with p95 `157.387ms`.

### EPIC-C17 — Performance regressions in CI

Status: `done`

What closed it:

- Added `.github/workflows/continuous-benchmark.yml` with nightly cron and manual dispatch.
- Added `make continuous-benchmark-hosted-gate`, which regenerates load-smoke, single-node performance, CI-safe 10K/100K scale reports, trend reports, memory audit, and the continuous benchmark gate.
- Gate keeps the `1.2` p95/p99 threshold and uses a 25ms minimum absolute-delta floor for runner jitter.
- The workflow uploads `continuous-benchmark-reports` with JSON/Markdown evidence.
- Local `make continuous-benchmark-hosted-gate` passed.

### EPIC-A19 — Scale benchmarks 100K/1M/10M and curves

Status: `done`

What closed it:

- `make scale-bench-10m-lazy` now captures the controlled 10M lazy RSS/read/restart packet.
- `target/scale-bench/inventory.json` is `complete` with 19 reports and no missing acceptance items.
- `target/scale-bench/trends.json` is `complete` with 38 curves and A05/A06/A08/A09 optimization labels.
- `docs/SCALE_BENCHMARKS.md` publishes the 10M lazy packet and clearly scopes skipped storage estimates/validation.

### EPIC-B13 — Feedback as an indexed ranking signal

Status: `done`

What closed it:

- Reworked `FeedbackIndex` into maintained feedback records keyed by feedback
  cell plus indexed `source_cell_id -> feedback_record_ids` lookups.
- Added candidate-scoped feedback score APIs so ContextPack and EXPLAIN
  ANALYZE request only scores for the cells already in the candidate set.
- Added typed HTTP routes for `POST /v1/feedback` and
  `GET /v1/feedback/stats`, including source-cell existence checks,
  boolean validation, and AgentView read-permission filtering.
- Added shared API response structs, OpenAPI contract entries, generated
  Python/TypeScript OpenAPI types, API schema docs, and agent-memory docs.
- Restored the official-clean DeepSeek answer runner import path and adjusted
  the oracle audit so analysis scripts do not block clean artifacts.

Gates passed:

- `cargo fmt --check`
- `cargo test -p cortex-engine --test feedback_tests --test
  feedback_index_tests --all-features`
- `cargo test -p cortex-server feedback --all-features`
- `python3 -m py_compile scripts/enterprise_rag_bench/run_deepseek_answers.py
  scripts/enterprise_rag_bench/oracle_usage_audit.py
  scripts/check_openapi_contract.py`
- `python3 scripts/descriptor_hot_path_gate_check.py`
- `python3 scripts/query_scan_inventory_check.py`
- `make file-size-check`
- `make openapi-contract-check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make check`

DeepSeek 50-question smoke:

- run label: `deepseek50-20260614-b13`
- clean gate: passed; oracle audit: 0 violations
- metrics: `overall=30.6`, `correctness=32.0`, `completeness=36.4`,
  `document_recall=56.0`, `invalid_extra_docs=9.44`
- token accounting: `answer_tokens=286831`, `judge_tokens=24684`

Remaining follow-up:

- B14 owns stable explain contracts. Long-running benchmark evidence remains
  A19/C17 scope.

### EPIC-B12 — Session/episodic memory contract

Status: `done`

What closed it:

- Added `session_id` and `session_kind` to `CellDescriptor` payload-header
  materialization and binary descriptor encode/decode.
- Reworked `SessionIndex` into descriptor-backed records plus a maintained
  `session_id -> cell_id` map.
- Removed the lazy-residency full payload scan from
  `Database::retrieve_session_cells`; retrieval now filters by indexed
  descriptor session/scope/TTL metadata and materializes only matching payloads.
- Built the lazy-open session index from descriptors, so checkpoint-backed
  session cells are discoverable without resident payloads.
- Added `agent_session_lazy_tests` proving a lazy reopen retrieves one session
  while loading only that session's two payloads from segment storage.
- Documented the public session contract in `AGENT_MEMORY.md` and added the
  multi-session agent example at `examples/demo/agent_sessions/README.md`.

Gates passed:

- `cargo fmt --check`
- `cargo test -p cortex-core --all-features`
- `cargo test -p cortex-engine --test agent_session_tests --all-features`
- `cargo test -p cortex-engine --test agent_session_lazy_tests --all-features`
- `python3 scripts/descriptor_hot_path_gate_check.py`
- `python3 scripts/query_scan_inventory_check.py`
- `make file-size-check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make check`

Remaining follow-up:

- B13 is closed. Long-running benchmark evidence stays in A19/C17; no
  B12-specific large run is required.

### EPIC-B11 — Memory lifecycle as storage policy

Status: `done`

What closed it:

- Added descriptor-backed `MemoryLifecycleStore` with `expiry -> cells` and
  `cell_id -> lifecycle record` maps.
- Replaced `expired_memory_cells` and `memory_decay_scores` snapshot/payload
  scans with maintained lifecycle-index reads.
- Wired lifecycle updates into open/replay, put/patch, and tombstone through
  `DerivedStores` and `Database`.
- Added physical `MemoryLifecycleFilter` before candidate budget and payload
  materialization, so expired memory is excluded from AQL retrieve even before
  the background TTL job tombstones it.
- Added deterministic memory decay into `RankOp`: temporary memory scores are
  multiplied by remaining TTL freshness while permanent memory keeps normal
  ranking.
- Updated `AGENT_MEMORY.md` and `INGESTION.md`.
- Gates passed: `cargo test -p cortex-engine --test memory_tests`, `cargo test
  -p cortex-engine --test memory_lifecycle_tests`, `cargo test -p
  cortex-engine memory::lifecycle`, `cargo test -p cortex-server
  v1_remember_ttl_expiry_disappears_from_context --all-features`,
  `python3 scripts/agent_memory_demo_check.py`, and `cargo check -p
  cortex-engine`; final gates: `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.

Remaining follow-up:

- B12 owns session-specific indexing/API contract and any session retrieval
  payload-scan debt.

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

Done count in roadmap snapshot: `55`.

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
- B04 AgentView as an index invariant;
- B05 AgentView lifecycle API v1;
- B06 typed provenance model;
- B07 fact/claim store;
- B08 VerifyOp as a planned operator;
- B09 contradiction/conflict index;
- B10 temporal validity columns and temporal queries;
- B11 memory lifecycle as storage policy;
- B12 session/episodic memory contract;
- B13 feedback as an indexed ranking signal;
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

Work on B14 only:

1. inventory current selected-result and excluded-result explain fields;
2. define the smallest stable explain schema that preserves existing output;
3. thread the schema through retrieve/search/context/verify without changing
   ranking behavior;
4. add snapshot/golden tests before moving to B15.
