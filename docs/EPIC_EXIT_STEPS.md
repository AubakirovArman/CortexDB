# CortexDB Epic Exit Steps

Source plan: `/mnt/hf_model_weights/arman/3bit/sites/pl copy.md`.
Tracker: `docs/DATABASE_GRADE_EXECUTION_PLAN.md`.

Purpose: keep execution moving in epic order. Each epic gets a short exit path:
what to inspect, what to change, what evidence is enough, and when to move on.
This file is not a replacement for the detailed tracker; it is the guardrail
that prevents one epic from expanding forever.

Global rules:

1. Work in the current pointer order from `docs/DATABASE_GRADE_EXECUTION_PLAN.md`.
2. For the active epic, finish only the acceptance criteria listed there.
3. Add focused regression tests for behavior changes.
4. Run the relevant code gates before marking an epic done.
5. Do not run the 50-question EnterpriseRAG impact gate unless explicitly asked.
6. Update the tracker with evidence, remaining work, and next pointer.
7. Commit and push when the epic slice is coherent and checks pass.

Minimum gates unless the epic says otherwise:

- `cargo fmt --check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make check`
- `make openapi-contract-check` only when API/OpenAPI output changes

## Block A — Earn the word database

### EPIC-A01 — Чистый репозиторий и базовая воспроизводимость

Exit steps:
1. Audit `git status`, recent commits, and untracked files.
2. Split unrelated work into focused commits or document why no split is needed.
3. Run workspace fmt/test/clippy gates.
4. Mark done only when the branch is clean and reproducible; then move to A03/A02 per dependency order.

### EPIC-A02 — Типизированная модель метаданных

Exit steps:
1. Find remaining hot paths that parse payload metadata for scope/trust/date/type decisions.
2. Replace each production decision path with descriptor-backed metadata and add spoofed payload-vs-descriptor regression tests.
3. Extend the descriptor hot-path/static gate to cover every changed path.
4. Run targeted tests plus workspace gates.
5. Mark done only when descriptor is the permission source of truth and profile/static evidence covers hot paths; then move to A10.

### EPIC-A03 — DATA_MODEL.md

Exit steps:
1. Verify every descriptor field and MVCC/tombstone/TTL concept is documented.
2. Link README and metadata docs to the canonical model.
3. Keep examples aligned with current API names.
4. Mark done when a client author can understand the model without Rust code; then move to C12/A04.

### EPIC-A04 — MemTable-итераторы без клонирования

Exit steps:
1. Classify MemTable clone call sites into cold and hot paths.
2. Replace cold reads with borrowed iterators.
3. Add clone/static regression coverage.
4. Run workspace gates.
5. Mark done when cold paths avoid payload clones and clone gate protects regressions; then move to A05.

### EPIC-A05 — Indexed VERIFY FACT

Exit steps:
1. Route VERIFY through lexical/numeric candidate selection instead of full scan.
2. Preserve existing verdict behavior with fixture tests.
3. Add performance evidence for 100K/1M steady-state.
4. Gate against `snapshot_versions()` in VERIFY hot paths.
5. Mark done when VERIFY is O(k candidates) and tests/bench evidence is recorded; then move to C16/A19.

### EPIC-A06 — Indexed-only retrieve/ContextPack путь

Exit steps:
1. Inventory query-time full scans and `snapshot_versions()` uses in retrieve/search/ContextPack.
2. Add or reuse incremental delta indexes for uncheckpointed MemTable data.
3. Merge persisted and delta candidates without rebuilding from scratch.
4. Add correctness tests proving indexed results match full rebuild results.
5. Mark done when retrieval hot path has zero full-snapshot scans and p95 retrieve is published; then move to A07.

### EPIC-A07 — Segment format v2

Exit steps:
1. Specify the segment footer/record layout with descriptor, payload offset, length, and block CRC.
2. Implement v2 writer and dual v1/v2 reader.
3. Add random payload read tests and block-corruption tests.
4. Update storage fixtures and compatibility docs.
5. Mark done when one payload can be read without decoding the full segment; then move to A08.

### EPIC-A08 — Lazy payload residency

