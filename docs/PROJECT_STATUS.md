# CortexDB Project Status & Honesty Manifest (v0.1.0-core-alpha)

CortexDB has successfully transitioned from an early database loop prototype into a **published Core Alpha (v0.1.0-core-alpha)**. This document honestly defines the completed boundaries of the database core and separates them from our long-term distributed research goals.

---

## 1. What is Fully Completed & Stable (v0.1.0-core-alpha)

- **Durable Single-Node Storage:** Strict Write-Ahead Log (WAL) with group commits, MVCC MemTable, and incremental compaction.
- **Asynchronous Network Surface:** Async HTTP server built on Axum and Tokio. Blocking DB transactions run through per-tenant `DatabaseActor` workers behind `tokio::task::spawn_blocking`; the database core itself remains local and blocking.
- **Core Alpha HTTP Safety Controls:** Optional Bearer auth, static `admin`/`data` token policies, optional per-token `AgentView` binding for scope-aware data routes, bounded actor queues with explicit `database_busy` backpressure, optional fixed-window rate limiting, exact-origin CORS allowlisting, and opt-in route-level audit logging with an optional synced JSONL file sink.
- **Developer Web Console:** Built-in `/dashboard` UI for health, stats, validation, cell operations, keyword/vector search, ANN evaluation, ingestion, ingestion job lookup, AQL, ContextPack, and VERIFY FACT smoke paths. The console can target per-tenant database realms, keeps a short local request history, has frontend sources under `web/dashboard/src`, and is served from versioned static assets under `/dashboard/assets/v1/` so it can evolve into a standalone frontend product without growing Rust string modules.
- **Heuristic Fact Verification (`VERIFY FACT`):** Deterministic numeric and citation mismatch parser that runs without LLM calls. API display formatting may scale large integer values for readability.
- **Context Pack Compiler:** Budgeted, scored, and deduplicated Context Packs generated directly from AQL queries.
- **Ecosystem:** Python, TypeScript, and Rust SDK clients with tenant-aware routing, package dry-runs, local dataset fixture loaders, and complete automated verification gates (`make alpha-check`).

---

## 2. What is Experimental & Under Active Development (Non-goals for v1.0.0)

- **Consensus-Driven Replication (Raft):** Multi-node log syncing and election states (current status: experimental model with vote freshness checks, AppendEntries log matching, conflict truncation, and snapshot transfer smoke paths).
- **Consistent Hashing Sharding:** Distributed namespace layout and routing (current status: early layout primitives).
- **Guarded HNSW Approximate Search:** High-dimensional vector indexing with deterministic multi-layer graph links and exact-fallback guardrails. `DistanceMetric` supports fixed-point DotProduct, Cosine, and L2. Repeatable in-repo recall/latency reports plus release-mode synthetic, drift, external JSONL, and metric-matrix fixture gates exist. `ann_corpus_check` can evaluate larger external vectors/queries/ground-truth files; long-running benchmark history remains future tuning work.

---

## 3. General Limitations

- **Production Readiness:** CortexDB is suitable for local experiments, research, agent memory demos, and early contributors. It is not ready for critical high-availability production databases.
- **Memory Consumption:** The MVCC MemTable keeps active transactions in memory; compact often to maintain a lightweight footprint.
- **Security Model:** Core Alpha has local safety controls and static route roles, but not a dynamic multi-user RBAC policy store, per-user quotas, tamper-evident audit trails, at-rest encryption, or distributed security hardening.
