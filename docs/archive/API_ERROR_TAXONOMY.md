# CortexDB API Error Taxonomy

This document freezes the Core Alpha HTTP error contract. Server routes return a
typed JSON body for failures:

```json
{
  "code": "invalid_aql",
  "error": "invalid_aql",
  "message": "safe human-readable message"
}
```

`code` is the SDK-facing stable enum value. `error` currently mirrors `code` for
simple clients. `message` is safe to show to users and must not expose secrets,
filesystem internals, private scope names, brain names, or stack traces.

## Stable Codes

| HTTP status | Code | Producer | Client behavior |
| --- | --- | --- | --- |
| `400` | `bad_request` | Missing parameters, malformed non-AQL inputs, invalid vector literals, invalid job ids. | Fix the request and retry. |
| `400` | `invalid_tenant` | Tenant realm name fails charset, length, or path-safety validation. | Fix tenant id; do not retry unchanged. |
| `400` | `invalid_aql` | AQL parse error or non-policy bind error that is not classified more specifically. | Fix the AQL query and retry. |
| `400` | `unknown_field` | AQL `WHERE` references a field that is not filterable. | Use a supported AQL field and retry. |
| `400` | `unsupported_operator` | AQL `WHERE` uses a parsed comparator that the binder does not support for that field. | Use a supported operator such as `=` or `IN`. |
| `401` | `unauthorized` | Bearer auth is enabled and the request is missing or has a wrong token. | Authenticate and retry. |
| `403` | `forbidden` | Non-AgentView authorization denials, such as a `data` token attempting an admin/metrics route. | Treat as a hard deny. |
| `403` | `permission_denied` | `AgentView`, scope, mode, or policy denial. | Treat as a hard deny; changing the query must not bypass policy. |
| `404` | `not_found` | Unknown route or missing resource such as an ingestion job. | Fix the path/id or handle missing resource. |
| `413` | `payload_too_large` | Request body exceeds the server body boundary. | Reduce body size. |
| `429` | `rate_limited` | Optional fixed-window server rate limit is exhausted. | Back off and retry later. |
| `429` | `quota_exceeded` | Per-principal or per-tenant quota is exhausted. | Reduce write/load volume, raise the relevant quota, or retry after the local window/queue drains. |
| `500` | `storage_corruption` | Storage checksum, format, missing-file, or invariant failure. | Stop writes; run validation/repair workflow. |
| `500` | `internal` | Unexpected internal error not classified above. | Treat as server fault. |
| `503` | `database_busy` | Database actor queue is full or the local database lock is busy. | Back off and retry later. |
| `503` | `service_unavailable` | Server component is unavailable but not classified as queue/lock pressure. | Retry later or restart the server. |

## Mapping Rules

- Engine-level errors are classified by `EngineError::code()` and documented in
  [`ENGINE_ERROR_MODEL.md`](ENGINE_ERROR_MODEL.md). The HTTP adapter maps that
  engine code into this SDK-facing API taxonomy.
- AQL syntax and generic non-policy bind failures map to `400 invalid_aql`.
- AQL unknown fields map to `400 unknown_field`.
- AQL unsupported comparators map to `400 unsupported_operator`.
- Policy-denied AQL bind failures map to `403 permission_denied`.
- `DatabaseAlreadyOpen` and bounded actor queue pressure map to
  `503 database_busy`.
- Storage checksum, format, missing-file, and invariant failures map to
  `500 storage_corruption`.
- Unknown routes and missing ingestion jobs map to `404 not_found`.
- Tenant validation is performed before routing and maps to
  `400 invalid_tenant`.

## Compatibility Rules

1. Existing code names, HTTP status mappings, and JSON fields are stable for the
   Core Alpha contract.
2. A new error code requires updating this document, `docs/openapi.yaml`,
   `docs/API_JSON_SCHEMAS.md`, SDK decoding expectations, and server taxonomy
   tests.
3. Reusing a code for a different class of failure is a breaking API change.
4. Messages may become clearer, but must remain safe and must not become the
   only machine-readable signal.