Exit steps:
1. Introduce `PayloadRef`/equivalent while keeping memory mode as default.
2. Load descriptors/indexes at open and fetch payload on demand.
3. Add cache/config plumbing behind a feature or mode flag.
4. Run crash/restart matrix in memory and lazy modes.
5. Mark done when lazy mode shows measured RSS reduction and passes recovery tests; then move to A09.

### EPIC-A09 — Disk-resident индексы

Exit steps:
1. Measure current persisted index re-merge and memory spikes.
2. Implement incremental merge/tombstone handling for new segments.
3. Add equivalence tests: incremental index equals full rebuild.
4. Record checkpoint/search latency before and after.
5. Mark done when checkpoint no longer triggers full re-merge; then move to A10/A12.

### EPIC-A10 — LogicalPlan IR + Policy Rewrite

Exit steps:
1. Define LogicalPlan nodes for scan/filter/rank/pack/verify.
2. Move AgentView policy insertion into a formal rewrite stage.
3. Add tests proving policy predicates are always present before physical planning.
4. Keep existing AQL behavior stable.
5. Mark done when plans can be inspected before/after policy rewrite; then move to A11.

### EPIC-A11 — Operator-based executor

Exit steps:
1. Define the executor operator trait and minimal operator set.
2. Port one read path through operator execution without changing public output.
3. Add execution trace tests.
4. Compare old and new path outputs.
5. Mark done when retrieve/pack can run through operators; then move to B02/B08.

### EPIC-A12 — Статистика хранилища

Exit steps:
1. Define per-segment stats: df, cardinality, zone maps, payload bytes, index sizes.
2. Persist stats in manifest or segment metadata.
3. Expose stats through storage stats/reporting.
4. Add validation tests for stats consistency.
5. Mark done when planner can read reliable stats; then move to A13.

### EPIC-A13 — Cost model v0

Exit steps:
1. Define simple costs for lexical/vector/hybrid/pack paths.
2. Feed the model with A12 statistics.
3. Add deterministic plan-choice tests.
4. Add explain output for selected path and reason.
5. Mark done when the engine can choose a path from stats instead of hardcoding; then move to A11/C07.

### EPIC-A14 — Snapshot pinning и GC-барьер

Exit steps:
1. Add explicit read-transaction pin lifecycle.
2. Make version GC respect active read pins.
3. Add long-reader vs writer/tombstone tests.
4. Document snapshot isolation boundaries.
5. Mark done when old visible versions cannot be collected under active readers; then move to A16.

### EPIC-A15 — Транзакционный API

Exit steps:
1. Define the atomic multi-cell write API contract.
2. Implement WAL batching and apply atomicity for all cells in the batch.
3. Add crash/replay tests at batch boundaries.
4. Document read-your-writes and failure behavior.
5. Mark done when partial batch visibility is impossible; then move to A16/A17.

### EPIC-A16 — Конкурентный read path

Exit steps:
1. Split read execution from the single actor queue where safe.
2. Use snapshot pins for concurrent readers.
3. Add mixed read/write throughput tests.
4. Preserve write ordering and permission invariants.
5. Add route-classification regression coverage so mutating endpoints always take write locks.
6. Mark done when reads do not block behind unrelated slow reads and queued writers cannot starve; then move to E15.

### EPIC-A17 — Checkpoint без stop-the-world

Exit steps:
1. Define WAL rotation/checkpoint boundary semantics.
2. Implement checkpoint without shutting down the writer.
3. Add crash tests across rotation, segment write, and manifest publish.
4. Record write latency during checkpoint.
5. Mark WAL-rotation safety done when the writer is not shutdown/restarted and recovery handles rotated WAL files.
6. Move route-level fully concurrent checkpoint build to A18 unless a two-phase checkpoint API is added here.

### EPIC-A18 — Фоновая инкрементальная компакция

Exit steps:
1. Define compaction triggers and segment selection policy.
2. Implement background compaction under snapshot/manifest safety.
3. Add retired-segment GC tests.
4. Record latency and disk-space behavior.
5. Mark done when compaction runs incrementally without corrupting visible snapshots; then move to E11.

