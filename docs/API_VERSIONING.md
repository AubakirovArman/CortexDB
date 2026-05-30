# API Versioning Policy

## Current Status

CortexDB is at **v0.1.0-core-alpha**. All HTTP endpoints are prefixed with `/v1/`.

## Versioning Rules

### 1. URL Path Prefixing

Every API endpoint includes a version prefix:

```text
/v1/health
/v1/stats
/v1/cell
/v1/context
/v1/search
```

Legacy compatibility aliases (without `/v1/`) exist for a small set of core endpoints but are deprecated:

```text
/get   → /v1/cell (GET)
/put   → /v1/cell (POST)
/flush → /v1/flush (POST)
/tombstone → /v1/cell (DELETE)
```

### 2. Stability Guarantees by Phase

| Phase | Stability | Breaking Changes |
|-------|-----------|------------------|
| Core Alpha (now) | **No stability guarantee.** | May change without notice. |
| Beta | JSON field names stable; new fields additive only. | 2-week deprecation window. |
| v1.0 | Full backward compatibility within major version. | Only in new major versions. |

### 3. What Constitutes a Breaking Change

Breaking (requires new major version):
- Removing or renaming a JSON field
- Changing the meaning of a field
- Removing an endpoint
- Changing required query parameters
- Changing error codes

Non-breaking (additive):
- Adding new JSON fields
- Adding new endpoints
- Adding new enum values
- Adding new query parameters (optional)

### 4. Response Version Indicator

All JSON responses include an implicit version through the API path. Responses
with independent schema lifecycles may also expose an explicit schema marker;
for example, `/v1/context` returns `schema_version: "context_pack.v1"`.

### 5. Planned Versions

| Version | Focus | Tentative |
|---------|-------|-----------|
| `/v1/` | Core Alpha — current, including ContextPack v1 | Now |
| `/v2/` | Future breaking API contracts | Post-beta |

### 6. Client Best Practice

Clients should:
1. Pin the major version in the URL (`/v1/`, `/v2/`).
2. Ignore unknown JSON fields (forward compatibility).
3. Handle all documented HTTP status codes, including new ones.
4. Not depend on exact error message strings; use the `error` code field.

## Example

```bash
# Current alpha (stable within the alpha iteration)
curl 'http://127.0.0.1:8181/v1/health'

# Future beta
 curl 'http://127.0.0.1:8181/v1/health'  # still works
 curl 'http://127.0.0.1:8181/v2/context'  # new features
```
