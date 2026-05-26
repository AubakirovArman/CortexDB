# CortexDB Project Status & Honesty Manifest (v1.0.0-stable)

CortexDB has successfully transitioned from an early database loop prototype into a **Stable Core Alpha Candidate (v1.0.0-stable)**. This document honestly defines the completed boundaries of the database core and separates them from our long-term distributed research goals.

---

## 1. What is Fully Completed & Stable (v1.0.0)

- **Durable Single-Node Storage:** Strict Write-Ahead Log (WAL) with group commits, MVCC MemTable, and incremental compaction.
- **Asynchronous Network Surface:** Fully async server built on Axum and Tokio. Blocking DB transactions are safely executed on a thread pool via `tokio::task::spawn_blocking` to prevent worker thread starvation.
- **Heuristic Fact Verification (`VERIFY FACT`):** Fully generic numeric and citation mismatch parser that is 100% deterministic and runs without any float calculations or LLM calls.
- **Context Pack Compiler:** Budgeted, scored, and deduplicated Context Packs generated directly from AQL queries.
- **Ecosystem:** Fully typed Python and TypeScript SDK clients, local dataset fixture loaders, and complete automated verification gates (`make alpha-check`).

---

## 2. What is Experimental & Under Active Development (Non-goals for v1.0.0)

- **Consensus-Driven Replication (Raft):** Multi-node log syncing and election states (current status: primitive experimental models).
- **Consistent Hashing Sharding:** Distributed namespace layout and routing (current status: early layout primitives).
- **HNSW Approximate Search:** High-dimensional vector indexing (current status: exact scan is our production-grade standard; HNSW remains an experimental module).

---

## 3. General Limitations

- **Production Readiness:** CortexDB is suitable for local experiments, research, agent memory demos, and early contributors. It is not ready for critical high-availability production databases.
- **Memory Consumption:** The MVCC MemTable keeps active transactions in memory; compact often to maintain a lightweight footprint.