### EPIC-A19 — Scale-бенчмарки

Exit steps:
1. Keep reproducible benchmark commands for 100K/1M/10M targets.
2. Record RAM, open time, put/get/retrieve/verify/checkpoint latency.
3. Store reports in a stable target/docs path.
4. Compare each storage/indexing change against the baseline.
5. Mark done when the scale envelope is measured and documented; then move to C17/C20.

### EPIC-A20 — Property-based тесты ядра

Exit steps:
1. Cover MVCC, WAL replay, recovery, index equivalence, and permissions.
2. Keep generated cases deterministic enough for CI.
3. Add regression seeds for found bugs.
4. Run full workspace gates.
5. Mark done when core invariants have property coverage and no flaky failures; then move to A02/A06.

## Block B — Agent-native database primitives

### EPIC-B01 — ContextPack JSON Schema v1

Exit steps:
1. Freeze schema fields and versioning rules.
2. Add JSON schema/snapshot contract tests.
3. Update server/CLI/SDK examples.
4. Mark done when schema changes require explicit version bump; then move to B02.

### EPIC-B02 — ContextPackBuilder как физический оператор

Exit steps:
1. Move pack assembly behind executor/operator boundaries.
2. Preserve current ContextPack output semantics.
3. Add operator trace/explain tests.
4. Mark done when pack building is an execution operator, not post-processing; then move to B03.

### EPIC-B03 — Token-budget pushdown

Exit steps:
1. Push token budget into retrieval/rank/pack planning.
2. Stop reading low-value payload after budget is satisfied.
3. Add tests for deterministic early termination.
4. Mark done when token budget reduces work before payload materialization; then move to F07.

### EPIC-B04 — AgentView permission bitmap

Exit steps:
1. Build permission bitmaps from descriptor scopes.
2. Intersect permission bitmap at scan time.
3. Add spoofed payload and all-surface permission tests.
4. Mark done when unreadable candidate bytes are not read; then move to E09.

### EPIC-B05 — AgentView lifecycle API v1

Exit steps:
1. Define create/read/update/delete lifecycle for AgentView.
2. Persist view metadata safely.
3. Add server/CLI/API tests.
4. Mark done when AgentView is durable and inspectable; then move to E08.

### EPIC-B06 — Typed provenance model

Exit steps:
1. Move source_ref, citation, content_hash, and source trust into typed descriptor/provenance fields.
2. Preserve legacy payload compatibility.
3. Add provenance validation/export tests.
4. Mark done when ContextPack provenance no longer depends on payload headers; then move to B14.

### EPIC-B07 — Fact/claim store

Exit steps:
1. Define typed claim/fact records with numeric values and units.
2. Store facts in descriptor/body structures that can be indexed.
3. Add parsing and equality/conflict tests.
4. Mark done when the maintained typed store feeds VERIFY with parser fallback
   retained; the metric-sorted numeric index remains C13.
5. Then move to B08.

### EPIC-B08 — VerifyOp

Exit steps:
1. Represent verification as a logical and physical operator.
2. Reuse indexed candidates and typed facts.
3. Preserve existing VERIFY outputs.
4. Add EXPLAIN/trace coverage for VERIFY stages.
5. Mark done when VERIFY participates in plans/explain; then move to B09/B15
   by dependency.

### EPIC-B09 — Инкрементальный contradiction/conflict-индекс

Exit steps:
1. Define contradiction index keys and update rules.
2. Update the index on put/patch/tombstone.
3. Add equivalence tests against full rebuild.
4. Mark done when conflict lookup is incremental and candidate-based; then move to B08/B14.

### EPIC-B10 — Temporal validity

Exit steps:
1. Promote valid_from/valid_to into typed fields.
2. Add temporal query predicates and index support.
3. Add stale/fresh/conflict tests.
4. Mark done when temporal filters avoid payload parsing; then move to C14.

### EPIC-B11 — Memory lifecycle

Exit steps:
1. Define TTL/decay/retention policy as storage behavior.
2. Apply lifecycle during query and maintenance.
3. Add expiration/decay tests.
4. Mark done when memory lifecycle is deterministic and documented; then move to B12/B13.

