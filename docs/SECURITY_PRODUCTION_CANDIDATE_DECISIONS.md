# Security Production-Candidate Decisions

Status: production-candidate boundary for local single-node releases.

This register prevents CortexDB from relying on implicit security assumptions.
Every item below is either implemented for the current local single-node
boundary or explicitly deferred with a release-blocking rule.

## Decision Matrix

| Area | Production-candidate decision | Implemented now | Deferred work | Release-blocking rule |
| --- | --- | --- | --- | --- |
| Dynamic RBAC | Defer full enterprise RBAC to beta+. | Static `admin`/`data` route roles, file-backed token rotation, optional token-to-AgentView binding, admin policy mutations, disabled principal lifecycle, and redacted durable policy-cell mirror. | Role hierarchy, user sessions, external identity, compliance workflows. | Block any release or docs that claim enterprise RBAC, dynamic user management, or external identity integration. |
| Per-token quotas | Defer token-aware quota accounting to beta. | Optional process-wide fixed-window rate limit with typed `rate_limited` errors. | Safe token fingerprints, per-token/tenant/route quotas, distributed quota state. | Block multi-user quota claims unless an API gateway or reverse proxy owns quotas outside CortexDB. |
| Tamper-evident audit | Defer tamper-evident audit-chain to beta. | JSONL route audit, redaction checks, CLI audit summary/filter tooling. | Hash chain, sequence continuity verification, SIEM export. | Block compliance/audit-integrity claims until `cortexdb audit --verify-chain` exists and is tested. |
| Encrypted backup | Defer built-in encrypted backups to beta. | Local backup, restore drill, validated local offsite staging. | Authenticated encryption, key-provider interface, encrypted restore drill. | Block encrypted-at-rest backup claims unless encryption is provided by the filesystem or external backup tool. |
| Remote object-store backup | Defer provider-backed upload to future product work. | Local `backup-offsite-stage` creates a validated staging copy for external tooling. | S3/GCS/Azure adapters, remote manifest, remote restore drill. | Block remote durability or managed backup claims until provider-backed upload/restore gates exist. |

## Required Gates

Before a production-candidate local single-node label:

```bash
make security-hardening-check
make production-candidate-check
make backup-drill-check
make backup-offsite-check
make openapi-contract-check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
```

## Claim Boundary

Allowed wording:

- local single-node auth roles;
- file-backed token rotation;
- AgentView-scoped data tokens;
- route-level JSONL audit with redaction checks;
- local backup/restore/offsite-staging validation.

Forbidden wording until future gates land:

- enterprise RBAC;
- per-user or per-token quota guarantees;
- tamper-evident audit compliance;
- built-in encrypted backups;
- built-in remote object-store backups;
- managed-service security guarantees.
