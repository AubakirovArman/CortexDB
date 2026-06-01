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
| HTTP auth token | Grants route-class access as a static or file-backed `admin` or `data` token. |
| Tenant realm paths | Separate local database directories under one server root. |
| SDK/API contracts | Define what external callers can rely on and what errors reveal. |

## Trust Boundaries

1. **Local database files**: trusted only by the process that owns the database
   lock. Other processes must not mutate files concurrently.
2. **HTTP server boundary**: optional Bearer token auth gates requests when
   configured. Without `CORTEXDB_AUTH_TOKEN`, `CORTEXDB_AUTH_TOKENS`, or
   `CORTEXDB_AUTH_TOKENS_FILE`, the server should be treated as
   unauthenticated.
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
| HTTP request without credentials | Optional `CORTEXDB_AUTH_TOKEN`, `CORTEXDB_AUTH_TOKENS`, or `CORTEXDB_AUTH_TOKENS_FILE` Bearer token check. | Implemented when any token is configured. |
| Data token accesses admin routes | Static `admin`/`data` token roles deny data tokens from dashboard, stats, validation, flush, compact, and metrics routes. | Implemented and tested. |
| Token rotation requires process restart | `CORTEXDB_AUTH_TOKENS_FILE` is re-read for every request and fails closed if missing, empty, or invalid. | Implemented and tested. |
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
| Request floods against the local API | Optional process-wide fixed-window limit via `CORTEXDB_RATE_LIMIT_PER_MINUTE`; optional policy-store `request_quota_per_minute` limits per principal. | Implemented as local Core Alpha guards. |
| Missing operational access trail | Optional structured HTTP audit events via `CORTEXDB_AUDIT_LOG`; optional synced JSONL file sink via `CORTEXDB_AUDIT_LOG_FILE`; `cortexdb audit` reviews JSONL files with route/status/action/tenant filters, redaction checks, and local chain verification. | Implemented for route-level events. |
| Unvalidated local backups | `cortexdb backup`, `restore`, `backup-drill`, `backup-prune`, and `backup-offsite-stage` validate source and restored copies. | Implemented for local filesystem/offsite-staging workflows. |

## Out Of Scope For Core Alpha

The following are not production security guarantees yet:

- TLS/mTLS and certificate management.
- User identity, sessions, dynamic RBAC policy stores, org roles, or external identity providers.
- Persisted auth policy management beyond local file-backed token rotation.
- Per-token quotas by route class or distributed rate limiting.
- Multi-origin, wildcard, or per-token CORS policies.
- Compliance-grade audit trails or SIEM export beyond the local audit-chain
  verification foundation.
- At-rest encryption or envelope key management.
- Encrypted backups or built-in remote object-store upload.
- Secret rotation workflow.
- Side-channel analysis of timing, file sizes, or query result counts.
- Production-grade distributed consensus security.

## Required Deployment Posture

For any non-local deployment:

1. Set `CORTEXDB_AUTH_TOKEN` to a strong random admin value, configure
   `CORTEXDB_AUTH_TOKENS` with static `admin`/`data` token roles, or configure
   `CORTEXDB_AUTH_TOKENS_FILE` with one `role:token[:agent_id]` policy per
   line.
2. For scoped API deployments, bind data tokens with
   `CORTEXDB_AUTH_TOKENS="data:token:agent_id"` so data routes enforce
   readable/writable scope policy.
3. Terminate TLS in a trusted reverse proxy.
4. Restrict network access to trusted clients.
5. Run one tenant per isolated realm or process when data separation matters.
6. Store database files on a trusted local filesystem.
7. Use `cortexdb backup-drill` or `cortexdb backup` plus `cortexdb restore` and
   `cortexdb validate`; do not copy live database directories manually.
8. Treat dashboard access as administrative.
9. Enable `CORTEXDB_CORS_ALLOW_ORIGIN` only for one trusted browser origin;
   keep it unset for local CLI/SDK-only deployments.
10. Set `CORTEXDB_RATE_LIMIT_PER_MINUTE` for exposed local deployments, and set
   `request_quota_per_minute` on JSON policy-store principals when local
   principal isolation is needed; use an API gateway or reverse proxy for
   distributed quotas.
11. Set `CORTEXDB_AUDIT_LOG=true` when route-level access auditing is required;
    export `tracing` output to your process supervisor or log pipeline.
12. Set `CORTEXDB_AUDIT_LOG_FILE` when route-level audit events should also be
    persisted to a local JSONL file.
13. Use `cortexdb audit <audit.jsonl> --summary --redaction-check` during
    incident review or release validation.

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
make security-hardening-check
```

Security-sensitive test areas include:

- `crates/cortex-server/src/tests/security_tests.rs`
- `crates/cortex-server/src/tests/snapshot_api_tests.rs`
- `crates/cortex-engine/tests/validation_tests.rs`
- `crates/cortex-engine/tests/corruption_matrix.rs`
- `crates/cortex-engine/tests/recovery_modes.rs`
- `crates/cortex-engine/tests/persisted_index_tests.rs`

## Beta Security Backlog

1. Extend local per-principal quotas into route-class quotas and distributed
   quota state.
2. Expand CORS beyond the current single exact-origin allowlist only after
   adding user/RBAC-aware authorization.
3. Extend file-backed token rotation into a persisted auth policy management
   workflow. The current target design is in
   [`RBAC_POLICY_STORE_DESIGN.md`](RBAC_POLICY_STORE_DESIGN.md).
4. Extend the JSONL audit sink into tamper-evident audit trails and SIEM export.
5. Add encrypted backup support and remote object-store upload adapters.
6. Add documented secret rotation beyond local token-file replacement.
7. Add deployment hardening guide for reverse proxy and systemd/container use.
8. Split operational admin routes from data routes.
9. Add security-focused release checklist.
10. Review replication and dashboard paths before claiming beta readiness.

The release-facing checklist is maintained in
[`SECURITY_RELEASE_CHECKLIST.md`](SECURITY_RELEASE_CHECKLIST.md). Production
candidate security claim boundaries are recorded in
[`SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md`](SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md),
and the latest local hardening evidence is summarized in
[`SECURITY_HARDENING_EVIDENCE.md`](SECURITY_HARDENING_EVIDENCE.md).