### EPIC-B12 — Session/episodic memory contract

Exit steps:
1. Define durable session memory semantics.
2. Enforce permissions from descriptor scope.
3. Add session restart/TTL/scope tests.
4. Mark done when session retrieval is descriptor-safe and documented; then move to B13.

### EPIC-B13 — Feedback ranking signal

Exit steps:
1. Define feedback records and ranking impact.
2. Index feedback by target cell/query context.
3. Add tests proving feedback affects rank without bypassing relevance/permissions.
4. Mark done when feedback is an indexed ranking signal; then move to B14.

### EPIC-B14 — Explainability contract

Exit steps:
1. Define explain fields for each result and exclusion.
2. Thread explain through retrieve/search/context/verify.
3. Add snapshot tests for stable explain output.
4. Mark done when each result can say why it was selected or excluded; then move to B15.

### EPIC-B15 — EXPLAIN ANALYZE

Exit steps:
1. Add runtime counters per operator/stage.
2. Expose elapsed time, candidates, filtered counts, payload reads, and token budget.
3. Add CLI/server tests.
4. Mark done when AQL can report actual execution metrics; then move to B17.

### EPIC-B16 — Policy Rewrite proof

Exit steps:
1. Formalize policy rewrite invariants.
2. Add tests/property tests over generated plans.
3. Verify all physical scans carry policy constraints and direct read surfaces
   delegate descriptor/scope reads to `PolicyRewrite`.
4. Add/update the static gate that rejects production read-policy bypasses.
5. Mark done when policy safety is proven at plan level and E09 property tests
   pass; then move to C02 per the corrected dependency-stage roadmap.

### EPIC-B17 — Tool registry

Exit steps:
1. Define typed tool descriptor storage.
2. Index permissions/risk/schema fields.
3. Add lookup/recommendation tests.
4. Mark done when tools are catalog entries, not payload conventions; then move to B18.

### EPIC-B18 — Knowledge graph/provenance index

Exit steps:
1. Define graph edge keys and update rules.
2. Maintain the graph incrementally.
3. Add equivalence tests against rebuild.
4. Mark done when graph/provenance lookup avoids full scans; then move to B14/C15.

### EPIC-B19 — REMEMBER write-path policy

Exit steps:
1. Define REMEMBER permission/type/TTL rules.
2. Enforce rules before WAL append.
3. Add policy denial and replay tests.
4. Mark done when REMEMBER cannot write outside AgentView policy; then move to B05/B11.

### EPIC-B20 — Multi-brain

Exit steps:
1. Decide whether BrainId is real product scope or removed/simplified.
2. If removed/simplified, document `default = BrainId(1)` and deprecated alias
   migration.
3. Add tests/gates proving current AQL catalog behavior matches the decision.
4. Mark done when the semantics are explicit; then move to C01.

## Block C — Indexing, retrieval, and performance

### EPIC-C01 — Интернирование термов

Exit steps:
1. Add term dictionary/interning design.
2. Migrate lexical index postings to compact term IDs.
3. Add compatibility and memory tests.
4. Mark done when term memory drops and search output is stable; then move to C03.

### EPIC-C02 — Roaring bitmaps

Exit steps:
1. Replace BTreeSet postings/bitmap internals with roaring-compatible structures.
2. Preserve bitmap VM semantics.
3. Add equivalence tests and memory benchmarks.
4. Mark done when intersections are faster and outputs match; then move to A09/C03.

### EPIC-C03 — Честный BM25

Exit steps:
1. Implement field-aware BM25 with IDF, TF, and length normalization.
2. Replace fake term-count ranking in retrieval paths.
3. Add golden ranking tests.
4. Mark done when lexical rank is BM25-backed and explainable; then move to C04.

### EPIC-C04 — Токенизация

Exit steps:
1. Define tokenizer contract for Unicode and optional stemming.
2. Update index and query tokenization consistently.
3. Add RU/KZ/EN and punctuation tests.
4. Mark done when tokenizer changes are backward-compatible or migrated; then move to C05.

