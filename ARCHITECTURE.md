# CortexDB Architecture

CortexDB is an experimental Core Alpha agent-native context database. The
current system is a single-node durable database core with AQL, context
packing, deterministic verification, search foundations, CLI, SDK, and an HTTP
server. It is not a production distributed database yet.

## System Shape

```text
CLI / SDK / HTTP / Dashboard
        |
        v
cortex-server / cortex-cli / cortex-sdk
        |
        v
cortex-engine
  Database
  WAL replay
  checkpoint / compact
  AQL retrieve
  search
  ContextPack
  VERIFY FACT
  ingestion / memory
        |
        +------------------+
        |                  |
        v                  v
cortex-core          cortex-storage
  CellId              ACLOG WAL
  CommitSeq           manifest
  KnowledgeCell       .acs segments
  MemTable MVCC       .acb bitmap index
  ReadTxn             .aci lexical index
                      .acv vector index
                      .ach HNSW graph
        ^
        |
cortex-aql
  parser -> AST -> binder -> bitmap program
```

## Workspace Crates

| Crate | Responsibility |
| --- | --- |
| `cortex-aql` | AQL AST, parser, policy validator, binder, bitmap bytecode, and mock bitmap VM. |
| `cortex-core` | Core value types, `KnowledgeCell`, MVCC MemTable, read transactions, version visibility, manifest primitives. |
| `cortex-storage` | ACLOG WAL, segment files, bitmap/lexical/vector/HNSW storage formats, checksums, readers, writers. |
| `cortex-engine` | Main `Database` facade, WAL replay, checkpoint/compact, validation, repair, AQL retrieval, search, ContextPack, VERIFY FACT, ingestion, local agent memory. |
| `cortex-server` | Axum/Tokio HTTP API, per-tenant database realms, `DatabaseActor`, typed JSON responses, dashboard assets. |
| `cortex-cli` | `cortexdb` command-line tool for local database operations, fixtures, search, context, verify, WAL and manifest utilities. |
| `cortex-sdk` | Rust HTTP client for the versioned server API; Python and TypeScript SDK packages live under `sdk/`. |

## Data Model

The durable unit is a knowledge cell:

```text
CellId
CommitSeq
payload bytes
metadata lines in payload for Core Alpha:
  scope=...
  status=...
  type=...
  source=...
  source_trust=...
```

`docs/CELL_METADATA_MODEL.md` defines the Core Alpha source of truth for
metadata. WAL sections and future typed sections may carry optimized metadata,
but the payload metadata model is the current compatibility boundary.

## Write Path

```text
Database::put_cell / patch_cell / tombstone_cell
        |
        v
DbOperation with durable CommitSeq
        |
        v
ACLOG WAL append + durability policy
        |
        v
MemTable MVCC update
        |
        v
visible through ReadTxn / latest reads
```

The write invariant is strict: WAL append happens before MemTable mutation. If
WAL append fails, MemTable must not change. New operation records include a
durable `CommitSeq` in the `CellCore` section.

Patch is currently a full payload replacement. Section-level patching is a
future compatibility and merge-policy milestone.

## Restart and Recovery

```text
Database::open(path)
        |
        v
acquire database lock
cleanup known orphan temp files
load manifest + checkpointed segments
scan WAL
decode operations
apply visible versions into MemTable
start WAL writer
```

Recovery supports strict and best-effort modes. Strict recovery fails on
corruption. Best-effort recovery stops at the safe truncate offset for partial
or corrupt WAL tails.

Storage validation checks live segment bundles, manifest consistency, candidate
IDs, index readability, vector/HNSW compatibility, WAL scan state, and
checkpoint sequence invariants.

## Checkpoint and Compact

Checkpoint writes changed cells since the last checkpoint into a new segment
bundle:

```text
.acs segment
.acb bitmap index
.aci lexical index
.acv vector index
.ach HNSW graph
manifest update
WAL truncation after durable manifest publication
```

Compact writes a full visible snapshot and retires superseded segment bundles.
Critical storage files are written with temp-file, fsync, atomic rename, parent
directory fsync, and CRC32C validation.

## Candidate IDs

Bitmap, lexical, vector, and HNSW indexes operate on internal candidate IDs, not
raw `CellId` values. Candidate IDs are persisted in segments and mapped back to
full `CellId` values to avoid `u64 -> u32` truncation. Candidate ID `0` is
reserved and invalid.

## Read and Query Path

Direct reads use MVCC visibility:

```text
created_seq <= read_seq
AND (deleted_seq IS NULL OR deleted_seq > read_seq)
```

AQL retrieval uses the compiled bitmap pipeline:

```text
AQL string
-> parser
-> Raw AST
-> binder + AgentView policy
-> BoundRetrievePlan
-> BitmapProgram
-> persisted/runtime bitmap provider
-> candidate IDs
-> CellId lookup
-> visible payloads
```

