# Post-Core Alpha Implementation Plan — Current Status

Дата фиксации: 2026-05-31

Цель: не просто добавить фичи, а стабильно довести четыре крупных слоя после Core Alpha.

## Что уже закрыто (в Core Alpha readiness sense)

- Core Alpha pipeline: WAL + MVCC + checkpoint/compact + restart + validation.
- AQL compiler и Runtime retrieval (bitmap + persisted индексы).
- ContextPack v1.
- Search foundation: keyword + vector exact + ANN/HNSW fallback.
- REST API (v1), OpenAPI, typed response contracts.
- CLI + SDK (Rust/Python/TypeScript) contracts, dry-runs, and smoke checks.
- HTTP safety controls: static and file-backed bearer token policies,
  per-token AgentView binding, bounded actor backpressure, rate limit, and audit
  sink.
- Replication + cluster primitives присутствуют на уровне модулей (эпические тесты для лидера/логов/репликации уже покрывают базовую механику).

## Что осталось по крупным эпикам (большие TODO)

### 1) Production-grade ANN/HNSW (высокий приоритет)
- [x] Конфигурируемый recall-policy guard для production: `require_slo`, exact fallback policy, `production_safe`, `slo_violations`.
- [x] Добавлены полиси-поля ANN (fbackoff, fallback + limits), visited-count и budget-guard (`max_visited_candidates`).
- [x] Порог `MIN_ANN_RECALL_Q16` и fallback в exact.
- [x] Golden-фикстуры для recall guard и baseline benchmark hooks.
- [x] Repeatable recall/latency JSON report (`ann_repeatable_report_json`) for benchmark archival.
- [x] Release-mode ANN fixture gate (`make ann-fixture-check`) comparing synthetic recall, graph shape, upper-layer shape, latency ceilings, and production safety against a checked-in baseline.
- [x] ANN fixture report artifact (`make ann-fixture-report`, `target/ann/ann_fixture_report.json`) uploaded by CI for commit-to-commit drift inspection.
- [x] ANN drift check (`make ann-drift-check`) compares current synthetic recall, graph shape, upper-layer shape, and latency against `ann_drift_baseline_v1.json`.
- [x] External ANN JSONL fixture gate (`make ann-external-check`) validates explicit vectors and named queries against exact top-k.
- [x] Deterministic multi-layer graph persisted in `.ach` upper-layer trailer.
- [x] Metric matrix gate (`make ann-metric-matrix-check`) evaluates dot/cosine/L2 against exact top-k on the fixed JSONL fixture.
- [x] External-corpus harness (`ann_corpus_check`) accepts vectors, queries, and ground-truth JSONL files for larger recall suites.
- [x] Наблюдаемость (словарь метрик / отчёт по деградации графа) через расширенный `AnnSearchReport`.
- [x] Profile-aware durable HNSW construction through `DatabaseOptions::hnsw_build_config` for checkpoint/compact `.ach` graphs.
- [x] Long-running ANN history gates fail closed when no archived run/corpus
  exists and fail on recall, graph-shape, latency, or production-safety
  regression (`make ann-history-regression-check`,
  `make ann-real-embedding-history-regression-check`).
- [x] Real-embedding baseline packages now include embedding preflight/export
  provenance, reject synthetic `hash-smoke` metadata, and require hosted
  source-archive SHA-256 when publishing a baseline from GitHub Actions.
- [x] Collection-level vector metadata is persisted in the manifest as
  `vector_profile` (`dimension`, `metric`) and validation rejects live `.acv`
  / `.ach` bundles that drift from that collection profile.
- [x] `ef_construction` is now persisted and reported as a first-class HNSW
  build/profile knob for checkpoint/compact graphs and ANN corpus runs.
- [x] Real-embedding readiness now has a machine-readable non-secret report
  (`make ann-real-embedding-readiness`) that records whether corpus, queries,
  endpoint/model env, optional API key, and source archive metadata are present
  before a real-domain baseline run.

### 2) Real distributed consensus
- [x] Отделить репликационный consensus-log и local WAL по строгим durability guarantees.
- [x] Идемпотентный replay/apply с корректной синхронизацией term/index
  across crash/restart: duplicate recovered entries are no-ops, while gaps,
  duplicate conflicts, zero terms, and term regression fail closed.
- [x] Полный snapshot transfer + peer resync + membership lifecycle
  (join/leave/rotation): `make replication-lifecycle-check` runs chunked
  snapshot transfer, durable peer install, snapshot fault handling,
  database-backed repair snapshots, durable progress reconciliation,
  topology/runtime startup, and joint/stable membership rotation resume suites
  with uploaded JSON/log evidence.
- [x] Split-brain + partition matrix тесты в CI: `make
  replication-partition-check` runs failure injection, five-node partition
  matrix, stale-leader rejection, and rejoin-repair suites with uploaded JSON/log
  evidence.
