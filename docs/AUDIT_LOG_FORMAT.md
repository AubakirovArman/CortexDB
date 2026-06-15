# Audit Log Format

Status: E07 productized local audit JSONL format.

CortexDB writes one JSON object per line when `CORTEXDB_AUDIT_LOG_FILE` is set.
The current record schema is `cortexdb.audit.v1`; SIEM exports use
`cortexdb.siem.audit.v1`.

## Server Options

```bash
export CORTEXDB_AUDIT_LOG=true
export CORTEXDB_AUDIT_LOG_FILE="./audit/http.jsonl"
export CORTEXDB_AUDIT_LOG_ROTATE_BYTES=104857600
export CORTEXDB_AUDIT_LOG_FSYNC=always
```

- `CORTEXDB_AUDIT_LOG_FILE` implies audit logging and creates parent
  directories.
- `CORTEXDB_AUDIT_LOG_ROTATE_BYTES` rotates the active JSONL file after the
  configured byte limit. Rotated files remain local JSONL files with independent
  chain verification.
- `CORTEXDB_AUDIT_LOG_FSYNC=always` flushes and calls `sync_data()` for every
  event. `flush` or `flush-only` flushes without `sync_data()` for lower write
  latency.

## Record Fields

| Field | Meaning |
| --- | --- |
| `schema_version` | Always `cortexdb.audit.v1` for local server audit records. |
| `audit_event` | Event type, for example `http_response` or `llm_inference_decision`. |
| `audit_action` | Route category such as `read`, `write`, `search`, `verify`, `admin`, or `metrics`. |
| `chain_id`, `sequence`, `prev_hash`, `event_hash` | Local hash-chain metadata checked by `cortexdb audit --verify-chain`. |
| `principal_id`, `auth_role`, `auth_agent_id` | Authenticated principal metadata when available; tokens are never stored. |
| `scope_decision` | `allowed`, `denied`, or `not_applicable`; raw scope labels are not stored. |
| `method`, `path`, `tenant`, `request_id` | HTTP route metadata without query strings. |
| `status`, `error_code`, `duration_ms`, `unix_time_ms` | Response outcome and timing metadata. |
| `llm` | Safe inference decision metadata; prompts, provider responses, and API keys are not stored. |

## Review And Export

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
cortexdb audit verify ./audit/http.jsonl
cortexdb audit ./audit/http.jsonl --route /v1/cell --status 403
cortexdb audit-export-siem ./audit/http.jsonl ./audit/siem.jsonl --redaction-check --verify-chain
```

The audit CLI filters by route, status, action, and tenant. Redaction checks
fail if body-like fields, query strings, bearer tokens, prompts, provider
responses, API keys, or secrets appear in records.

## Boundary

This is a local operational audit trail with tamper-evident JSONL chain checks.
It is not a compliance-certified immutable ledger and does not provide
vendor-managed SIEM delivery.
