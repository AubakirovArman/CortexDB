# CortexDB Project Status & Honesty Manifest (v0.1.0-core-alpha candidate)

CortexDB has successfully transitioned from an early database loop prototype into a **Stable Core Alpha Candidate (v0.1.0-core-alpha candidate)**. This document honestly defines the completed boundaries of the database core and separates them from our long-term distributed research goals.

---

## 1. What is Fully Completed & Stable (v0.1.0-core-alpha candidate)

- **Durable Single-Node Storage:** Strict Write-Ahead Log (WAL) with group commits, MVCC MemTable, and incremental compaction.
- **Asynchronous Network Surface:** Async HTTP server built on Axum and Tokio. Blocking DB transactions run through per-tenant `DatabaseActor` workers behind `tokio::task::spawn_blocking`; the database core itself remains local and blocking.
- **Developer Web Console:** Built-in `/dashboard` UI for health, stats, validation, cell operations, keyword/vector search, ANN evaluation, ingestion, ingestion job lookup, AQL, ContextPack, and VERIFY FACT smoke paths. The console can target per-tenant database realms and keeps a short local request history.
- **Heuristic Fact Verification (`VERIFY FACT`):** Deterministic numeric and citation mismatch parser that runs without LLM calls. API display formatting may scale large integer values for readability.
- **Context Pack Compiler:** Budgeted, scored, and deduplicated Context Packs generated directly from AQL queries.
- **Ecosystem:** Python, TypeScript, and Rust SDK clients with package dry-runs, local dataset fixture loaders, and complete automated verification gates (`make alpha-check`).

---

## 2. What is Experimental & Under Active Development (Non-goals for v1.0.0)

- **Consensus-Driven Replication (Raft):** Multi-node log syncing and election states (current status: experimental model with vote freshness checks, AppendEntries log matching, conflict truncation, and snapshot transfer smoke paths).
- **Consistent Hashing Sharding:** Distributed namespace layout and routing (current status: early layout primitives).
- **HNSW Approximate Search:** High-dimensional vector indexing (current status: exact scan is our production-grade standard; HNSW remains experimental but guarded by exact fallback for empty, invalid, or under-returning graphs).

---

## 3. General Limitations

- **Production Readiness:** CortexDB is suitable for local experiments, research, agent memory demos, and early contributors. It is not ready for critical high-availability production databases.
- **Memory Consumption:** The MVCC MemTable keeps active transactions in memory; compact often to maintain a lightweight footprint.
