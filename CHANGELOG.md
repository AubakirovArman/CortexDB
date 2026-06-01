# Changelog

## Unreleased

### P1 (Alpha Polish)

- Added `docs/DOCUMENTATION_INDEX.md`,
  `docs/DOCUMENTATION_AUDIT.md`, and
  `docs/CONTEXT_PACK_TECHNOLOGY.md` so project markdown has an explicit source
  map and Context Pack has both a contract doc and a technology overview.
- Refreshed public-facing docs to remove stale Core Alpha "candidate" wording,
  avoid production-ready overclaims, and keep SDK publication / real-embedding
  automation described as local/tag-gated until beta.
- Added source-of-truth notices to roadmap mirror docs so `BETA_DELTA.md` and
  `REMAINING_EXECUTION_PLAN.md` remain the canonical current-status documents.
- Added `docs/PL_IMPLEMENTATION_TASKS.md`, a normalized actionable task list
  derived from the external `/mnt/hf_model_weights/arman/3bit/sites/pl.md`
  audit.
- Added backup restore-drill support through `Database::backup_restore_drill_path`
  and `cortexdb backup-drill`, proving a backup can be restored, opened,
  replayed, and validated before it is trusted operationally.
- Added backup retention pruning through `Database::prune_backup_retention` and
  `cortexdb backup-prune`, keeping the latest sortable backup directories after
  successful restore drills.
- Added validated offsite backup staging through
  `Database::stage_backup_offsite`, `cortexdb backup-offsite-stage`, and
  `make backup-offsite-check`, giving external upload tools an atomically
  published, preflight-restored backup directory.
- Added `make backup-drill-check`, which creates a temporary database, runs
  multiple restore drills, prunes old backups, validates readback, and writes
  `target/backup-drill/report.json` as release/runbook evidence.
- Added `make crash-fault-check`, which runs targeted crash/restart/corruption
  tests, injects a partial WAL tail plus orphan temp file through the CLI repair
  path, and uploads `target/crash-fault/report.json` in CI.
- Added `make chaos-restart-check`, which starts the real HTTP server, runs a
  fixed-seed sequence of writes, flushes, compacts, forced process kills, stale
  unlocks, repair, restart, and readback checks, then uploads
  `target/chaos-restart/report.json` in CI.
- Added static multi-token HTTP auth policies with `admin` and `data` roles,
  optional per-token AgentView binding, and admin/data route separation.
- Added first-class HNSW `ef_construction` reporting across engine, HTTP,
  OpenAPI, CLI JSON, SDK contracts, ANN corpus runs, and persisted `.ach/.acm`
  profile metadata.
- Added `make production-evidence-sweep`, which runs OpenAPI contract, backup
  drill, ANN release evidence, and replication partition checks with a combined
  `target/production-evidence/report.json` runbook artifact.
- Added `make ann-real-embedding-readiness`, a non-secret readiness report for
  real-domain ANN/HNSW baseline prerequisites before endpoint-backed embedding
  runs.
- Added the first endpoint-backed `investment-projects-v1` real-domain
  embedding baseline evidence with `BAAI/bge-m3`, 221 vectors, 40 queries, and
  a validated local baseline package.
- Documented the `investment_projects` real-domain embedding gate as local-only
  until beta, after a one-off GitHub Actions evidence run validated the workflow
  shape and artifacts.
- Added `docs/BETA_DELTA.md` and `make beta-delta-check` to keep the public
  Core Alpha vs beta-readiness statement aligned with production evidence,
  ANN real-embedding readiness, SDK publication, UI, and consensus blockers.
- Added `docs/PUBLIC_CLAIMS_POLICY.md` and `make public-claims-check` to keep
  README/API/status wording aligned with the current Core Alpha, experimental,
  blocked, and non-production boundaries.
- Added `single_node_performance_check` and `make single-node-performance-check`
  to emit a Strict/Balanced single-node engine performance matrix covering
  put/get/search/context/checkpoint/compact/restart lifecycle phases.
- Added `docs/BINARY_PLATFORM_MATRIX.md` and `make binary-platform-matrix-check`
  so binary releases prove a clean install through fixture load, query,
  backup/restore, and HTTP server health/query.
- Added `make tenant-recovery-check`, a real HTTP tenant isolation plus
  backup/restore gate that verifies tenant payload boundaries before and after
  restoring the server root.
