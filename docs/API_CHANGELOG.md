# API Changelog

## Versioning Policy

See [`API_VERSIONING.md`](API_VERSIONING.md) for stability guarantees and breaking-change rules.

---

## Unreleased

### Added
- **AQL-specific error classes** — HTTP, OpenAPI, and Rust SDK error taxonomy
  now distinguish `unknown_field` and `unsupported_operator` from generic
  `invalid_aql` while keeping both as `400` user-input errors.
- **AQL Explain response** — `/v1/aql`, `cortexdb aql`, and the Rust SDK now
  expose an optional `explain` object for `EXPLAIN RETRIEVE CONTEXT`, including
  selected mode, bitmap plan, filters, candidate counts, limit, budget, and
  citation requirement.
- **Persisted contradiction relation cells** — engine users can now call
  `Database::persist_contradiction_relation` to write durable Relation cells
  with `predicate=contradicts`; `conflict_index` and `conflicts_for_fact` read
  both inline `contradicts=` markers and persisted relation cells.
- **Search Explain contribution details** — `/v1/search/explain`, `cortexdb
  search-explain`, and the Rust SDK now expose rank, matched terms, term
  contribution details, lexical/vector q16 shares, and hybrid fusion rank
  scores.
- **Search query routing decisions** — `/v1/search`, `cortexdb search`, and the
  Rust SDK now support `mode=auto` and expose `routing.selected_strategy` plus
  `routing.reason` for keyword/vector/hybrid selection.

### Changed
- **Error taxonomy contract guard** — `make openapi-contract-check` now also
  verifies the stable error-code list across `API_ERROR_TAXONOMY.md`,
  `API_JSON_SCHEMAS.md`, `API.md`, OpenAPI, server mappings, server snapshots,
  and Rust SDK decoder tests. Adding or changing an SDK-visible error code now
  requires updating every contract surface together.

---

## v0.1.0-core-alpha.4 (current)

### Added
- **ANN/HNSW construction profile field** — `AnnSearchReport` now includes
  `hnsw_ef_construction` in HTTP, OpenAPI, CLI JSON, and SDK contracts so
  recall/latency reports can be compared against the exact graph build profile.
- **Frozen API error taxonomy** — `docs/API_ERROR_TAXONOMY.md` now defines
  stable Core Alpha error codes, HTTP status mappings, producer rules, and
  compatibility rules.
- **SDK live error compatibility** — Python, TypeScript, and Rust SDK smoke
  checks now validate structured live-server error decoding for `invalid_aql`,
  `not_found`, and `invalid_tenant`.
- **Static multi-token auth policies** — HTTP server auth now supports
  `admin` and `data` token roles via `CORTEXDB_AUTH_TOKENS`, with optional
  per-token `AgentView` binding.

### Changed
- **API docs alignment** — `docs/API.md` and `docs/API_JSON_SCHEMAS.md` now
  list the full error enum already present in `RouterError` and OpenAPI:
  `invalid_tenant`, `forbidden`, and `service_unavailable`.
- **Rust SDK error enum alignment** — `cortex-sdk` now decodes the full Core
  Alpha error taxonomy, including `rate_limited`.
- **Admin/data route separation** — `data` tokens are denied from dashboard,
  stats, validation, flush, compact, and metrics routes with `403 forbidden`.

---

## v0.1.0-core-alpha.3

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
- **VERIFY numeric conflicts** — numeric conflict semantics now live in
  engine-level `VerificationReport.numeric_conflicts`; CLI/server JSON surfaces
  serialize that report instead of re-parsing payloads.
- **Source trust explain metadata** — ContextPack explain and VERIFY evidence
  now include deterministic `source_trust_category` alongside
  `source_trust_q16`.
- **VERIFY exports** — `/v1/verify` and `cortexdb verify` now support stable
  Markdown and deterministic audit-text report exports.

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
