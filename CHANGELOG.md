# Changelog

## Unreleased

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
