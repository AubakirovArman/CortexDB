# Compliance Boundary Mapping

Status: local evidence boundary, not external certification.
Schema: `cortexdb.compliance_boundary.v1`.

## Boundary Statement

CortexDB currently supports local controls that can help an internal security or
compliance review. The repository does not claim SOC 2, ISO 27001, HIPAA, GDPR,
legal-grade verification, or any other external compliance certification.

Supported certified frameworks today: none.

Any public compliance claim must name:

1. the framework;
2. the exact control objective;
3. the CortexDB evidence artifact;
4. the remaining operator responsibility;
5. the external review or certification status.

If any of those five fields are missing, the claim must remain a non-goal.

## Current Local Evidence Controls

| Control area | Current evidence | Boundary |
| --- | --- | --- |
| Route authorization | Static `admin` and `data` route roles; dashboard/admin routes require admin. | Not enterprise RBAC. |
| Local policy store | `CORTEXDB_AUTH_POLICY_STORE_FILE` supports explicit principals, roles, disabled state, optional AgentView binding, and local quotas. | Not a mutable RBAC admin system. |
| Policy review | `cortexdb auth-review` reports redacted local policy-store/token-file roles, principals, AgentView bindings, quotas, and disabled state. | Local file review only; not remote governance. |
| Scope enforcement | AgentView-bound tokens restrict readable/writable scopes on data routes. | Not external identity or org-wide role hierarchy. |
| Request quotas | Process-wide limit plus policy-store per-principal fixed-window quota. | Not distributed quota state. |
| Audit redaction | HTTP audit records contain route metadata and principal metadata, not request bodies, query strings, or bearer tokens. | Not full data-loss-prevention. |
| Audit chain | Local JSONL audit records include `chain_id`, `sequence`, `prev_hash`, and `event_hash`; `cortexdb audit --verify-chain` detects local continuity failures. | Not a compliance-grade immutable ledger. |
| SIEM export | `cortexdb audit-export-siem` writes normalized local JSONL with principal and audit-chain metadata after optional redaction and chain checks. | Not vendor-managed SIEM delivery. |
| Backup validation | Local backup, restore, offsite staging, and restore drills validate storage before trust. | Not encrypted backup or managed disaster recovery. |
| Public claim guard | `make public-claims-check` blocks wording drift for local single-node claims. | Not a legal review. |

## Explicit Non-Claims

CortexDB does not currently claim:

- SOC 2 compliance;
- ISO 27001 certification;
- HIPAA compliance;
- GDPR processor or controller compliance;
- legal-grade fact verification;
- compliance-grade immutable audit ledger;
- external identity provider compliance;
- managed-cloud compliance inheritance;
- distributed authorization consistency;
- vendor-managed SIEM delivery.

## Promotion Requirements

Before any framework-specific compliance claim can be promoted, the repository
must add:

1. a named framework mapping;
2. control-by-control evidence links;
3. operator responsibility matrix;
4. audit export retention policy;
5. key management and backup encryption boundary;
6. policy mutation and rollback evidence;
7. external identity lifecycle evidence, if identity is in scope;
8. independent review status.

## Required Local Gate

Run:

```bash
make compliance-boundary-check
```

The gate checks this document and public security docs for the local evidence
boundary. Passing the gate only means the boundary is documented. It does not
mean CortexDB is certified for any external compliance framework.
