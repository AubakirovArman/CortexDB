# CortexDB Security Threat Model

This document describes the Core Alpha security model as implemented today. It
is intentionally conservative: anything not listed as an explicit control should
be treated as future work.

## Assets

| Asset | Why It Matters |
| --- | --- |
| WAL and storage files | Durable source of user data and recovery state. |
| Cell payloads and metadata | May contain confidential project, agent, or document context. |
| AgentView policy | Defines which brains, scopes, modes, and memory types an agent can use. |
| ContextPack output | Can leak evidence, citations, or scoped private data to an agent. |
| HTTP auth token | Grants route-class access as a static `admin` or `data` token. |
| Tenant realm paths | Separate local database directories under one server root. |
| SDK/API contracts | Define what external callers can rely on and what errors reveal. |

## Trust Boundaries

1. **Local database files**: trusted only by the process that owns the database
   lock. Other processes must not mutate files concurrently.
2. **HTTP server boundary**: optional Bearer token auth gates requests when
   configured. Without `CORTEXDB_AUTH_TOKEN` or `CORTEXDB_AUTH_TOKENS`, the
   server should be treated as unauthenticated.
3. **Tenant realm boundary**: tenant IDs select subdirectories below
   `root/realms/`. This is path isolation, not a full authorization system.
4. **AgentView boundary**: AQL binding and bitmap execution must only narrow
   allowed scopes and candidates.
5. **Storage recovery boundary**: corrupt, partial, or mismatched storage files
   must fail closed or stop at a safe recovery point.
6. **SDK/client boundary**: clients must receive stable typed errors rather than
   internal implementation details.

## Threats And Current Controls

| Threat | Current Control | Status |
| --- | --- | --- |
| HTTP request without credentials | Optional `CORTEXDB_AUTH_TOKEN` or `CORTEXDB_AUTH_TOKENS` Bearer token check. | Implemented when any token is configured. |
| Data token accesses admin routes | Static `admin`/`data` token roles deny data tokens from dashboard, stats, validation, flush, compact, and metrics routes. | Implemented and tested. |
| Authenticated HTTP data request bypasses AgentView scope policy | Optional legacy `CORTEXDB_AUTH_AGENT_ID` or per-token `role:token:agent_id` mappings load persisted AgentViews for scope-bound data routes. | Implemented and tested. |
| Path traversal through tenant ID | Percent-decoded tenant validation, limited charset and length. | Implemented and tested. |
| Oversized request body | Axum request body limit. | Implemented and tested. |
| Concurrent writers corrupting local files | Database lock file with owner metadata. | Implemented and tested. |
| AQL query expands permissions | Binder checks AgentView and runtime AgentAllowed mask intersects results. | Implemented and tested. |
| Policy denial leaks internal names | Safe policy messages avoid exposing brain/scope names. | Implemented for known policy paths. |
| Corrupt WAL payload/header | CRC checks and strict/best-effort recovery behavior. | Implemented and tested. |
| Corrupt segment/index/manifest | CRC checks and validation/open failures. | Implemented and tested. |
| Partial WAL tail after crash | Reader reports safe truncate offset. | Implemented and tested. |
| Stale lock after crash | Manual/CLI unlock and stale lock policy. | Implemented for local operations. |
| API error drift | Typed `RouterError`, OpenAPI checks, snapshot tests. | Implemented and gated. |
| SDK contract drift | Python, TypeScript, and Rust live SDK smoke checks. | Implemented and gated. |
| Browser cross-origin API calls | CORS disabled by default; optional exact-origin allowlist via `CORTEXDB_CORS_ALLOW_ORIGIN`. | Implemented for one trusted origin. |
| Request floods against the local API | Optional process-wide fixed-window limit via `CORTEXDB_RATE_LIMIT_PER_MINUTE`. | Implemented as a coarse Core Alpha guard. |
| Missing operational access trail | Optional structured HTTP audit events via `CORTEXDB_AUDIT_LOG`; optional synced JSONL file sink via `CORTEXDB_AUDIT_LOG_FILE`. | Implemented for route-level events. |

