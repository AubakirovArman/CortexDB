# CortexDB Architecture

CortexDB is an experimental Core Alpha agent-native context database. The
current implementation is a single-node durable database core with AQL,
MemTable MVCC, ACLOG WAL, checkpoint/compact, search foundations,
ContextPack, deterministic VERIFY FACT, CLI, SDKs, dashboard assets, and an
Axum HTTP API.

It is not a production distributed database yet. Distributed consensus,
enterprise RBAC, managed cloud, and production-grade ANN without exact fallback
remain future product layers.

## System Shape

```text
CLI / SDK / HTTP / Dashboard
        |
        v
cortex-cli / cortex-sdk / cortex-server
        |
        v
cortex-engine
  Database facade
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

## Crates

| Crate | Responsibility |
| --- | --- |
| `cortex-aql` | AQL AST, parser, policy validator, binder, bitmap bytecode, and mock bitmap VM. |
| `cortex-core` | Core value types, `KnowledgeCell`, MVCC MemTable, read transactions, version visibility, and manifest primitives. |
| `cortex-storage` | ACLOG WAL, segment files, bitmap/lexical/vector/HNSW storage formats, checksums, readers, and writers. |
| `cortex-engine` | Main `Database` facade, WAL replay, checkpoint/compact, validation, repair, AQL retrieval, search, ContextPack, VERIFY FACT, ingestion, and local agent memory. |
| `cortex-server` | Axum/Tokio HTTP API, per-tenant database realms, `DatabaseActor`, typed JSON responses, auth, audit events, metrics, dashboard assets, and backpressure. |
| `cortex-cli` | `cortexdb` command-line tool for local database operations, fixtures, search, context, verify, backup/restore, WAL, and manifest utilities. |
| `cortex-sdk` | Rust HTTP client; Python and TypeScript SDKs live under `sdk/` and follow the SDK release contract. |

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

`docs/CELL_METADATA_MODEL.md` defines the Core Alpha source of truth. WAL
sections and future typed sections may carry optimized metadata, but payload
metadata lines are the current compatibility boundary.

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

The invariant is strict: WAL append happens before MemTable mutation. If WAL
append fails, MemTable does not change. New operation records include durable
`CommitSeq` in the `CellCore` WAL section.

Patch is currently a full payload replacement. Section-level patching is a
future merge-policy milestone.

## Open and Recovery

```text
Database::open(path)
        |
        v
acquire database lock
cleanup known orphan temp files
load manifest + checkpointed segment bundles
scan WAL
decode operations
apply visible versions into MemTable
start WAL writer
```

Strict recovery fails on corruption. Best-effort recovery stops at a safe
truncate offset for partial or corrupt WAL tails. Validation checks live segment
bundles, manifest consistency, candidate IDs, index readability, vector/HNSW
compatibility, WAL scan state, and checkpoint sequence invariants.

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

Bitmap, lexical, vector, and HNSW indexes operate on internal candidate IDs,
not raw `CellId` values. Candidate IDs are persisted in segments and mapped
back to full `CellId` values to avoid `u64 -> u32` truncation. Candidate ID `0`
is reserved and invalid.

## Query Path

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

AQL filters may only narrow the `AgentView` allowed mask. `BitmapOp::Not` is a
complement inside the segment-local universe, not a permission expansion
mechanism.

## Search and ANN

Search currently includes:

- keyword/BM25-like lexical scoring;
- exact vector search;
- guarded ANN/HNSW search with exact fallback;
- hybrid/RRF foundations;
- multilingual tokenizer tests.

ANN/HNSW has fixture, drift, metric-matrix, corpus smoke, real-embedding, and
release-evidence gates. Exact search remains the correctness fallback, and
long-running production latency history is still future work.

## ContextPack

ContextPack is the main agent-native output:

```text
candidate cells
-> scoring and feedback weighting
-> token budget estimation
-> redundancy checks
-> citation/anomaly reporting
-> typed JSON response
```

ContextPack prepares source-grounded context for an external agent or model. It
does not call an LLM inside the database core.

## VERIFY FACT

`VERIFY FACT` is deterministic and heuristic. It compares retrieved evidence,
citations, normalized terms, contradiction markers, and numeric conflicts. It
can report supported, contradicted, mixed, or insufficient evidence. It is not
a legal proof engine or production-grade factual certification layer.

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
realms are directory-backed and path-validated. Authentication is Bearer-token
based with legacy single-token mode, static `admin`/`data` token policies, and
file-backed token rotation through `CORTEXDB_AUTH_TOKENS_FILE`.

Core Alpha includes bounded actor queues, explicit `database_busy`
backpressure, fixed-window rate limiting, exact-origin CORS allowlisting, and
route-level audit events with an optional synced JSONL sink.

## Public Interfaces

| Interface | Scope |
| --- | --- |
| CLI | Local database operations, fixtures, stats, validation, search, context, verify, backup/restore, WAL/manifest tools. |
| HTTP API | Versioned `/v1/*` JSON API documented by `docs/openapi.yaml` and `docs/API_JSON_SCHEMAS.md`. |
| SDKs | Rust client crate plus Python/TypeScript packages under the SDK release process. |
| Dashboard | Dependency-free static developer console with embedded server assets and standalone `web/dashboard/dist` artifact. |
| Docker | Local `cortex-server` packaging with non-root runtime user and healthcheck. |

## Release Gates

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
make single-node-performance-check
make tenant-recovery-check
make production-evidence-sweep
make ann-release-evidence-check
make backup-drill-check
make backup-offsite-check
make crash-fault-check
make chaos-restart-check
make sdk-contract-check
```

`make alpha-check` and `make release-check` compose broader release evidence,
including demos, SDK checks, dashboard checks, ANN release evidence,
backup/restore drills, offsite backup staging, crash/fault repair evidence,
and process kill/restart evidence.

## Core Alpha Non-Goals

- Production-grade distributed consensus.
- Enterprise RBAC, external identity providers, and audit compliance.
- Managed cloud service.
- Built-in LLM inference.
- Production-quality HNSW without exact fallback, recall gates, and longer
  latency history.
- General-purpose replacement for PostgreSQL, SQLite, or vector databases.

## Next Architecture Work

1. Real distributed consensus with replicated log semantics, leader election,
   failover, partitions, and split-brain tests.
2. Full product Web UI beyond the developer dashboard.
3. ANN/HNSW production tuning with real embedding baselines and longer
   latency-history gates.
4. Dynamic RBAC policy store and external identity integration.
5. Tamper-evident audit trail and external log/SIEM export.
6. Migration policy for storage/API format changes.
