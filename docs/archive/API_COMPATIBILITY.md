# API Compatibility Policy

Version: `v0.1.0-core-alpha`

## Stability Levels

| Endpoint | Method | Stability |
|----------|--------|-----------|
| `/v1/health` | GET | **Stable** |
| `/v1/stats` | GET | **Stable** |
| `/v1/validate` | GET | **Stable** |
| `/v1/cell` | GET / POST / DELETE | **Stable** |
| `/v1/flush` | POST | **Stable** |
| `/v1/compact` | POST | **Stable** |
| `/v1/aql` | POST | **Stable** |
| `/v1/search` | POST | **Stable** |
| `/v1/search/ann-evaluate` | POST | **Evolutionary** |
| `/v1/context` | POST | **Stable** |
| `/v1/remember` | POST | **Stable** |
| `/v1/verify` | POST | **Stable** |
| `/v1/ingest/text` | POST | **Stable** |
| `/v1/ingest/json` | POST | **Stable** |
| `/v1/ingest/csv` | POST | **Stable** |
| `/v1/ingest/jobs/{job_id}` | GET | **Stable** |

> **Stable** — JSON shape will not change without a version bump and migration note.  
> **Evolutionary** — fields may be added; existing fields will not be removed without notice.

## Breaking vs Non-Breaking Changes

### Breaking (requires version bump)

- Removing a field from a response
- Renaming a field
- Changing the type of a field (e.g. `integer` → `string`)
- Removing an endpoint
- Changing HTTP status codes for existing error conditions

### Non-Breaking (documented in changelog)

- Adding a new field to a response
- Adding a new endpoint
- Adding a new query parameter
- Adding a new error code

## Versioning Rules

- The API is versioned in the URL path: `/v1/...`
- When a breaking change is introduced, a new `/v2/...` path prefix is created.
- Old versions remain available for at least one minor release cycle.
- `docs/openapi.yaml` is the single source of truth for the current version.

## SDK Contract Tests

- Rust: `cargo test --workspace --all-features` includes snapshot/golden tests.
- Python: `sdk/python/tests/test_smoke.py` runs against a live server.
- TypeScript: `sdk/typescript/tests/smoke.test.ts` runs against a live server.
- OpenAPI coverage: `make openapi-check` validates that every router endpoint is documented.

## Deprecated Endpoints

Legacy compatibility aliases are deprecated and kept only for early local
clients. SDKs must use the versioned replacements.

| Deprecated route | Replacement | Removal target |
| --- | --- | --- |
| `/get` | `GET /v1/cell` | no earlier than the first beta minor release |
| `/put` | `POST /v1/cell` | no earlier than the first beta minor release |
| `/flush` | `POST /v1/flush` | no earlier than the first beta minor release |
| `/tombstone` | `DELETE /v1/cell` | no earlier than the first beta minor release |

## Deprecated Fields

None at this time. When a field is deprecated:

1. It is marked `deprecated: true` in `openapi.yaml`.
2. It is listed in this document with a removal target version.
3. It is preserved in responses for at least one minor release.