## Out Of Scope For Core Alpha

The following are not production security guarantees yet:

- TLS/mTLS and certificate management.
- User identity, sessions, dynamic RBAC policy stores, org roles, or external identity providers.
- Token rotation workflow or persisted auth policy management.
- Per-token quotas or distributed rate limiting.
- Multi-origin, wildcard, or per-token CORS policies.
- Tamper-evident audit trails or SIEM export.
- At-rest encryption or envelope key management.
- Encrypted backups.
- Secret rotation workflow.
- Side-channel analysis of timing, file sizes, or query result counts.
- Production-grade distributed consensus security.

## Required Deployment Posture

For any non-local deployment:

1. Set `CORTEXDB_AUTH_TOKEN` to a strong random admin value, or configure
   `CORTEXDB_AUTH_TOKENS` with static `admin`/`data` token roles.
2. For scoped API deployments, bind data tokens with
   `CORTEXDB_AUTH_TOKENS="data:token:agent_id"` so data routes enforce
   readable/writable scope policy.
3. Terminate TLS in a trusted reverse proxy.
4. Restrict network access to trusted clients.
5. Run one tenant per isolated realm or process when data separation matters.
6. Store database files on a trusted local filesystem.
7. Back up the database directory only after stopping writes or using a future
   backup command with consistency guarantees.
8. Treat dashboard access as administrative.
9. Enable `CORTEXDB_CORS_ALLOW_ORIGIN` only for one trusted browser origin;
   keep it unset for local CLI/SDK-only deployments.
10. Set `CORTEXDB_RATE_LIMIT_PER_MINUTE` for exposed local deployments; use an
   API gateway or reverse proxy for user-aware quotas.
11. Set `CORTEXDB_AUDIT_LOG=true` when route-level access auditing is required;
    export `tracing` output to your process supervisor or log pipeline.
12. Set `CORTEXDB_AUDIT_LOG_FILE` when route-level audit events should also be
    persisted to a local JSONL file.

## Error Disclosure Policy

External errors should use stable codes and safe messages:

- `unauthorized` for missing or invalid Bearer tokens;
- `invalid_tenant` for tenant path/charset violations;
- `permission_denied` for AgentView and policy denials;
- `invalid_aql` for parse or non-policy bind errors;
- `database_busy` for actor queue pressure or lock conflicts;
- `storage_corruption` for storage invariants and corrupted files.

Errors must not reveal private scope names, brain names, filesystem paths outside
the database root, secrets, or full internal backtraces.

## Security Test Gates

Current relevant gates:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
make openapi-check
make openapi-contract-check
make sdk-contract-check
```

Security-sensitive test areas include:

- `crates/cortex-server/src/tests/security_tests.rs`
- `crates/cortex-server/src/tests/snapshot_api_tests.rs`
- `crates/cortex-engine/tests/validation_tests.rs`
- `crates/cortex-engine/tests/corruption_matrix.rs`
- `crates/cortex-engine/tests/recovery_modes.rs`
- `crates/cortex-engine/tests/persisted_index_tests.rs`

## Beta Security Backlog

1. Add per-token quotas and user-aware rate limiting.
2. Expand CORS beyond the current single exact-origin allowlist only after
   adding user/RBAC-aware authorization.
3. Extend static `CORTEXDB_AUTH_TOKENS` into persisted auth policy and rotation
   workflows.
4. Extend the JSONL audit sink into tamper-evident audit trails and SIEM export.
5. Add backup/restore with integrity verification.
6. Add documented secret rotation.
7. Add deployment hardening guide for reverse proxy and systemd/container use.
8. Split operational admin routes from data routes.
9. Add security-focused release checklist.
10. Review replication and dashboard paths before claiming beta readiness.
