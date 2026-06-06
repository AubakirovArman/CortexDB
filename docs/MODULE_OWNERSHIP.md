# Module Ownership

Status: Epic 30 module ownership boundary map.

This document is the ownership contract for CortexDB's current Core Alpha code
shape. It separates stable facades from internal implementation modules so new
work does not accidentally promote implementation details into public API,
server, CLI, or SDK promises.

## Stable Facades

| Facade | Primary owner | Compatibility rule |
| --- | --- | --- |
| `cortex_engine::*` crate-root exports | Engine facade and compatibility | Keep `examples/engine_api_compat` compiling, update `docs/ENGINE_API.md`, update freeze fixtures, and run `make engine-api-check`. |
| `cortex-server` HTTP routes | Server API and typed contract | Update `docs/openapi.yaml`, `docs/API_JSON_SCHEMAS.md`, and run `make openapi-contract-check`. |
| `cortex-cli` commands | Local operator UX | Preserve human and `--json` output contracts or add explicit migration notes. |
| `cortex-sdk` packages | External client lifecycle | Preserve generated typed bindings and release policy when SDK contracts change. |

The embedded Rust stable facade is the `cortex-engine` crate root, including
`Database`, `DatabaseOptions`, `EngineFeatureFlags`, `EngineError`,
`EngineErrorCode`, `EngineResult`, `DbOperation`, `ContextPack`,
`StorageStats`, `StorageValidationReport`, `RepairReport`, backup/restore
reports, retrieval/search reports, and compatibility report structs.

Implementation modules may remain public for current integration tests and
local tooling, but only crate-root exports and documented API surfaces are
compatibility promises.

## Required Ownership Matrix

| Area | Owner | Scope | Required gate |
| --- | --- | --- | --- |
| storage | Storage durability owner | WAL, manifest, segments, indexes, checkpoint/compact, backup/restore, validation. | `make storage-compat-check`; focused storage/backup tests for behavior changes. |
| search | Retrieval/search owner | Keyword, vector, ANN/HNSW, routing, recall/drift gates, search explain. | `make retrieval-quality-check`; ANN gates when vector/ANN behavior changes. |
| context | ContextPack owner | Packing, token budget, citations, dedup, explain, export formats. | `make context-pack-quality-check`. |
| verify | Verification owner | VERIFY FACT, numeric guards, contradiction logic, legal boundary reports, source trust. | `make verification-quality-check`; legal checks when legal boundary changes. |
| ingestion | Ingestion/provenance owner | Text/JSON/CSV/PDF ingestion, source refs, progress/jobs, typed bodies. | Ingestion tests plus API/CLI contract checks when exposed. |
| server | Server/API owner | Axum server, sync harness, auth, quotas, audit, dashboard, OpenAPI. | `make openapi-contract-check`; relevant `cargo test -p cortex-server`. |
| cli | CLI/operator owner | Local operator commands, JSON output, repair/backup/WAL/manifest tools. | `cargo test -p cortex-cli`; `make cli-product-check` for UX changes. |
| sdk | SDK contract owner | Published package contracts and generated bindings. | `make sdk-contract-check`; SDK release gates when publishing. |

Every non-trivial change should name the affected owner area in the PR or commit
description. Cross-area changes must run the strictest affected gate.

## Internal modules

The following top-level `crates/cortex-engine/src/lib.rs` modules are owned
implementation areas. The module ownership check verifies that this table stays
in sync with the current top-level module list.

| Module | Owner area | Boundary |
| --- | --- | --- |
| `agent_views` | server | Durable AgentView store used by server/auth and AQL runtime policy. |
| `backup` | storage | Backup, restore, retention, offsite staging, and dry-run validation. |
| `bundle` | storage | Segment bundle paths and retired segment garbage collection. |
| `checkpoint` | storage | Segment/index publication, compact, candidate mapping, profile safety. |
| `cleanup` | storage | Private filesystem cleanup internals. |
| `compatibility` | engine-facade | Public compatibility summaries and contract evidence. |
| `context` | context | ContextPack packing, explain, dedup, exports, citations, token budget. |
| `database` | engine-facade | Main `Database` facade and public DB operations. |
| `database_files` | storage | Private database path/file helpers. |
| `distributed` | distributed | Experimental single-node/distributed status facade. |
| `error` | engine-facade | Engine error taxonomy and stable error codes. |
| `feedback` | context | Context/search feedback cells and feedback scoring hooks. |
| `graph` | search | Knowledge graph and relation-facing retrieval support. |
| `ingestion` | ingestion | Ingestion adapters, chunks, jobs, PDF contracts, source ref reports. |
| `legal` | verify | Legal verification boundary and legal report contracts. |
| `lock` | storage | Private database lock implementation. |
| `memory` | context | Agent memory expiration, decay, and memory-specific operations. |
| `operation` | storage | WAL operation encoding/decoding and cell core metadata. |
| `options` | engine-facade | Database options, recovery mode, stale lock policy, feature flags. |
| `query` | search | AQL runtime catalog/provider/candidate/metadata integration. |
| `repair` | storage | Best-effort repair and operator maintenance reports. |
| `replay` | storage | WAL replay and recovery metrics. |
| `replication` | distributed | Experimental consensus, log matching, peer transport, snapshot repair. |
| `search` | search | Lexical, vector, ANN/HNSW, routing, quality, and persisted search. |
| `source_trust` | verify | Source trust categories and default trust policy. |
| `tool_registry` | ingestion | Tool registry cell descriptors and permission metadata. |
| `typed_body` | ingestion | Typed knowledge cell bodies for facts/entities/relations. |
| `validation` | storage | Storage validation report and stats. |
| `verification` | verify | Verification reports, numeric guards, contradictions, exports. |

## Cross-Crate Boundaries

| Crate | Owner area | Responsibility |
| --- | --- | --- |
| `cortex-aql` | search | AQL parser, binder, policy, mock bitmap VM. |
| `cortex-core` | storage | Cell ids, commit seq, MemTable MVCC. |
| `cortex-storage` | storage | WAL, manifest, segment/index/vector/HNSW formats. |
| `cortex-engine` | engine-facade | Stable facade over storage/core/query/search/context/verify. |
| `cortex-cli` | cli | Local operator UX and command contracts. |
| `cortex-server` | server | HTTP API, auth, quotas, audit, dashboard, and typed responses. |
| `cortex-sdk` | sdk | Client contracts and typed HTTP bindings. |

## Review Checklist

Before moving a function from an internal module to the stable facade:

1. Add a public API compile test or update the API compatibility fixture.
2. Add or update `docs/ENGINE_API.md`.
3. Update `fixtures/engine/public_api_freeze_v1.json` if the facade changes.
4. Confirm `make engine-api-check` passes.
5. Decide whether OpenAPI, SDK, CLI, or migration docs also need updates.

Before adding a new `cortex-engine` top-level module:

1. Add it to the Cortex Engine Module Map above.
2. Assign an owner area from the Required Ownership Matrix.
3. Add a gate if the module introduces a new public contract.
4. Run `make module-ownership-check`.
