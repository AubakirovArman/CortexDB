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

`EPIC-C11` — AQL query cache: metrics and policy.

C11 exit steps:

1. Expose AQL query cache hit/miss/eviction/capacity metrics in stats or
   metrics surfaces.
2. Document and test catalog/seq invalidation policy.
3. Add configured size/policy coverage without changing default behavior.

C11 current state:

- next.
- C10 is closed with plan-aware segment pruning from existing manifest zone
  maps, freshness created-at range pruning, conservative unknown/NOT behavior,
  10-segment fixture coverage, and EXPLAIN `segment_pruning` counters.
- C09 is closed with view-pruned AQL index construction, conservative
  scope-zone segment skipping, stale-candidate safety, planner cardinality
  coverage, and EXPLAIN permission pruning counters.
- C07 is closed with AQL `USING MODE hybrid`, physical lexical+dense RRF,
  quality fixture coverage, and EXPLAIN/ANALYZE path reporting.
- C06 is closed with nightly/manual ANN reports, optional BGE-M3 cache recall
  gate, and planner coverage for large-corpus ANN with exact fallback.
- C05 is closed with `ACV1` contiguous disk vector rows, disk-resident exact
  scan, stable chunked dot-product scoring, and `ACV0` read-only compatibility.
- C04 is closed with configured Unicode analyzer/tokenizer, optional stemming,
  and manifest analyzer-profile protection.
- C03 is closed with canonical BM25 and field weights.
- Do not reopen C01 unless `ACI4` compact lexical format or dual-read tests regress.
- Do not reopen B20 unless `BRAIN_SEMANTICS.md` or alias tests regress.
- Do not reopen B19 unless REMEMBER write contract or allocation tests regress.
- Do not reopen C15 unless graph traversal budget accounting or 100K p95
  evidence regresses.

## Active Partial Tail

`EPIC-D05` — SDK publish.

D05 split state:

- local package gates exist;
- publication remains externally blocked on public registry credentials/trusted
  publishing;
- do not block kernel/database epics on D05.

## Recently Closed

### EPIC-C10 — Segment zone maps + segment skipping

Status: `done`

What closed it:

- Reused existing manifest zone maps for created-at min/max and
  scope/status/type counts.
- AQL cache binding now builds the persisted index with the bound retrieve
  bitmap program.
- Segment-level bitmap evaluation prunes scope/status/type predicate segments
  while treating `NOT`, memory-type, and unknown handles conservatively.
- Freshness requirements prune by created-at range before persisted indexes are
  opened.
- EXPLAIN emits `segment_pruning` skipped/opened/total counters.
- Added 10-segment type pruning and freshness pruning regression fixtures.

Important follow-up:

- C11 should make AQL query cache behavior visible through metrics/policy.

### EPIC-C09 — Permission-aware index pruning

Status: `done`

What closed it:

- AQL cached and uncached binding now builds an AgentView-pruned index.
- Persisted checkpoint indexes open only segments whose stats may contain a
  readable scope.
- Skipped segments still contribute candidate footer removals, so unreadable
  patch/tombstone segments cannot resurrect stale readable candidates.
- The cost model has a 1% allowed-cardinality fixture selecting bitmap-first.
- EXPLAIN emits `permission_pruning` with skipped/opened/total segments.

Important follow-up:

- C10 generalizes segment skipping beyond permission scopes to temporal/type
  zone-map predicates.

### EPIC-C07 — Hybrid retrieval in engine

Status: `done`

What closed it:

- Added `RetrievalMode::Hybrid` parsing/binding and mode labels.
- Routed AQL hybrid retrieval through `BitmapIndexScan`, `PermissionFilter`,
  `LexicalScan`, `VectorScan`, and `HybridRrfOp`.
- Kept the implementation generic: no EnterpriseRAG-specific heuristics.
- Added a quality fixture where the hybrid candidate with both lexical and
  dense evidence ranks above single-signal candidates.
- Added EXPLAIN/EXPLAIN ANALYZE coverage for lexical/vector paths and RRF
  fusion.

Important follow-up:

