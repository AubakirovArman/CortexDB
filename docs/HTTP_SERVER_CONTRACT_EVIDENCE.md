# HTTP Server Contract Evidence

Last local HTTP contract and operations run: 2026-05-31, passed.

Run:

```bash
make http-contract-ops-check
```

Primary artifact:

```text
target/http-contract-ops/report.json
```

## Coverage

This gate covers:

- route-level auth role behavior;
- typed error-code taxonomy and OpenAPI contract;
- `x-request-id` response propagation and audit correlation;
- rate-limit `429 rate_limited` behavior;
- exact-origin CORS behavior;
- audit redaction for route-level JSONL audit records.

## Latest Local Checks

```text
request_id_propagation: true
auth_roles: true
rate_limit: true
cors: true
audit_redaction: true
typed_errors: true
```

## Boundary

This is a local Core Alpha HTTP contract gate. It does not prove enterprise
identity integration, dynamic RBAC policy storage, multi-node security, or
tamper-evident audit trails.