### EPIC-C05 — Disk-resident vector storage

Exit steps:
1. Define vector segment/storage format and dimension checks.
2. Implement exact scan over disk-resident vectors.
3. Add persistence and corruption tests.
4. Mark done when vectors can exceed RAM-bound metadata assumptions; then move to C06/C07.

### EPIC-C06 — HNSW guarded productization

Exit steps:
1. Define recall/latency gates and fallback policy.
2. Run repeatable recall reports against exact scan.
3. Add observability and no-fallback guardrails.
4. Mark done when HNSW promotion is evidence-gated; then move to C07.

### EPIC-C07 — Hybrid retrieval

Exit steps:
1. Integrate lexical+dense RRF into engine retrieval, not only scripts.
2. Make weights/explain visible.
3. Add permission and ranking regression tests.
4. Mark done when AQL/ContextPack can use hybrid retrieval safely; then move to F05.

### EPIC-C08 — Server-side embedding integration

Exit steps:
1. Define provider config without storing secrets.
2. Add retries/timeouts/backpressure.
3. Add local mock/provider tests.
4. Mark done when embeddings can run server-side reproducibly; then move to C19.

### EPIC-C09 — Permission-aware index pruning

Exit steps:
1. Add scope/permission pruning before expensive ranking.
2. Prove pruning never changes allowed results.
3. Add property tests with random AgentViews.
4. Mark done when unreadable candidates are pruned at index time; then move to B04/E09.

### EPIC-C10 — Segment zone maps

Exit steps:
1. Define zone map fields per segment.
2. Use maps to skip segments for filters/time ranges/scopes.
3. Add correctness and skip-count explain tests.
4. Mark done when segment skipping is measured; then move to A13.

### EPIC-C11 — AQL query cache

Exit steps:
1. Define cache keys, invalidation, and metrics.
2. Cache parse/bind/plan safely across AgentView boundaries.
3. Add cache hit/miss tests.
4. Mark done when repeated queries reuse safe cached plans; then move to B15.

### EPIC-C12 — Rank key precompute

Exit steps:
1. Find repeated rank-key computation in sort loops.
2. Replace with cached/precomputed keys.
3. Add ranking stability tests.
4. Mark done when sort paths do not recompute expensive metadata; then move to A04/A02.

### EPIC-C13 — Fact/numeric индекс

Exit steps:
1. Define numeric/fact index keys and normalized values.
2. Update index on write/replay/checkpoint.
3. Add numeric query/conflict tests.
4. Mark done when numeric VERIFY uses the index; then move to B07/B08.

### EPIC-C14 — Temporal индекс

Exit steps:
1. Define temporal keys for created/valid ranges.
2. Add range query and stale/fresh lookup support.
3. Add timezone/date edge tests.
4. Mark done when temporal filters are indexed; then move to B14.

### EPIC-C15 — Graph index performance

Exit steps:
1. Measure current graph build/query cost.
2. Incrementally maintain graph edges.
3. Add rebuild-equivalence and performance tests.
4. Mark done when graph query latency is bounded by candidate edges; then move to B18.

### EPIC-C16 — Memory profiling harness

Exit steps:
1. Keep portable RSS/working-set measurements.
2. Decide whether allocator-level profiling dependency is allowed.
3. Add repeatable memory report commands.
4. Mark done when memory regressions are measurable in CI or local gate; then move to A19/C17.

### EPIC-C17 — Перф-регрессии в CI

Exit steps:
1. Select stable microbench gates with low flake risk.
2. Store baselines and thresholds.
3. Add CI/reporting integration.
4. Mark done when regressions are visible without blocking on noisy full benches; then move to A19.

### EPIC-C18 — Concurrent read throughput bench

Exit steps:
1. Define read workloads and concurrency levels.
2. Measure current actor/snapshot behavior.
3. Add reports before/after A16.
4. Mark done when concurrent read throughput is tracked; then move to A16/E15.

### EPIC-C19 — Ingestion throughput + embedding pipeline

