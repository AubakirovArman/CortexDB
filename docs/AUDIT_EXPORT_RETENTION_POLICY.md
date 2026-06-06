# Audit Export And Retention Policy

Status: local production-boundary policy for Epic 95.

Machine-readable policy:
[`AUDIT_EXPORT_RETENTION_POLICY.json`](AUDIT_EXPORT_RETENTION_POLICY.json).

## Export Path

CortexDB supports local audit review and local SIEM-normalized JSONL export:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
cortexdb audit verify ./audit/http.jsonl
cortexdb audit-export-siem ./audit/http.jsonl ./audit/siem.jsonl --redaction-check --verify-chain
```

`audit-export-siem` writes one JSON object per line with schema
`cortexdb.siem.audit.v1`. The export preserves route metadata, principal
metadata, request id, status, duration, and local audit-chain metadata. It does
not ship records to a vendor-managed SIEM service.

## Retention Classes

| Class | Boundary | Payloads or tokens | Retention |
| --- | --- | --- | --- |
| `local_audit_jsonl` | Local filesystem audit sink | No | Operator-defined |
| `siem_export_jsonl` | Local export file before downstream shipping | No | Operator-defined |
| `local_only_raw_security_debug` | Ignored local debugging paths | May contain sensitive material | Do not publish; delete after local incident review |

Core Alpha and the current production boundary do not enforce legal retention
schedules. Operators must define their own storage duration, rotation cadence,
and downstream archival policy.

## Redaction Policy

Audit files and SIEM exports must not contain request bodies, full query
strings, bearer tokens, prompts, provider responses, API keys, or secrets.

Forbidden field names include:

```text
authorization
auth_header
bearer_token
body
payload
query
query_string
prompt
provider_response
api_key
secret
```

Safe fields include route/action metadata, status, duration, tenant,
`request_id`, principal id, role, optional AgentView id, chain metadata, and
safe LLM decision metadata such as provider, model, outcome, citation count, and
guardrail reason. `llm.prompt_body_logged` and `llm.secrets_logged` must remain
false for exported audit records.

## Required Gates

```bash
make audit-chain-check
make audit-export-retention-check
```

The retention gate validates this policy, checks the SIEM export implementation
and tests, and writes:

```text
target/audit-export-retention/report.json
```

## Boundary

This policy is a local audit-export and retention boundary. It is not a
compliance-certified immutable ledger, external timestamping service, legal
retention schedule, or vendor-managed SIEM delivery guarantee.
