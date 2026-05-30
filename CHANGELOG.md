# Changelog

## Unreleased

### P1 (Alpha Polish)

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
- Added file-backed HTTP token rotation through `CORTEXDB_AUTH_TOKENS_FILE`;
  the server re-reads the local policy file per request and fails closed on
  missing, empty, or invalid token files.
- Hardened SDK release lifecycle checks: protected `sdk-release` deployment
  environment, Node.js 24 workflow runtime, and tag/version lock-step
  enforcement before public publish jobs.
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
- Added durable membership rotation primitives in the replication log:
  `MembershipConfig`, `membership_entry`, and committed membership recovery.
- Added joint-consensus membership safety primitives: joint entries preserve old
  and new voter sets, and commit requires majorities from both sets.
- Added crash/restart partition seed coverage for replicated logs: uncommitted
  partitioned writes stay uncommitted after restart until heal, and committed
  restarted leaders catch up healed followers.
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