Exit steps:
1. Define ingestion and embedding throughput workloads.
2. Add batching/backpressure metrics.
3. Record failures/retries and output quality.
4. Mark done when ingestion throughput is reproducible; then move to C08/D14.

### EPIC-C20 — Baseline-сравнение с наивным стеком

Exit steps:
1. Define fair baseline stack and workloads.
2. Compare retrieval/pack/verify latency and quality.
3. Document honest wins/losses.
4. Mark done when claims are backed by comparable evidence; then move to E03.

## Block D — Developer experience and adoption

### EPIC-D01 — CLI help

Exit steps:
1. Audit CLI commands/help text.
2. Group commands and improve usage output.
3. Add CLI help snapshot/tests.
4. Mark done when a new user can discover commands from `--help`; then move to D02.

### EPIC-D02 — `cortexdb init` + doctor

Exit steps:
1. Define init layout and doctor checks.
2. Implement safe, idempotent commands.
3. Add CLI integration tests.
4. Mark done when a clean environment can initialize and self-check; then move to D03.

### EPIC-D03 — GETTING_STARTED

Exit steps:
1. Keep a five-minute path from install to first ContextPack.
2. Verify commands on a clean local checkout.
3. Link from README.
4. Mark done when quickstart works without hidden setup; then move to D04.

### EPIC-D04 — Flagship demo

Exit steps:
1. Build a demo that exercises permissions and numeric conflict.
2. Make the demo deterministic and documented.
3. Add smoke tests where practical.
4. Mark done when the demo proves the product thesis; then move to D14.

### EPIC-D05 — SDK publishing

Exit steps:
1. Verify package metadata, versions, and generated types.
2. Dry-run local package build/install.
3. Publish only when credentials/tag policy are available.
4. Mark done when packages are available from target registries; then move to D06-D08.

### EPIC-D06 — Python SDK

Exit steps:
1. Add typed models, retries, and timeouts.
2. Test against local server contract.
3. Add examples.
4. Mark done when Python clients can use core APIs safely; then move to D05.

### EPIC-D07 — TypeScript SDK

Exit steps:
1. Add typed request/response models.
2. Test build and local server calls.
3. Add examples and package metadata.
4. Mark done when TS clients can use core APIs safely; then move to D05.

### EPIC-D08 — Async Rust SDK

Exit steps:
1. Define shared API types or client crate boundaries.
2. Add async client with retries/timeouts.
3. Add integration tests against local server.
4. Mark done when Rust client API is stable enough for examples; then move to D05.

### EPIC-D09 — Docker GHCR + compose

Exit steps:
1. Harden Dockerfile and runtime user/volume behavior.
2. Add compose quickstart.
3. Add image build smoke check.
4. Mark done when a fresh user can run server through Docker; then move to D15.

### EPIC-D10 — OpenAPI source of truth

Exit steps:
1. Ensure OpenAPI matches typed server responses.
2. Add contract generation/check command.
3. Connect SDK generation or validation to the contract.
4. Mark done when API schema drift fails a check; then move to D06-D08.

### EPIC-D11 — MCP server adapter

Exit steps:
1. Define MCP operations for put/search/context/verify.
2. Implement adapter without bypassing permissions.
3. Add smoke tests and examples.
4. Mark done when agents can access CortexDB through MCP; then move to D14.

### EPIC-D12 — Документация консолидация

Exit steps:
1. Archive duplicate process docs.
2. Keep only core docs visible at top level.
3. Add a docs index.
4. Mark done when docs are navigable and claims are consistent; then move to D13.

### EPIC-D13 — mdBook docs-сайт

Exit steps:
1. Create mdBook structure from core docs.
2. Add build/check command.
3. Fix broken links.
4. Mark done when docs site builds reproducibly; then move to D15.

### EPIC-D14 — Examples

Exit steps:
1. Pick three real integration examples.
2. Keep examples small, runnable, and documented.
3. Add smoke tests or scripts.
4. Mark done when examples demonstrate common workflows; then move to D15.

### EPIC-D15 — v0.2.0-beta release

Exit steps:
1. Align workspace/package versions and release notes.
2. Decide tag strategy without force-moving published tags unless approved.
3. Run release checklist and clean clone checks.
4. Mark done when the beta tag/artifacts are correct; then move to next release scope.