- [x] Базовая документированная модель Raft-like и внутренние модули протокола уже есть.
- [x] Есть первичные тесты `election/append/log/transport`.
- [x] Начальный failure-injection integration harness покрывает minority
  partition, stale leader rejection after healed majority, and idempotent
  replication-log replay after restart.
- [x] Chunked snapshot resync теперь проверяет contiguous chunks, consistent
  leader/term metadata, durable follower install, and stale-state replacement.
- [x] Membership lifecycle получил начальный reconfiguration API and tests for
  join/leave majority counting and invalid local-node removal.
- [x] In-memory transport получил partition-aware links and a five-node
  partition matrix covering minority writes, healed quorum, majority election,
  and stale minority leader rejection.
- [x] TCP snapshot transport smoke path now streams multi-chunk
  `SnapshotSegment` payloads and rejects a non-zero first chunk.
- [x] Durable peer snapshot install: a `ReplicationPeerServer` can receive TCP
  snapshot chunks and install the decoded `SnapshotSegment` into follower
  database storage, replacing stale local state across restart.
- [x] Peer snapshot fault coverage now proves partial transfers, stale chunks,
  and corrupt final chunks do not replace durable follower state.
- [x] Membership rotation can now be encoded as replicated log entries and
  recovered from `replication.aclog` using only committed config entries.
- [x] Joint-consensus membership safety primitive: joint config entries encode
  old/new voter sets and commit only after majorities from both sets.
- [x] Automated membership rotation API: catches up voters, appends durable
  joint/stable membership entries, and only publishes the final voter set after
  joint-consensus commit.
- [x] Membership rotation restart resume: a leader recovered with a committed
  joint config can finish the stable config phase after restart, while
  uncommitted joint configs cannot be resumed.
- [x] Crash/restart partition matrix seed coverage: partitioned leader restart
  preserves uncommitted entries without committing them, and committed leader
  restart catches up healed followers with the recovered commit index.
- [x] Node rejoin repair primitive: `repair_lagging_voter` catches up small lag
  with missing `AppendEntries` and selects snapshot install when lag exceeds
  the configured threshold.
- [x] Node rejoin repair sweep: `repair_lagging_voters` walks current voters,
  reports caught-up/repaired/snapshot-required followers, and ignores non-voter
  progress inputs.
- [x] Progress-aware repair scheduling: `plan_replication_repair_sweep`
  classifies voters from durable follower progress, rejects inconsistent
  progress, and separates safe planning from network mutation.
- [x] One-shot repair cycle execution: `run_replication_repair_cycle` executes
  append-repair decisions and returns snapshot-required followers as explicit
  handoff requests.
- [x] Snapshot repair sender: `send_replication_snapshot_request` chunks
  `SnapshotSegment` payloads, requires cumulative follower ACKs, and drives TCP
  peer durable install.
- [x] Repair worker loop: `run_replication_repair_worker` ties durable follower
  progress, append repair, snapshot repair sends, and pending-snapshot handoff
  into bounded ticks suitable for a future background task.
- [x] Background repair task: `spawn_replication_repair_background_task` owns
  transport/progress/snapshot sources in a stoppable OS-thread loop with
  finite-run policies for deterministic CI coverage.
- [x] Database-backed repair snapshots: `Database::replication_snapshot_segment`
  and `ReplicationDatabaseSnapshotSource` let the background repair runtime
  source `SnapshotSegment` payloads from current storage state.
- [x] Durable follower progress source: `ReplicationFollowerProgressStore`
  persists follower commit/observed indexes atomically and
  `ReplicationStoredProgressSource` feeds that state into the repair worker
  after restart.
- [x] ACK-driven progress recording: `ReplicationProgressRecordingTransport`
  persists successful AppendEntries ACKs and final snapshot ACKs into the
  follower progress store.
- [x] Default progress-recording background runtime:
  `spawn_replication_repair_background_task_with_progress_store` wires one
  durable progress store into both repair planning and successful ACK recording.
- [x] Membership-aware durable repair progress reconciliation:
  `ReplicationFollowerProgressStore` can now sync its contents with stable,
  joint, or current consensus voter sets, pruning retired peers and seeding
  newly joined voters for explicit repair.
- [x] Operator-facing replication path placement:
  `ClusterConfig::replication_paths` validates local node identity and provides
  node-scoped paths for consensus log, repair progress, and snapshot inbox
  files.
- [x] Durable operator cluster topology config:
  `ClusterConfig::store/load` persist `CORTEXDB_CLUSTER_CONFIG_V1` with
  validated local-node identity, peer addresses, and replication factor.