- Added the `examples/real_domains/investment_projects` corpus with 56
  Kazakhstan/Central Asia investment-project documents, 165 chunks, 40
  analyst-style queries, ground truth, validators, and real-embedding
  readiness wiring.
- Added file-backed HTTP token rotation through `CORTEXDB_AUTH_TOKENS_FILE`;
  the server re-reads the local policy file per request and fails closed on
  missing, empty, or invalid token files.
- Hardened SDK release lifecycle checks: protected `sdk-release` deployment
  environment, Node.js 24 workflow runtime, and tag/version lock-step
  enforcement before public publish jobs.
- Added a checked SDK examples release artifact gate, packaging Rust, Python,
  and TypeScript examples with checksum evidence under
  `target/sdk-release-artifacts/`.
- Added SDK deprecation policy checks so deprecated OpenAPI aliases, SDK route
  usage, and breaking-change changelog requirements stay synchronized.
- Added `POST /v1/search/explain` endpoint — returns tokenized query terms and per-cell
  score breakdown (total, lexical, vector scores + payload preview).
- Added `?format=prometheus` to `GET /v1/metrics` — outputs 13 metrics in Prometheus
  text exposition format with `# HELP`/`# TYPE` annotations.
- Added `docs/AUTH.md` — Bearer token authentication guide with SDK examples.
- Added `docs/API_CHANGELOG.md` — version tracking from alpha through alpha.3.
- Added `docs/CLI.md` — full reference for 20+ CLI commands with examples.
- Added `docs/VERIFY_FACT.md` — how numeric conflict detection works, verdict taxonomy,
  and alpha limitations.
- Improved CLI error messages with actionable advice (suggest `repair`, `unlock --force`,
  check AQL syntax, etc.).

### P2 (Quality & Test Coverage)

- Added replication failure-injection integration coverage for minority
  partitions, stale leader rejection after a higher-term majority forms, and
  idempotent replication-log replay after restart.
- Added replication snapshot-resync and membership lifecycle coverage: chunked
  snapshot assembly rejects missing/mixed chunks before durable install, and
  voter reconfiguration tests prove join/leave quorum behavior.
- Added partition-aware in-memory replication transport plus a five-node
  partition matrix and TCP multi-chunk snapshot transport smoke coverage.
- Added durable peer snapshot install: TCP snapshot chunks can now be assembled
  and installed into follower database storage through `ReplicationPeerServer`.
- Added peer snapshot fault regression coverage: partial transfers, stale
  chunks, and corrupt final chunks do not replace durable follower state.
- Added durable membership rotation primitives in the replication log:
  `MembershipConfig`, `membership_entry`, and committed membership recovery.
- Added joint-consensus membership safety primitives: joint entries preserve old
  and new voter sets, and commit requires majorities from both sets.
- Added automated joint-consensus membership rotation that catches up voters,
  persists joint/stable config entries, recovers the final voter set after
  restart, and refuses stable publication without joint quorum.
- Added membership rotation restart-resume coverage: a recovered leader can
  finish the stable phase only after a committed joint config.
- Added crash/restart partition seed coverage for replicated logs: uncommitted
  partitioned writes stay uncommitted after restart until heal, and committed
  restarted leaders catch up healed followers.
- Added `repair_lagging_voter` for explicit node rejoin repair: small lag is
  caught up with missing `AppendEntries`, while lag above policy chooses
  snapshot install without mutating the follower log.
- Added `repair_lagging_voters` sweep reporting, so a leader can process
  caught-up, append-repair, and snapshot-required voters in one pass.
- Added `plan_replication_repair_sweep`, a progress-aware repair scheduler that
  validates durable follower progress and classifies voters before sending
  append repair or snapshot install work.
- Added `run_replication_repair_cycle`, which executes append-repair decisions
  from a schedule and hands snapshot-required followers to the future snapshot
  sender explicitly.
- Added `send_replication_snapshot_request`, which chunks snapshot repair
  handoff requests, verifies cumulative follower ACKs, and covers TCP durable
  follower install.
- Added `run_replication_repair_worker`, a bounded repair loop that reads
  follower progress, performs append repair, sends available snapshot repairs,
  and returns pending snapshot handoffs without spinning.
- Added `spawn_replication_repair_background_task`, giving the repair loop a
  stoppable OS-thread runtime boundary with deterministic finite-run tests.