## Block E — Reliability, security, and operations

### EPIC-E01 — WAL writer errors

Exit steps:
1. Audit writer error paths and background channels.
2. Ensure errors are surfaced and stop unsafe writes.
3. Add failure injection tests.
4. Mark done when WAL errors cannot be swallowed silently; then move to E11.

### EPIC-E02 — Backup UX

Exit steps:
1. Define one backup/restore happy path.
2. Add verify/drill command flow.
3. Simplify docs around that path.
4. Mark done when users can backup and verify with one documented flow; then move to E14.

### EPIC-E03 — WAL archive / PITR groundwork

Exit steps:
1. Define archive file naming and retention.
2. Add safe archive hooks around checkpoint/rotation.
3. Add restore-to-seq groundwork tests.
4. Mark done when PITR has durable prerequisites; then move to E05.

### EPIC-E04 — Corruption handling

Exit steps:
1. Define quarantine behavior for corrupt WAL/segments/indexes.
2. Add repair/report UX.
3. Add corruption matrix tests.
4. Mark done when corruption produces actionable reports, not ambiguous crashes; then move to E02/E14.

### EPIC-E05 — Observability

Exit steps:
1. Add tracing spans for route/queue/engine/storage stages.
2. Add Prometheus-compatible metrics.
3. Add docs and smoke tests.
4. Mark done when operators can see latency, queue, WAL, and retrieval metrics; then move to E06/E15.

### EPIC-E06 — Backpressure and tenant limits

Exit steps:
1. Define per-tenant resource limits.
2. Enforce queue/input/ingestion limits.
3. Add overload tests and metrics.
4. Mark done when overload degrades predictably; then move to E15.

### EPIC-E07 — Audit log

Exit steps:
1. Define audit event schema.
2. Emit events for auth, writes, reads, verify, admin actions.
3. Add retention/export tests.
4. Mark done when security-relevant actions are auditable; then move to E13.

### EPIC-E08 — Tenant isolation test suite

Exit steps:
1. Generate multi-tenant/AgentView cases.
2. Test all public surfaces for isolation.
3. Add regression seeds.
4. Mark done when tenant leaks fail tests; then move to E09.

### EPIC-E09 — Permission property suite

Exit steps:
1. Formalize “no bytes outside AgentView” invariant.
2. Generate random scopes, cells, and operations.
3. Test AQL/search/get/verify/context/export surfaces.
4. Mark done when the invariant is property-tested across surfaces; then move to B16/C09.

### EPIC-E10 — Fuzzing decode paths

Exit steps:
1. Select WAL/segment/manifest/index decoders for fuzzing.
2. Add fuzz targets and seed corpus.
3. Document local/nightly run commands.
4. Mark done when malformed bytes cannot panic decode paths; then move to E04.

### EPIC-E11 — Chaos consolidation

Exit steps:
1. Consolidate crash/shutdown/failure simulation harnesses.
2. Add graceful shutdown assertions.
3. Run key crash matrix.
4. Mark done when chaos checks are repeatable; then move to A17/A18.

### EPIC-E12 — Migration framework

Exit steps:
1. Inventory the current compatibility surfaces:
   `cortex_storage::format::storage_format_specs`,
   `fixtures/storage/storage_format_freeze_v1.json`,
   `fixtures/migration/compatibility_matrix_v1.json`,
   `cortexdb migrate`, and the migration gates.
2. Define one migration version registry exposed by the engine/API. It must list
   current storage formats, legacy magics, required gates, current release,
   release-to-release upgrade fixtures, and downgrade policy.
3. Add registry tests proving every frozen format and release path is present.
4. Verify the existing offline migration runner has backup/drill behavior, or add
   missing dry-run/precondition checks before changing status to done.
5. Cover A02/A07/C01/C02 format migrations through fixtures/change notes and
   compatibility gates.
6. Run targeted compatibility tests, `make migration-compatibility-check`,
   `make storage-format-freeze-check`, `make storage-format-change-note-check`,
   and API contract checks if `/v1/compatibility` changes.