- [x] Durable topology startup/reload:
  `open_replication_node_runtime` loads the operator topology, recovers
  committed membership from the node-scoped consensus log, reconciles durable
  repair progress with the recovered voter set, and rejects commit indexes
  beyond the recovered log.
- [x] Consensus recovery shape validation:
  `recover_consensus*` rejects non-contiguous log indexes, zero terms, and
  commit indexes beyond the recovered log before publishing recovered state.
- [x] Consensus durability API split:
  `ReplicationLog` now exposes `ConsensusLogOptions` /
  `ConsensusLogDurability` instead of the local WAL durability enum, mapping to
  strict storage WAL fsync internally.
- [x] Durable term/index replay summary:
  `recover_log_state` publishes validated `current_term`, `last_log_index`,
  `last_log_term`, and `next_log_index` boundaries and rejects term regression
  before restart code can append new entries.

### 3) Full web UI (не embedded HTML only)
- [x] Вынести dashboard из Rust string modules в versioned static assets under `crates/cortex-server/assets/dashboard/v1`.
- [x] Завести frontend source-of-truth under `web/dashboard/src` plus `make dashboard-build` / `make dashboard-check`.
- [x] Завести standalone static build artifact under `web/dashboard/dist`, independent from the server crate asset copy.
- [ ] Превратить standalone static build в полноценный frontend-продукт с
  page-specific workflows, product-grade auth UX, and broader visual regression
  coverage.
- [x] Начальный route-level shell: dashboard views now deep-link through
  `/dashboard/overview`, `/dashboard/cells`, `/dashboard/search`,
  `/dashboard/ann-eval`, `/dashboard/aql`, `/dashboard/context`,
  `/dashboard/verify`, `/dashboard/ingest`, `/dashboard/storage`, and
  `/dashboard/cluster` with document-title
  updates and browser back/forward behavior.
- [x] Standalone build now emits per-route HTML entrypoints under
  `web/dashboard/dist/dashboard/<route>/index.html`, so copied dashboard links
  work against static hosting as well as the server route.
- [x] Standalone dashboard release pipeline now packages
  `web/dashboard/dist` into `target/dashboard/dashboard-v1.tar.gz` with a
  checked `package_manifest.json`, file sizes, SHA-256 checksums, CI upload,
  and `make dashboard-release-check`.
- [x] Standalone dashboard release now carries
  `dashboard_manifest.json`, fixing the Core Alpha frontend stack
  (`dependency-free-static-html-css-js`), release channel, asset root,
  route IDs, and route entrypoints as a machine-checkable contract.
- [x] Dashboard session UX now has a checked memory-only bearer token policy,
  tenant-only `sessionStorage` persistence, and a clear-session control covered
  by Playwright smoke.
- [x] Dashboard Playwright smoke now exercises page-specific workflows for AQL,
  ContextPack, Verify, Ingest, and ANN evaluation in addition to Cells, Search,
  Storage, and Cluster.
- [x] Наборы views в текущем static dashboard: overview, cells, search/explain, ANN, AQL, context, verify, ingest, storage health, cluster status.
- [x] Базовый tenant/token control в UI для scoped API calls; полноценный auth UX остаётся будущей standalone UI задачей.
- [x] Playwright/CI smoke путь для текущего `/dashboard`: asset loading, route navigation, cell put/get, keyword search.
- [x] Playwright smoke дополнен search explain, storage validation, and cluster status.
- [x] Playwright screenshot artifacts for desktop/mobile dashboard review in CI.
- [x] Есть минимальный dashboard, static asset routes, and accessibility smoke tests.

### 4) Stable SDK package lifecycle
- [x] Полный release-процедурный lock-step по версиям `server <-> OpenAPI <-> SDK`.
- [x] Локальные и tag-gated package preflight checks for crates.io / npm / PyPI artifacts.
- [ ] Публичная публикация в crates.io / npm / PyPI remains beta-stage and depends on registry credentials plus release-train ownership.
- [x] Deprecation policy и changelog для breaking changes в SDK contract.
- [x] Подготовлены release workflows и базовые contract checks (локально, в repo).

## Непосредственный следующий 2-недельный sprint

1. ANN/HNSW: keep the endpoint-backed `investment-projects-v1` real-domain
   baseline local-only for Core Alpha; GitHub-hosted promotion is deferred until
   beta to avoid provider-secret and scheduled-spend risk.
2. ANN/HNSW: откалибровать SLO thresholds на повторных real-domain baseline
   прогонах в стабильной среде.
3. Consensus: keep expanding crash/restart coverage around node rejoin repair
   and durable snapshot handoff.
4. UI: начать multi-page standalone app после текущих dashboard screenshot artifacts.
5. SDK: перейти к следующему продуктному слою после закрытия release/deprecation gates.

## Критерий перехода к следующему слою

- `cargo check --workspace`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Все contract snapshots не меняются без `API/SDK` change-log заметки.
