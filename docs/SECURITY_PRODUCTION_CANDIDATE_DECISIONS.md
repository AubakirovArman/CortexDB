# Security Production-Candidate Decisions

Status: production-candidate boundary for local single-node releases.

This register prevents CortexDB from relying on implicit security assumptions.
Every item below is either implemented for the current local single-node
boundary or explicitly deferred with a release-blocking rule.

## Decision Matrix

| Area | Production-candidate decision | Implemented now | Deferred work | Release-blocking rule |
| --- | --- | --- | --- | --- |
| Dynamic RBAC | Defer full enterprise RBAC to beta+. | Static `admin`/`data` route roles, file-backed token rotation, optional token-to-AgentView binding, admin policy mutations, disabled principal lifecycle, and redacted durable policy-cell mirror. | Role hierarchy, user sessions, external identity, compliance workflows. | Block any release or docs that claim enterprise RBAC, dynamic user management, or external identity integration. |
| Per-token quotas | Support local policy-store principal quotas for the single-node boundary. | Process-wide limits plus policy-store request/minute, body-byte/minute, and actor-queue quotas with typed `rate_limited` errors and aggregate metrics. | Distributed, tenant-aware, and route-class quota accounting. | Block distributed or multi-node quota claims unless an API gateway or future quota service owns them. |
| Tamper-evident audit | Support a local tamper-evident audit chain for JSONL route audit files. | File-backed records carry `chain_id`, monotonic `sequence`, `prev_hash`, and `event_hash`; `cortexdb audit --verify-chain`, `cortexdb audit verify <file>`, and SIEM export tests validate continuity. | Compliance-certified immutable ledger, external timestamping, and vendor-managed SIEM delivery. | Block compliance-certified audit-ledger claims until external immutability and audit review workflows are implemented. |
| Encrypted backup | Support local passphrase encrypted backup archives for the single-node beta boundary. | `cortexdb backup-encrypted`, `cortexdb restore-encrypted`, wrong-passphrase rejection, corrupt-ciphertext rejection, and restore validation. | KMS/key-provider interface, external authenticated-encryption proof, remote object-store restore, and compliance custody workflow. | Block KMS, managed backup, or compliance-grade encryption claims until provider-backed encrypted restore drills exist. |
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