- Added `Database::replication_snapshot_segment` and
  `ReplicationDatabaseSnapshotSource`, so background repair can source
  snapshots from current database storage instead of hand-built fixtures.
- Added `ReplicationFollowerProgressStore` and
  `ReplicationStoredProgressSource`, so background repair can resume planning
  from atomically persisted follower progress after restart.
- Added `ReplicationProgressRecordingTransport`, which persists successful
  AppendEntries and final snapshot ACK progress into the repair progress store.
- Added `spawn_replication_repair_background_task_with_progress_store`, the
  default background repair helper that uses one durable progress store for
  both repair planning input and successful peer ACK output.
- Added membership-aware repair progress reconciliation, so durable follower
  progress stores can prune retired voters, seed newly joined voters, and sync
  with stable or joint membership configs before repair planning.
- Added operator-facing replication path placement through
  `ClusterConfig::replication_paths`, including node-scoped consensus log,
  repair progress, and snapshot inbox paths.
- Added durable operator cluster topology persistence through
  `ClusterConfig::store/load` using the `CORTEXDB_CLUSTER_CONFIG_V1` text
  format and atomic replacement.
- Added `open_replication_node_runtime`, which loads durable operator topology,
  recovers committed membership from the node-scoped consensus log, reconciles
  repair progress with current voters, and rejects commit indexes beyond the
  recovered log.
- Hardened replication consensus recovery so recovered runtime state now
  rejects non-contiguous log indexes, zero terms, and commit indexes beyond the
  recovered log.
- Added route-level dashboard navigation: standalone and server-served
  dashboard views now deep-link through `#/overview`, `#/cells`, `#/search`,
  and related routes with document-title updates and back/forward support.
- Split replication consensus-log durability from local WAL durability by adding
  `ConsensusLogOptions` / `ConsensusLogDurability` and mapping consensus logs
  to strict storage WAL fsync internally.
- Added `RecoveredConsensusLog` / `ReplicationLog::recover_log_state` so
  restart code can use validated term/index replay boundaries and reject term
  regression before appending new replicated entries.
- Added `ConsensusState::apply_replayed_entry`, making crash/restart replay
  idempotent for already-present entries and fail-closed on gaps, duplicate
  conflicts, zero terms, or term regression.
- Added `make replication-partition-check` and CI evidence upload for explicit
  split-brain, partition matrix, and rejoin-repair regression coverage.
- Added `make replication-lifecycle-check` and CI evidence upload for snapshot
  transfer, peer resync, durable repair progress, and membership rotation
  lifecycle coverage.
- Added field weights tests: `title_field_weights_six_times_body`,
  `source_field_weights_same_as_body`.
- Added RRF (Reciprocal Rank Fusion) tests: overlap boost, empty lexical/vector fallback,
  limit respect after fusion.
- Made ingestion jobs fully persistent via HTTP: all `POST /v1/ingest/*` endpoints now
  create tracked jobs, `GET /v1/ingest/jobs` lists all jobs, `IngestResponse` includes
  `job_id`. Added `seed_next_id_from_disk()` to prevent ID collisions across restarts.
- Verified ContextPack benchmark: 1K cells = ~15ms, 10K cells = ~1s.
- SDK tenant/auth support confirmed complete (Python/TS/Rust already supported).

### P3 (Big Epics)

- **Observability v0**: Actor queue depth tracking (AtomicUsize), request latency
  counters (count + duration_ms_total), tracing subscriber with `RUST_LOG` support,
  `tracing::info_span` per HTTP request. Metrics endpoint returns 17 fields including
  actor + request metrics in both JSON and Prometheus formats.
- **Agent Memory v1**: `Database::forget_cell()` soft-deletes cells, `POST /v1/forget`
  HTTP endpoint, CLI `cortexdb forget` command, background TTL scheduler (every 60s)
  via `ActorCommand::ExpireMemory` + `DatabaseActor::expire_memory()`.
- **Typed Knowledge Model v1**: `KnowledgeCellType` implements `std::str::FromStr`,
  cell type validation rejects unknown types, default cell_type fixed to `"raw"`,
  typed body parsers (`FactBody`, `EntityBody`, `RelationBody`),
  `Database::ingest_entity()` and `Database::ingest_relation()` methods.
- **Knowledge Graph foundation**: `Database::graph_neighbors(entity_name)` traverses
  Relation cells by subject/object, `Database::tool_cells()` queries Tool cells,
  `GraphEdge` and `ToolCell` structs.

