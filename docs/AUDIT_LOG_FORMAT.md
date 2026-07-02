# Audit Log Format

Status: E07 productized local audit JSONL format, upgraded to keyed v2 audit
records for file-backed logs.

CortexDB writes one JSON object per line when `CORTEXDB_AUDIT_LOG_FILE` is set.
The current server-written record schema is `cortexdb.audit.v2`; legacy
`cortexdb.audit.v1` hash-chain records remain readable by the CLI as local
compatibility records. SIEM exports use `cortexdb.siem.audit.v1`.

## Server Options

```bash
export CORTEXDB_AUDIT_LOG=true
export CORTEXDB_AUDIT_LOG_FILE="./audit/http.jsonl"
export CORTEXDB_AUDIT_LOG_ROTATE_BYTES=104857600
export CORTEXDB_AUDIT_LOG_FSYNC=always
export CORTEXDB_AUDIT_MAC_KEY_ID="local-audit-key-2026q2"
export CORTEXDB_AUDIT_MAC_KEY_HEX="000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
```

- `CORTEXDB_AUDIT_LOG_FILE` implies audit logging and creates parent
  directories.
- `CORTEXDB_AUDIT_LOG_ROTATE_BYTES` rotates the active JSONL file after the
  configured byte limit. Rotated files remain local JSONL files with independent
  chain verification.
- `CORTEXDB_AUDIT_LOG_FSYNC=always` flushes and calls `sync_data()` for every
  event. `flush` or `flush-only` flushes without `sync_data()` for lower write
  latency.
- `CORTEXDB_AUDIT_MAC_KEY_HEX` is required for file-backed v2 audit logs and
  must be a 32-byte hex value. `CORTEXDB_AUDIT_MAC_KEY_ID` labels the MAC key
  in records and defaults to `local-audit-key` when omitted. Keep the key
  outside the audit log and pass it to verification through a key file.

## Record Fields

| Field | Meaning |
| --- | --- |
| `schema_version` | `cortexdb.audit.v2` for current local server audit records; `cortexdb.audit.v1` is legacy readable input. |
| `audit_event` | Event type, for example `http_response` or `llm_inference_decision`. |
| `audit_action` | Route category such as `read`, `write`, `search`, `verify`, `admin`, or `metrics`. |
| `chain_id`, `sequence`, `prev_hash`, `event_hash` | Local SHA-256 hash-chain metadata checked by `cortexdb audit --verify-chain`. |
| `mac_key_id`, `event_mac` | Key label and HMAC-SHA-256 event MAC for current `cortexdb.audit.v2` records. |
| `principal_id`, `auth_role`, `auth_agent_id` | Authenticated principal metadata when available; tokens are never stored. |
| `scope_decision` | `allowed`, `denied`, or `not_applicable`; raw scope labels are not stored. |
| `method`, `path`, `tenant`, `request_id` | HTTP route metadata without query strings. |
| `status`, `error_code`, `duration_ms`, `unix_time_ms` | Response outcome and timing metadata. |
| `accountability_receipt_hash` | Optional domain-separated BLAKE3 hash of an emitted `accountability_receipt.v1` object for JSON ContextPack/VERIFY responses when receipt signing is configured; included in `event_hash` and `event_mac` without storing the receipt body. |
| `llm` | Safe inference decision metadata; prompts, provider responses, and API keys are not stored. |

## Review And Export

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain --mac-key-file ./audit/audit-mac.key
cortexdb audit verify ./audit/http.jsonl --mac-key-file ./audit/audit-mac.key
cortexdb audit ./audit/http.jsonl --route /v1/cell --status 403
cortexdb audit-export-siem ./audit/http.jsonl ./audit/siem.jsonl --redaction-check --verify-chain --mac-key-file ./audit/audit-mac.key
```

The audit CLI filters by route, status, action, and tenant. Redaction checks
fail if body-like fields, query strings, bearer tokens, prompts, provider
responses, API keys, or secrets appear in records. Chain verification for v2
records requires the matching MAC key through `--mac-key-file`; without it,
keyed records fail verification instead of being accepted as hash-only events.

## Boundary

This is a local operational audit trail with SHA-256 hash-chain checks and
HMAC-SHA-256 event MACs. When signed JSON accountability receipts are emitted,
the audit record commits the receipt hash through `accountability_receipt_hash`.
It is not a compliance-certified immutable ledger, does not provide
vendor-managed SIEM delivery, and does not prevent an operator with the MAC key
from creating a new valid local chain.
