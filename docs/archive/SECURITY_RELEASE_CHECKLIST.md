# Security Release Checklist

Use this checklist before labeling a CortexDB build as beta-security ready.

## Required Local Gates

```bash
make security-hardening-check
make security-gate-v2-check
make security-release-report-check
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
- `cortexdb audit <audit.jsonl> --summary --redaction-check --verify-chain`
  passes before trusting local chain-v1 audit files.
- `cortexdb audit-export-siem <audit.jsonl> <siem.jsonl> --redaction-check
  --verify-chain` passes before handing local JSONL exports to downstream
  tooling.
- `AUDIT_EXPORT_RETENTION_POLICY.md` still matches the implemented local
  export, retention, and redaction boundary.
- `SECURITY_BETA_BASELINE.md` still separates implemented controls from beta
  and enterprise non-goals.
- `SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md` still records release-blocking
  decisions for dynamic RBAC, per-token quotas, tamper-evident audit, encrypted
  backups, and remote object-store backups.
- Backups are validated by restore drills before being trusted.
- Tenant IDs are restricted to the documented safe character set.
- Public deployment terminates TLS at a trusted reverse proxy.

## Explicit Non-Goals For Core Alpha

- No built-in TLS or certificate lifecycle.
- No full enterprise RBAC administration system beyond the local policy-store
  controls and admin APIs.
- No distributed/global quota accounting beyond local per-principal guards.
- No compliance-grade immutable audit ledger, legal retention enforcement,
  external timestamping, or vendor-managed SIEM delivery.
- No KMS-backed encrypted backup custody or provider object-store upload
  adapter.
- No distributed security guarantees.

Any release note, README, API doc, dashboard copy, or SDK doc that claims one of
these non-goals must be blocked until the matching implementation and gate are
added.
