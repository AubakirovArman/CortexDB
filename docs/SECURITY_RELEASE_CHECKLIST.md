# Security Release Checklist

Use this checklist before labeling a CortexDB build as beta-security ready.

## Required Local Gates

```bash
make security-hardening-check
make openapi-contract-check
make sdk-contract-check
make backup-drill-check
make backup-offsite-check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
```

## Operator Checks

- Auth is configured with strong `admin` and `data` bearer tokens.
- Data tokens are scoped to persisted AgentViews where scope isolation matters.
- `CORTEXDB_AUTH_TOKENS_FILE` is used for local token rotation.
- Dashboard access is treated as administrative.
- `CORTEXDB_AUDIT_LOG_FILE` is enabled when route-level audit review is needed.
- `cortexdb audit <audit.jsonl> --summary --redaction-check` passes.
- `SECURITY_BETA_BASELINE.md` still separates implemented controls from beta
  and enterprise non-goals.
- Backups are validated by restore drills before being trusted.
- Tenant IDs are restricted to the documented safe character set.
- Public deployment terminates TLS at a trusted reverse proxy.

## Explicit Non-Goals For Core Alpha

- No built-in TLS or certificate lifecycle.
- No persisted enterprise RBAC administration store.
- No per-token quota accounting.
- No tamper-evident audit chain or SIEM exporter.
- No built-in encrypted backup or object-store upload adapter.
- No distributed security guarantees.