AQL filters may only narrow the `AgentView` allowed mask. The binder starts from
agent-allowed and live masks, then intersects compiled `WHERE` filters.
`BitmapOp::Not` is a complement inside the segment-local universe, not a
permission expansion mechanism.

## Search

Search is part of `cortex-engine` and currently includes:

- keyword/BM25-like lexical scoring;
- exact vector search;
- guarded ANN/HNSW search with exact fallback;
- hybrid/RRF foundations;
- multilingual tokenization tests.

ANN/HNSW has recall and drift gates, metric-matrix checks, external fixture
checks, and report artifacts. It remains guarded: exact search is the critical
correctness fallback, and long-running production latency history is still a
future milestone.

## ContextPack

ContextPack is CortexDB's main agent-native output. The engine turns retrieved
cells into a bounded package:

```text
candidate cells
-> scoring and feedback weighting
-> token budget estimation
-> redundancy checks
-> citation/anomaly reporting
-> typed JSON response
```

ContextPack is evidence-aware but not an LLM. It prepares source-grounded
context for an external agent or model.

## VERIFY FACT

`VERIFY FACT` is deterministic and heuristic. It compares retrieved evidence,
citations, normalized terms, natural-language contradiction markers, and
numeric conflicts. It can report supported, contradicted, mixed, or
insufficient evidence. It is not a legal proof engine and should not be
presented as production-grade factual certification.

## HTTP Server

`cortex-server` provides an async HTTP surface over a local blocking database
core:

```text
Axum/Tokio request
-> body size/auth/tenant checks
-> per-tenant DatabaseActor
-> route_database(&mut Database)
-> typed response struct
-> JSON
```

The actor owns the `Database` for a tenant and serializes mutations. Tenant
realms are directory-backed and path-validated. Authentication is currently
Bearer-token based. Core Alpha also has opt-in `AgentView` binding for one
configured token, bounded actor queues with explicit `database_busy`
backpressure, fixed-window rate limiting, exact-origin CORS allowlisting, and
route-level audit events with an optional synced JSONL file sink.

Full multi-token RBAC, per-user quotas, tamper-evident audit trails, SIEM
export, and production tenant authorization remain future security milestones.

## Public Interfaces

| Interface | Scope |
| --- | --- |
| CLI | Local database operations, fixtures, stats, validation, search, context, verify, WAL/manifest tools. |
| HTTP API | Versioned `/v1/*` JSON API documented by `docs/openapi.yaml` and `docs/API_JSON_SCHEMAS.md`. |
| SDKs | Rust client crate plus Python/TypeScript packages under the SDK release process. |
| Dashboard | Dependency-free static developer console with embedded server assets and standalone `web/dashboard/dist` artifact. |
| Docker | Local `cortex-server` packaging with non-root runtime user and healthcheck. |

## Safety Boundaries

- AQL cannot expand `AgentView` permissions.
- WAL append precedes MemTable mutation.
- New WAL operations include durable `CommitSeq`.
- Candidate IDs are internal and must not truncate `CellId`.
- Segment/index/manifest writes are atomic and checksummed.
- Best-effort recovery stops at safe offsets.
- Tenant names are path-validated.
- HTTP body size is bounded.
- Actor queues are bounded and expose explicit backpressure.
- Browser CORS is disabled by default and only supports one exact allowlisted
  origin when configured.
- HTTP audit logging is opt-in and records route metadata without request
  bodies or query strings.
- Production claims are intentionally limited to experimental Core Alpha.

## Current Release Gates

The main local gates are:

```sh
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make openapi-contract-check
make sdk-check
make dashboard-check
make dashboard-smoke
make dashboard-screenshots
make ann-fixture-check
make ann-drift-check
make ann-external-check
make ann-metric-matrix-check
make ann-release-evidence-check
make sdk-contract-check
```

`make alpha-check` and `make release-check` compose broader release evidence,
including demo and ANN report gates.

## Non-Goals for Core Alpha

- Production-grade distributed consensus.
- Enterprise RBAC and audit compliance.
- Managed cloud service.
- Built-in LLM inference.
- Production-quality HNSW without exact fallback and recall gates.
- General-purpose replacement for PostgreSQL, SQLite, or vector databases.

## Next Architecture Work

The next architectural hardening items are:

1. API/error taxonomy freeze.
2. SDK end-to-end compatibility tests against a local server.
3. Backup/restore operational hardening and restore drills.
4. Multi-token auth policy, RBAC, and admin/data route separation.
5. Crash/fault injection harness.
6. Search and verification quality datasets.
7. Migration policy for storage/API format changes.
8. Tamper-evident audit trail and external log/SIEM export.
