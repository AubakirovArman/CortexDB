# Security Beta Baseline

This document turns the Core Alpha security backlog into a beta-track baseline.
It separates controls that are implemented today from controls that remain
design-only, so release gates can block overclaims.

## Current Implemented Controls

| Area | Implemented now |
| --- | --- |
| Static route roles | `admin` and `data` bearer-token roles are enforced by route class. |
| Token rotation | `CORTEXDB_AUTH_TOKENS_FILE` is re-read per request and fails closed on invalid content. |
| Policy review tooling | `cortexdb auth-review` reports local policy-store/token-file roles, principals, AgentView bindings, quotas, and disabled state without printing token values. |
| AgentView scoping | Token-bound AgentViews restrict readable/writable scopes on data routes. |
| Tenant path safety | Tenant IDs are percent-decoded, length-limited, and path-traversal checked. |
| Request limits | Process-wide and policy-store per-principal fixed-window request limits can return typed `rate_limited` errors. |
| Audit redaction | HTTP audit events store route metadata, status, tenant, request id, duration, and authenticated principal metadata, not query strings, request bodies, or bearer tokens. |
| Audit chain foundation | File-backed audit records include `chain_id`, `sequence`, `prev_hash`, and `event_hash`; `cortexdb audit --verify-chain` checks local continuity; `cortexdb audit-export-siem` exports normalized JSONL for downstream SIEM ingestion. |
| Compliance boundary | `COMPLIANCE_BOUNDARY_MAPPING.md` states that current controls are local evidence only and that CortexDB has no external compliance certification today. |
| Dashboard gate | `/dashboard` is an admin route; data tokens are denied. |
| Backup validation | Local backup, restore, offsite staging, and restore drills validate storage before trust. |

## Beta Implementation Backlog

### 1. RBAC Policy Store

Goal: continue from the implemented JSON principal policy store toward a full
enterprise RBAC administration layer without breaking the current token
contract.

Plan:

1. Keep `CORTEXDB_AUTH_POLICY_STORE_FILE` as the local durable JSON policy-store
   entry point for principals, roles, disabled principals, and optional
   AgentView binding.
2. Keep `cortexdb auth-review` as the read-only local policy preview command
   for roles, principals, optional AgentView binding, quotas, and disabled
   state.
3. Persist principal, credential, role-binding, and AgentView-binding records in
   a system scope.
4. Add an `AuthPolicyResolver` abstraction that can read from static options,
   token files, or the policy store.
5. Fail closed when policy-store records are corrupt, expired, disabled, or
   inconsistent.
6. Add write APIs only after audit review, backup/restore, and rollback
   behavior are documented.

Beta gate:

- disabled principals cannot authenticate;
- local policy review output never contains raw token material;
- data/auditor/operator roles cannot access admin-only routes;
- AgentView-bound principals cannot read or write forbidden scopes;
- policy-store corruption fails closed.

### 2. Per-Principal Quotas

Goal: continue from the implemented local per-principal fixed-window guard
toward route-aware and distributed quota accounting.

Plan:

1. Keep `request_quota_per_minute` on policy-store principals as the local
   fixed-window quota entry point.
2. Keep raw tokens out of quota keys, metrics, audit records, logs, and
   reports.
3. Add route-class and tenant dimensions after the local principal counter is
   stable.
4. Support fixed-window limits for beta; reserve sliding-window or
   token-bucket behavior for production tuning.
5. Return the existing typed `rate_limited` response on quota exhaustion.
6. Document that distributed quotas remain out of scope until real replicated
   state exists.

Beta gate:

- one principal exhausting quota does not block another principal;
- denied quota responses do not expose raw token material;
- data and admin route classes can be configured independently.

### 3. Tamper-Evident Audit Chain

Goal: extend JSONL route audit into a locally verifiable hash chain.

Plan:

1. Keep `prev_hash`, `event_hash`, `chain_id`, and `sequence` fields on
   file-backed audit records.
2. Keep hashing canonical route metadata only; do not hash or persist request
   bodies, full query strings, bearer tokens, or secrets.
3. Keep `cortexdb audit --verify-chain` validating sequence continuity and hash
   integrity.
4. Upgrade hash and export policy only with a documented compatibility plan if
   a compliance-grade audit ledger is promoted.
5. Keep `cortexdb audit-export-siem` as the local normalized JSONL adapter and
   add vendor-specific SIEM adapters only after schema review.

Beta gate:

- deleting, reordering, or editing a JSONL audit event is detected;
- chain verification can run offline;
- normalized SIEM export preserves principal and chain metadata without adding
  bodies, query strings, or tokens;
- redaction checks still pass for malicious ingestion failures.

### 4. Encrypted Backups

Goal: add optional encrypted backup bundles while preserving current restore
drill semantics.

Plan:

1. Keep unencrypted local backups as the Core Alpha compatibility path.
2. Add encrypted backup commands or flags that produce authenticated encrypted
   payloads and explicit encryption metadata.
3. Start with a local operator-managed key provider for deterministic tests.
4. Add a provider trait for KMS/secret-manager integrations later.
5. Restore into a temporary directory, validate storage, then atomically publish
   the restored target.

Beta gate:

- encrypted backup restore drill succeeds;
- corrupted ciphertext or authentication tag fails before writing target data;
- reports and audit events never expose key material.

### 5. Dashboard Auth Hardening

Goal: keep the dashboard useful for local operators while preventing it from
becoming an accidental data-token administration surface.

Plan:

1. Keep `/dashboard` and dashboard assets admin-only when auth is configured.
2. Keep the dashboard local read-only switch as client-side accident
   prevention, not an authorization boundary.
3. Add route-level tests for dashboard admin/data token behavior and static
   asset behavior.
4. Add visible operator warnings when auth is disabled.
5. Do not add RBAC mutation UI before the policy store and audit-chain work are
   stable.

Beta gate:

- data tokens cannot access dashboard HTML or assets;
- dashboard mutation requests still require server-side admin authorization;
- auth-disabled deployments are clearly labeled as local-only.

## Release Blocking Rule

`make rbac-policy-store-check`, `make quota-policy-check`,
`make audit-chain-check`, and `make security-hardening-check` must fail if the
repository loses evidence for:

- auth role tests;
- tenant validation tests;
- typed error contract tests;
- audit redaction tests;
- malicious ingestion denial tests;
- dashboard admin-only tests;
- documented boundaries for RBAC, per-token quotas, audit-chain, and encrypted
  backups;
- local reports for the RBAC policy store, quota policy, and audit-chain gates.

Passing this gate means the beta security baseline is documented and locally
reproducible. It does not mean CortexDB has external security certification,
managed-cloud identity, distributed authorization, or encrypted backups in the
current build.

Run `make compliance-boundary-check` before any release note or README wording
that mentions compliance. That gate verifies the current no-certification
boundary and local evidence map.
