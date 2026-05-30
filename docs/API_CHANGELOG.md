# API Changelog

## Versioning Policy

See [`API_VERSIONING.md`](API_VERSIONING.md) for stability guarantees and breaking-change rules.

---

## v0.1.0-core-alpha.3 (current)

### Added
- **`GET /v1/metrics`** — aggregated database metrics (storage + WAL + ANN/HNSW in one response).
- **`POST /v1/search/explain`** — returns tokenized query terms and per-cell score breakdown for debugging search results.
- **RouterError taxonomy** — `EngineError` variants now map to proper HTTP status codes:
  - `AqlParse` and non-policy `AqlBind` → `400 invalid_aql`
  - policy-denied `AqlBind` → `403 permission_denied`
  - `DatabaseAlreadyOpen` and full actor queues → `503 database_busy`
  - storage corruption or invariant failures → `500 storage_corruption`
  - unknown routes and missing jobs → `404 not_found`
- **New response fields** — `MetricsResponse` with `ann_graph_nodes`, `ann_total_edges`, etc.

### Changed
- **Tenant validation** — `:` is no longer allowed in tenant IDs (cross-platform safety).
- **Query params** — all query parameters are now percent-decoded (`+` → space, `%XX` decoded).
- **Actor shutdown** — `DatabaseActor::drop` now drops the sender before
  joining the worker, so shutdown cannot hang when the bounded queue is full.
- **Actor queue capacity** — configurable via
  `ServerOptions.actor_queue_capacity` and the
  `CORTEXDB_ACTOR_QUEUE_CAPACITY` environment variable (default 1024).

### Deprecated
- Legacy compatibility aliases `/get`, `/put`, `/flush`, and `/tombstone` remain
  available for early local clients but are deprecated. Use the versioned
  `/v1/*` replacements documented in `SDK_DEPRECATION_POLICY.md`.

### Security
- **Legacy sync_handler** is now gated under `#[cfg(test)]` and cannot be used in production builds.

---

## v0.1.0-core-alpha.2

### Added
- **URL percent-decoding** for query parameters (`query_param_decoded`, `query_param_opt_decoded`).
- **Tenant hardening** — expanded tests for path traversal (`../x`, `a%2Fb`, `.`, `..`).
- **OpenAPI alpha notice** for Search/HNSW endpoint.
- **`docs/DEMO_OUTPUT.md`** — reference output for the investment projects demo.

### Changed
- **`ContextPackAnomalyResponse.cell_id`** — changed from `u64` to `Option<u64>`.
- **`format_scale_currency`** — pure-integer formatter, no `f64` division.

---

## v0.1.0-core-alpha.1

### Added
- Typed JSON response structs for all server endpoints (`responses.rs`).
- `insta` snapshot tests for response serialization.
- `docs/TENANT_NAMING_RULES.md`.
- `docs/RELEASE_NOTES_v0.1.0-core-alpha.md`.

---

## v0.1.0-core-alpha

### Added (initial release)
- Single-node durable storage (WAL, MVCC, checkpoint, compact).
- AQL parser and executor.
- ContextPack with token budgets, citations, anomalies.
- VERIFY FACT with numeric conflict detection.
- Search foundation (keyword + vector + experimental HNSW).
- Ingestion foundation (text, JSON, CSV).
- HTTP server with Axum/Tokio, per-tenant `DatabaseActor`.
- CLI with `clap` derive.
- OpenAPI 3.1 contract.
- Python, TypeScript, and Rust SDK clients.