### Earlier

- Added engine-level `NumericValue` parser with magnitude/unit/currency support for verification.
- Refactored verification guards and server memory layer to delegate numeric formatting to engine.
- Added snapshot/golden API response tests for all public endpoints (health, stats, cell, context,
  verify, search, remember, ingest, flush, compact, error shapes).
- Added Search Quality v1 tests: unicode tokenizer (English, Russian, Kazakh), BM25 golden dataset,
  top-k limit, and descending score invariants.
- Added Metadata Model v1 validation with `CellMetadata::validate()` and `sanitized()` for stable
  decode and graceful degradation (scope path-traversal guard, empty field defaults).
- Added durable ingestion job tests: `save_ingestion_job`, `load_ingestion_job`,
  `list_ingestion_jobs`, and `IngestionProgressTracker` lifecycle coverage.
- Synced `docs/API_JSON_SCHEMAS.md` and `docs/openapi.yaml` with actual response shapes and
  error response codes.
- Added CLI integration tests with `assert_cmd`.

- Added AQL parser, binder, policy validation, and mock bitmap VM.
- Added ACLOG WAL v0 codec, reader recovery scan, and writer actor.
- Added in-memory MVCC MemTable and manifest skeleton.
- Added statement-level binding, bound plan variants, catalog facade traits, parser diagnostics,
  and bitmap bytecode explain output.
- Added `cortex-engine` usable single-node database loop with WAL replay and MemTable reads.
- Added durable operation commit sequence, AQL retrieve over engine candidates, initial segment/index
  file foundations, and a minimal CLI.
- Added manifest-backed checkpoint/recovery, engine flush to `.acs/.acb/.aci`, engine-backed AQL
  retrieve, MVP BM25/vector/HNSW helpers, minimal distributed placement, and `cortex-server`.
- Added incremental checkpoint tombstone markers, atomic manifest writes, persisted-index-first
  AQL retrieval with delta overlay, compaction, graph-backed HNSW-style search, JSON/auth server
  responses, CLI compact, and deterministic quorum log primitives.
- Added core-completion invariants: compact bitmap candidates with full `CellId` mapping, atomic
  segment/index writes, CRC corruption checks for core storage files, WAL restart-after-checkpoint
  tests, and the core completion checklist.
- Added tombstone-only checkpoint handling, MemTable stats, database storage validation/stats,
  CLI `stats`/`validate`, and tests for validation failures.
- Fixed persisted index merging across multiple checkpoint segments, added fallible candidate
  allocation, reverse candidate maps, database lock/drop shutdown, AgentView-aware runtime masks,
  and HTTP `stats`/`validate`.
- Added orphan temp cleanup on open, explicit `Database::close`, stronger storage validation,
  lifecycle/validation tests, and Core Alpha invariant documentation.
- Added unique atomic temp filenames, collect-all storage validation reports, explicit stale lock
  recovery, CLI `unlock --force`, and tests for those lifecycle paths.

## v0.1.0-core-alpha — Release Evidence

### Benchmark Matrix (release profile, local workstation)

```
put_1k_cells:                   373.070625ms
get_1k_cells:                   280.122µs
checkpoint_1k:                  18.904873ms
restart_replay_1k:              1.313963ms
compact_1k:                     16.205037ms
aql_retrieve_1k:                5.884727ms
context_pack_1k:                20.111004ms
batch_put_1k_cells:             3.316253ms
batch_put_10k_cells:            24.995694ms
checkpoint_10k:                 100.958304ms
compact_10k:                    79.051873ms
aql_retrieve_10k:               46.651475ms
context_pack_10k:               1.144221087s
```

### Demo Output (investment_projects fixture)

- ContextPack retrieval: 2 cells, 126 estimated tokens, 1000 token budget, truncated=false.
- VERIFY FACT "Solar Plant budget is 1.2B KZT": verdict=`mixed_evidence`, 1 supporting cell,
  1 contradicting cell, numeric conflict detected (`1.2B KZT` vs `1.4B KZT`).
- Storage validation: 1 live segment, 3 cells checked, WAL integrity ok.

### Test Matrix

- Workspace unit tests: 432 passed (all-features).
- CI gates: `cargo check`, `cargo test`, `cargo fmt --check`, `cargo clippy -D warnings`,
  `sdk-check`, `cargo bench`, demo script — all green.
