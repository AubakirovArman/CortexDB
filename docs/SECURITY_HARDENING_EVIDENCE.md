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
| Auth policy review | `cortexdb auth-review` shows local policy-store/token-file principals, roles, AgentView bindings, quotas, and disabled state while redacting token values. |
| Per-principal quotas | Process-wide rate limit and policy-store `request_quota_per_minute` are implemented; route-class and distributed quotas remain future work. |
| Principal-aware audit metadata | Authenticated route-level JSONL audit records include `principal_id`, `auth_role`, and `auth_agent_id` without storing bearer tokens. |
| Tamper-evident audit chain | File-backed route audit records include local chain metadata and `cortexdb audit --verify-chain` detects local deletion, reordering, and metadata edits; compliance-grade ledger and vendor-managed SIEM delivery remain future work. |
| SIEM audit export | `cortexdb audit-export-siem` exports normalized local JSONL with principal and audit-chain metadata after optional redaction and chain checks; vendor-managed SIEM delivery remains future work. |
| Compliance boundary mapping | `COMPLIANCE_BOUNDARY_MAPPING.md` and `make compliance-boundary-check` document local evidence controls and explicitly state that no external compliance framework is currently certified. |
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
It does not claim full RBAC, distributed quota, encrypted backup, or
tamper-evident audit compliance. It does not claim SOC 2, ISO 27001, HIPAA,
GDPR, legal-grade verification, or other external compliance certification.

## Latest Local Checks

```text
persisted_auth_policy_store: true
auth_policy_review: true
per_token_quota_boundary: true
per_principal_quota: true
audit_principal_metadata: true
audit_chain_foundation: true
siem_audit_export: true
compliance_boundary_mapping: true
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
