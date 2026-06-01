# Security Hardening Evidence

Last local security hardening run: 2026-06-01, passed.

Run:

```bash
make security-hardening-check
```

Primary artifact:

```text
target/security-hardening/report.json
```

## Coverage Matrix

| Area | Current status |
| --- | --- |
| Persisted auth policy store | File-backed token rotation and JSON principal policy store are implemented through `CORTEXDB_AUTH_TOKENS_FILE` and `CORTEXDB_AUTH_POLICY_STORE_FILE`; full enterprise RBAC administration remains future work. |
| Per-token quotas | Process-wide rate limit is implemented; user/token-aware quotas remain beta work. |
| Tamper-evident audit chain | Route-level JSONL audit plus redaction checks are implemented; tamper-evident chain/SIEM export remains beta work. |
| Encrypted backup support | Design exists in `ENCRYPTED_BACKUPS_DESIGN.md`; current backup/restore/offsite staging are local and unencrypted. |
| Remote object-store backup | Local offsite staging exists; provider-backed object-store upload remains future work. |
| Secret rotation docs | Token-file rotation is documented in `AUTH.md` and `SECURITY_THREAT_MODEL.md`. |
| Dashboard auth hardening | Dashboard remains admin-only; data tokens cannot access `/dashboard`. |
| Malicious ingestion tests | AgentView scoped ingestion bypass attempts are denied and do not echo body content. |
| Log redaction tests | Audit JSONL records contain route metadata, not query strings or request bodies. |
| Security release checklist | `SECURITY_RELEASE_CHECKLIST.md` lists required local checks and non-goals. |
| Beta security baseline | `SECURITY_BETA_BASELINE.md` separates implemented controls from RBAC, quota, audit-chain, encrypted-backup, and dashboard hardening work. |
| Production-candidate decisions | `SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md` records release-blocking decisions for RBAC, quotas, audit-chain, encrypted backups, and remote object-store backups. |

## Boundary

This evidence closes the Core Alpha security-hardening pass by proving current
guards and documenting what is explicitly not an enterprise security guarantee.
It does not claim full RBAC, per-user quota, encrypted backup, or
tamper-evident audit compliance.

## Latest Local Checks

```text
persisted_auth_policy_store: true
per_token_quota_boundary: true
audit_redaction: true
tamper_evident_audit_boundary: true
encrypted_backup_boundary: true
remote_backup_boundary: true
secret_rotation_docs: true
dashboard_auth_hardening: true
malicious_ingestion_tests: true
security_release_checklist: true
security_beta_baseline: true
production_candidate_decisions: true
```