- AQL hybrid needs an explicit `query_vector=` task line unless callers use the
  existing server-side embedding integration before execution.

### EPIC-C06 — HNSW guarded productization

Status: `done`

What closed it:

- Moved the broad ANN report bundle out of Rust PR CI into the nightly/manual
  ANN Regression workflow.
- Added `ann-nightly-regression-report` and `ann-nightly-reports` artifact
  upload.
- Added `ann-bge-m3-cache-recall-report` for the EnterpriseRAG BGE-M3 cache,
  with hosted-CI readiness fallback when the external cache is absent.
- Added cost-planner coverage for 1M+ broad corpora selecting ANN guarded by
  exact fallback while preserving exact for disabled HNSW and selective
  candidate sets.

Important follow-up:

- Hosted CI needs an external cache/artifact to execute the BGE-M3 recall gate;
  otherwise it publishes readiness only.

### EPIC-C05 — Disk-resident vector storage + SIMD exact scan

Status: `done`

What closed it:

- Added `ACV1` with a header, candidate table, contiguous fixed-dimension i16
  vector rows, and CRC.
- Kept `ACV0` read-only compatible.
- Routed persisted exact vector and hybrid vector legs through
  `VectorIndexReader` so exact search scans disk rows instead of materializing
  all vectors.
- Hid stale older segment vector rows by scanning live segments newest-to-oldest.
- Added stable chunk-8 deterministic dot-product scoring.

Important follow-up:

- Full 1M×768d p95 remains a C17 benchmark-packet run under the scale-gate rule.
- HNSW graph search still uses RAM-oriented vector maps; no-fallback and larger
  cache-backed promotion remain future benchmark work.

### EPIC-C04 — Unicode tokenizer + optional stemming

Status: `done`

What closed it:

- Added `TextAnalyzerConfig` to `DatabaseOptions`; default remains neutral with stemming disabled.
- Routed configured analyzer through checkpoint/compact, replication install, snapshot search, persisted `.aci` search, and AQL delta merge.
- Added manifest `ANLZ` analyzer profile and open-time mismatch rejection to prevent mixed token streams.
- Added Russian stemming fixture for `бюджету -> бюджет`, persisted search coverage, and manifest roundtrip coverage.
- Documented analyzer config and storage profile in `SEARCH.md`, `STORAGE_FORMATS.md`, and `ENGINE_API.md`.

### EPIC-C03 — Real BM25 with field weights

Status: `done`

What closed it:

- Added shared fixed-point BM25 helpers with `k1=1.2`, `b=0.75`, Q16 scoring, and float-reference tests.
- Wired canonical BM25 into live lexical search, persisted `.aci` search, AQL candidate ranking, retrieved-cell ranking, ContextPack `base_bm25`, and the enterprise retrieval benchmark scorer.
- Kept field weights through field term frequencies (`title=8`, `path=5`, `body=1`) and added persisted-vs-live field BM25 parity coverage.
- Added `docs/SCORING.md` and refreshed search/architecture/benchmark docs.

### EPIC-C01 — Term interning + compact postings

Status: `done`

What closed it:

- Added `ACI4` lexical index format with a sorted term dictionary and term-id postings.
- Added delta-varint candidate/frequency streams for compact postings.
- Kept `ACI0`, `ACI1`, `ACI2`, and `ACI3` read-only compatible.
- Added `docs/LEXICAL_INDEX.md`, storage format docs, compatibility fixtures, and `lexical-index-contract-check`.
- Proved legacy `ACI3` dual-read/rewrite and >3x compact persisted fixture reduction under `lexical_index_tests`.

### EPIC-B20 — Multi-brain semantics or removal

Status: `done`

What closed it:

- Added `docs/BRAIN_SEMANTICS.md`: `default = BrainId(1)` is the only real brain.
- Documented non-default brain names as deprecated aliases, not isolation namespaces.
- Routed runtime/statistics AQL catalogs through `resolve_single_brain_name`.
- Added `brain_semantics` tests and `multi-brain-contract-check` in `make check`.

### EPIC-B19 — REMEMBER write-path policy formalization

Status: `done`

What closed it:

