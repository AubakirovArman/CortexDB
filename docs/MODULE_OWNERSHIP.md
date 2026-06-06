# Module Ownership

Status: local Epic 8 module boundary map.

This document separates the stable facade from internal modules so future work
does not accidentally promote implementation details into public API promises.

## Stable facade

The stable facade for embedded Rust users is the `cortex-engine` crate root:

```text
cortex_engine::Database
cortex_engine::DatabaseOptions
cortex_engine::RecoveryMode
cortex_engine::StaleLockPolicy
cortex_engine::EngineError
cortex_engine::EngineResult
cortex_engine::DbOperation
cortex_engine::ContextPack
cortex_engine::ContextPackOptions
cortex_engine::StorageStats
cortex_engine::StorageValidationReport
cortex_engine::RepairReport
cortex_engine::BackupReport
cortex_engine::RestoreReport
cortex_engine::RetrievedCell
cortex_engine::CheckpointStats
cortex_engine::CandidateId
cortex_engine::EngineAqlIndex
cortex_engine::*Report structs
```

Primary owner: engine facade and compatibility surface.

Change rule:

- keep examples compiling;
- update `ENGINE_API.md` for behavior changes;
- run `make engine-api-check`;
- run `make engine-api-compat-check`;
- run `make engine-public-api-freeze-check`;
- add migration or deprecation notes for breaking changes.

The freeze fixture is `fixtures/engine/public_api_freeze_v1.json`.

## Internal modules

The following `cortex-engine` modules are implementation areas. They can evolve
faster than the stable facade, but changes must preserve the public boundary:

| Module area | Ownership intent |
| --- | --- |
| `checkpoint` | Segment/index publication, compact, candidate mapping. |
| `cleanup`, `lock`, `database_files` | Local filesystem safety internals. |
| `operation`, `replay` | WAL operation encoding and replay. |
| `query` | AQL runtime catalog/provider integration. |
| `search` | Lexical/vector/ANN implementation internals. |
| `context` | ContextPack packing, explain, dedup internals. |
| `verification` | VERIFY FACT scoring and guard internals. |
| `replication` | Experimental consensus and repair primitives. |
| `backup`, `repair`, `validation` | Operator-facing maintenance implementation. |

The helper modules `cleanup`, `database_files`, `lock`, and `options` are
intentionally private and should not be promoted with `pub mod` without an API
freeze update.

## Cross-Crate Boundaries

| Crate | Responsibility |
| --- | --- |
| `cortex-aql` | AQL parser, binder, policy, mock bitmap VM. |
| `cortex-core` | Cell ids, commit seq, MemTable MVCC. |
| `cortex-storage` | WAL, manifest, segment/index formats. |
| `cortex-engine` | Stable facade over storage/core/query/search. |
| `cortex-cli` | Local operator UX. |
| `cortex-server` | HTTP API and dashboard surface. |
| `cortex-sdk` | Client contracts and typed HTTP bindings. |

## Review Checklist

Before moving a function from an internal module to the stable facade:

1. Add a public API compile test.
2. Add or update a doctest.
3. Document behavior and errors in `ENGINE_API.md`.
4. Confirm `make engine-api-check` passes.
5. Decide whether SDK/API/OpenAPI contracts also need updates.
