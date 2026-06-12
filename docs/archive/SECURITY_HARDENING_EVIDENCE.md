# Security Hardening Evidence

Last local security hardening run: 2026-06-06, passed.

Run:

```bash
make security-hardening-check
make security-gate-v2-check
make security-release-report-check
```

Primary artifact:

```text
target/security-hardening/report.json
target/security-gate-v2/report.json
target/security-release/report.json
```

## Release Security Hardening Report

Current release report gate:

```bash
make security-release-report-check
```

The gate writes:

```text
target/security-release/report.json
```

It requires the following lower-level reports to pass before the release
security report can be trusted:

```text
target/security-gate-v2/report.json
target/compliance-boundary/report.json
```

The release-level gate is intentionally an evidence aggregator. It does not
replace the focused tests; it proves that the current release has a single
security-hardening report tied to:

- `make security-gate-v2-check`
- `make compliance-boundary-check`
- `make security-hardening-check`
- `make rbac-policy-store-check`
- `make quota-policy-check`
- `make audit-chain-check`
- `make audit-export-retention-check`

## Coverage Matrix

| Area | Current status |
| --- | --- |
| Persisted auth policy store | File-backed token rotation and JSON principal policy store are implemented through `CORTEXDB_AUTH_TOKENS_FILE` and `CORTEXDB_AUTH_POLICY_STORE_FILE`; admin-only local mutation routes support upsert, disable, and rollback; optional `capabilities` restrict a valid role to explicit API action classes and invalid capability lists fail closed; explicit `cortexdb.auth_policy.v0` token-list stores migrate into v1 in memory while unknown schemas fail closed; `make rbac-policy-store-check` verifies the local evidence gate; full enterprise RBAC administration remains future work. |
| Auth policy review | `cortexdb auth-review` shows local policy-store/token-file principals, roles, AgentView bindings, quotas, and disabled state while redacting token values. |
| Per-principal quotas | Process-wide rate limit and policy-store `request_quota_per_minute` are implemented; `make quota-policy-check` verifies the local evidence gate; route-class and distributed quotas remain future work. |
| Principal-aware audit metadata | Authenticated route-level JSONL audit records include `principal_id`, `auth_role`, and `auth_agent_id` without storing bearer tokens. |
| Tamper-evident audit chain | File-backed route audit records include local chain metadata and `cortexdb audit --verify-chain` detects local deletion, reordering, and metadata edits; `make audit-chain-check` verifies the local evidence gate; compliance-grade ledger and vendor-managed SIEM delivery remain future work. |
| SIEM audit export | `cortexdb audit-export-siem` exports normalized local JSONL with principal and audit-chain metadata after optional redaction and chain checks; vendor-managed SIEM delivery remains future work. |
| Audit export retention policy | `AUDIT_EXPORT_RETENTION_POLICY.md` and `.json` define local audit/SIEM export retention classes, forbidden redaction fields, safe metadata fields, and the `make audit-export-retention-check` evidence gate. |
| Compliance boundary mapping | `COMPLIANCE_BOUNDARY_MAPPING.md` and `make compliance-boundary-check` document local evidence controls and explicitly state that no external compliance framework is currently certified. |
| Encrypted backup support | Local passphrase encrypted backup MVP exists through `backup-encrypted`/`restore-encrypted`, wrong-passphrase and corrupt-ciphertext tests, and `make encrypted-backup-check`; KMS/compliance custody remains future work. |
| Remote object-store backup | Local offsite staging exists; provider-backed object-store upload remains future work. |
| Secret rotation docs | Token-file rotation is documented in `AUTH.md` and `SECURITY_THREAT_MODEL.md`. |
| Dashboard auth hardening | Dashboard remains admin-only; data tokens cannot access `/dashboard`. |
| Malicious ingestion tests | AgentView scoped ingestion bypass attempts are denied and do not echo body content. |
| Log redaction tests | Audit JSONL records contain route metadata, not query strings or request bodies. |
| Security release checklist | `SECURITY_RELEASE_CHECKLIST.md` lists required local checks and non-goals. |
| Beta security baseline | `SECURITY_BETA_BASELINE.md` separates implemented local controls from enterprise RBAC, distributed quotas, compliance audit ledger, KMS-backed encrypted backup, and dashboard hardening work. |
| Production-candidate decisions | `SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md` records release-blocking decisions for RBAC, quotas, audit-chain, encrypted backups, and remote object-store backups. |

## Boundary

This evidence closes the Core Alpha security-hardening pass by proving current
guards and documenting what is explicitly not an enterprise security guarantee.
It does not claim full RBAC, distributed quota, KMS-backed encrypted backup, or
tamper-evident audit compliance. It does not claim SOC 2, ISO 27001, HIPAA,
GDPR, legal-grade verification, or other external compliance certification.

## Remaining Risks

These risks are still explicit non-claims for this release:

- external identity integration is not production-enabled;
- enterprise compliance certification is not claimed;
- managed-cloud security is not claimed;
- distributed authorization correctness is not claimed;
- KMS-backed encrypted backup custody is not implemented;
- provider-backed object-store backup is not implemented;
- compliance-grade immutable audit ledger and legal retention enforcement are
  not implemented;
- TLS, mTLS, and certificate lifecycle remain deployment-boundary concerns.

## Latest Local Checks

```text
persisted_auth_policy_store: true
rbac_policy_store_gate: true
auth_policy_review: true
per_token_quota_boundary: true
per_principal_quota: true
quota_policy_gate: true
audit_principal_metadata: true
audit_chain_foundation: true
audit_chain_gate: true
siem_audit_export: true
audit_export_retention_gate: true
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