- Added `docs/AQL_V0_5.md` as the REMEMBER write contract over the v0.4 grammar.
- REMEMBER IDs now use manifest-backed `memory_cell_cursors`; generic ingest IDs use manifest-backed `next_cell_id` outside the memory namespace.
- Added concurrent REMEMBER allocation and remember→retrieve→verify regression tests.
- Updated the descriptor hot-path gate to require the manifest cursor invariant.

### EPIC-C15 — Incremental graph index performance

Status: `done`

What closed it:

- `KnowledgeGraphIndex` now stores compact adjacency as interned entity ids plus edge ids.
- Bulk graph-index build uses an add-only path instead of per-record remove scans.
- Graph retrieval has `GraphRetrievalReport` with visited edge/entity counts and `budget_exceeded`.
- `make graph-index-performance-check` passed on a 100K-node graph with p95 `0.550493ms`.

### EPIC-B18 — Incremental knowledge graph/provenance index

Status: `done`

What closed it:

- `GraphIndexStore` updates incrementally with `insert_record`/`remove_record`.
- `KnowledgeGraphIndex` now maintains adjacency, edge-kind, source-reference, and `source_support_edges_by_fact` maps.
- Graph APIs read the maintained store; lazy graph queries no longer rebuild from visible payloads.
- VERIFY source-support enrichment reads indexed source-support edges for current evidence cell ids.
- Added `docs/KNOWLEDGE_GRAPH.md`, `graph_index_incremental_tests`, and `knowledge-graph-check` in `make check`.

### EPIC-B17 — Typed tool registry

Status: `done`

What closed it:

- `Database::list_tools` and `recommend_tools_for_task` now read `ToolIndex`, not query-time visible-cell scans.
- `ToolIndex` maintains typed tool records plus `term_to_tools`/`tool_terms` maps for recommendation.
- Lazy payload open-time derived-store rebuild repopulates the tool index from visible payloads.
- Added `docs/TOOL_REGISTRY.md` as the short B17 contract and agent context+tools example.
- Added `tool_registry_index_tests` and wired `tool-registry-check` into `make check`.

### EPIC-B15 — EXPLAIN ANALYZE for AQL

Status: `done`

What closed it:

- Added CLI flag form `cortexdb aql ... --explain analyze` and `--explain plan`.
- Added HTTP query param `POST /v1/aql?scope=...&explain=analyze` for normal `RETRIEVE` bodies.
- Added explicit `actual_input_count`, `actual_output_count`, and nullable `estimated_output_count` to AQL execution trace operator responses.
- Documented examples and trace fields in `docs/EXPLAIN_ANALYZE.md`.
- Aligned OpenAPI and generated Python/TypeScript OpenAPI SDK types.

### EPIC-B14 — Explainability contract

Status: `done`

What closed it:

- Added typed `ContextPack::explain_cell(CellId)` for selected, excluded, and not-considered cells.
- Added CLI `cortexdb explain <db> <scope> <aql> --cell-id N` with summary and `context_cell_explain.v1` JSON output.
- Documented selected/excluded explain fields and first exclusion stages in `docs/EXPLAIN.md`.
- Added golden/regression coverage for stable selected fields and excluded `first_excluding_stage`.
- Aligned ContextPack schema/OpenAPI/generated SDK types with the existing `visible_conflict` anomaly code.

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

- B14 is closed. Long-running benchmark evidence remains A19/C17 scope.

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

Done count in roadmap snapshot: `73`.

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
- C05 disk-resident vector storage and exact scan;
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

Partial count in roadmap snapshot: `1`.

- D05 SDK publish:
  local gates exist; public registry publication remains externally blocked.

## Frozen Snapshot

- F02 replication;
- F03 consensus;
- F09 cloud/service.

Frozen means do not implement unless the plan explicitly thaws the epic.

## Next Exit Step

Work on C07 only:

1. inspect current hybrid search and AQL retrieval mode boundaries;
2. wire lexical+dense streams through a generic RRF operator;
3. add a quality fixture proving hybrid is at least lexical;
4. move to the next ordered epic after C07 acceptance is closed.