7. Update `docs/DATABASE_GRADE_EXECUTION_PLAN.md` with evidence and remaining
   work after each coherent slice.
8. Mark done only when format upgrades are repeatable, fixture-gated, and
   reversible by immutable backup; then move to A06 unless the tracker changes
   the next pointer.

### EPIC-E13 — Secrets-гигиена

Exit steps:
1. Audit repository, docs, scripts, and CI for secrets.
2. Add ignore patterns and secret scan checks.
3. Remove/rotate any exposed values.
4. Mark done when secret scan is clean and docs point to env usage; then move to D15.

### EPIC-E14 — Upgrade/rollback drill

Exit steps:
1. Define supported upgrade/rollback paths.
2. Run restore/migration drill on fixtures.
3. Document failure recovery.
4. Mark done when release candidates pass upgrade/rollback drill; then move to D15.

### EPIC-E15 — Per-route timeouts

Exit steps:
1. Define timeout/budget behavior per route.
2. Enforce slow-client and actor protection.
3. Add timeout tests and metrics.
4. Mark done when slow requests cannot starve the service; then move to D13.

## Block F — Long-term database research

### EPIC-F01 — Tiered storage v2

Exit steps:
1. Define hot/cold placement and page compression policy.
2. Build on A08 lazy payload and cache metrics.
3. Add eviction/readback tests.
4. Mark done when cold data can be served with bounded RAM; then move to F06.

### EPIC-F02 — Распределённая репликация

Exit steps:
1. Keep frozen until single-node database-grade work is stable.
2. Before unfreezing, write a replication design and non-goals.
3. Add log-shipping/snapshot-transfer prototype tests.
4. Mark done only when real replicated durability semantics exist; then move to F03.

### EPIC-F03 — Консенсус/мульти-нод транзакции

Exit steps:
1. Keep frozen until F02 and single-node transaction semantics are mature.
2. Define consensus protocol, failure model, and membership.
3. Add split-brain/rejoin/failover tests.
4. Mark done only with real consensus evidence; then move to production HA scope.

### EPIC-F04 — Agent transaction semantics

Exit steps:
1. Define multi-agent write conflicts and isolation semantics.
2. Add transaction tests for concurrent agent writes.
3. Document conflict outcomes.
4. Mark done when multi-agent writes have deterministic behavior; then move to F08.

### EPIC-F05 — Learned/calibrated ranking

Exit steps:
1. Define offline training/evaluation data without benchmark overfit.
2. Add calibrated ranking behind a feature/config.
3. Compare against deterministic ranking with held-out data.
4. Mark done when learned ranking improves without policy regressions; then move to C07.

### EPIC-F06 — Semantic compression памяти

Exit steps:
1. Define compression contract and loss boundaries.
2. Preserve provenance and answerability metadata.
3. Add quality/regression tests.
4. Mark done when compressed memory remains auditable; then move to F01.

### EPIC-F07 — Query optimization для LLM-контекста

Exit steps:
1. Define “value per token” cost model inputs.
2. Integrate with ContextPack planning.
3. Add tests showing better budget allocation.
4. Mark done when planner optimizes context value, not only document score; then move to A13/B03.

### EPIC-F08 — Multi-agent memory consistency

Exit steps:
1. Define consistency levels for shared/private memory.
2. Add conflict and visibility tests.
3. Document operational tradeoffs.
4. Mark done when multi-agent reads/writes have clear consistency guarantees; then move to F04.

### EPIC-F09 — Cloud/service mode

Exit steps:
1. Keep frozen until single-node operations, quotas, and security are stable.
2. Define managed-service boundaries and control plane.
3. Add deployment/security evidence.
4. Mark done only when cloud mode has real operational guarantees; then move to enterprise scope.

### EPIC-F10 — Формальная верификация инвариантов

Exit steps:
1. Pick core invariants for TLA+/stateright modeling.
2. Model WAL/recovery/MVCC/permission rewrite at useful abstraction.
3. Connect model findings to tests.
4. Mark done when at least one critical invariant has machine-checked evidence; then move to deeper database research.
