# CortexDB Security Model

This document summarizes the **current security model** for Core Alpha and what is
*not* guaranteed yet.

## Scope

CortexDB ships today as a single-node durable store + HTTP API. The security model is
anchored around:

- durable local storage safety (WAL + segments + manifest),
- request boundary controls (auth + tenant isolation),
- AgentView authorization in AQL + execution paths,
- contract-stable, non-sensitive API errors.

## Invariants that hold today

1. **Integrity on write path**
   - WAL and persisted indexes are CRC-protected.
   - Corruption is rejected or recovered according to strict/best-effort mode.

2. **Auth and tenant boundary**
   - Optional bearer-token auth is enforced when configured.
   - Token roles are currently `admin` and `data`.
   - Token policies are role-scoped (`admin`/`data`) and can be rotated from a file.
   - Tenant ID is validated and used for filesystem realm isolation.
   - Tenant allowlists are enforced before realm open/create, so a forbidden
     tenant data route cannot create or mutate that tenant directory.
   - Both the Axum `DatabaseActor` path and the legacy synchronous test harness
     route non-default tenants to `root/realms/<tenant>/`.
   - `make tenant-recovery-check` verifies tenant payload boundaries before
     and after backup/restore using a real HTTP server.

3. **Authorization in query execution**
   - AQL policies and runtime `AgentAllowed` masks prevent scope privilege expansion.
   - ContextPack JSON includes a per-cell `access_decision` trail linking each
     selected `cell_id` to the AgentView readable-scope decision that allowed it
     into the pack. HTTP responses attach the authenticated `principal_id` and
     `auth_role` when bearer-token policy store auth is configured.

4. **Permission-safe read invariant**
   - The source of truth for read authorization is
     `cortex_engine::plan::PolicyRewrite`.
   - Logical read plans are rewritten before execution so every `Scan` node
     carries the `agent_allowed` permission predicate; structural tests cover
     AQL retrieve/explain, search/explain, ContextPack/trace, cell get, verify,
     graph, memory, feedback, and export surfaces.
   - Direct descriptor-backed server surfaces such as `/v1/cell`, feedback,
     and memory routes delegate read decisions to `PolicyRewrite` before payload
     materialization. Stored-cell authorization uses durable `CellDescriptor`
     scope, not spoofable payload headers.
   - `make check` runs `policy-rewrite-gate-check`, which rejects direct
     production `AgentView::can_read_scope` calls outside `PolicyRewrite` and
     verifies the read-surface registry and structural tests remain present.
   - `cargo test -p cortex-server agent_view_property --all-features` exercises
     the E09 property suite across HTTP read surfaces before and after flush:
     no unreadable-scope payload marker may appear in success or error bodies.

5. **Error hardening**
   - Public API errors use stable machine-readable codes.
   - Policy errors avoid internal names like brain/scope identifiers.

6. **Operational safety**
   - Database lock prevents concurrent local writers.
   - Validation and repair tools run before recovery-critical operations.

## Not Yet Production Security

This section is intentionally explicit for beta/release checks: these items are
not production security guarantees yet.

The following are not production guarantees in Core Alpha:

- production IAM federation, distributed policy service, and external identity
  lifecycle,
- TLS/MTLS lifecycle in-process (use reverse proxy for HTTPS/TLS offload),
- encrypted at-rest backups and secret management integrations,
- tamper-evident audit export,
- distributed consensus correctness guarantees (single-node only),
- multi-tenant zero-trust isolation across untrusted processes.

## Operational posture

For non-local deployments, treat Core Alpha as **alpha quality** and:

- run behind trusted network controls,
- keep `admin` tokens strong and rotate them via token files,
- enable audit logging if route-level traceability is needed,
- gate exposed endpoints with reverse-proxy hardening and rate limiting,
- run `cortexdb validate`, `cortexdb backup`, and `cortexdb restore` in change control.
- run `make tenant-recovery-check` before releases that modify tenant routing,
  backup/restore, or server actor lifecycle.
- run `make security-check` before beta/release packaging; it writes
  `target/security/report.json` with focused auth, tenant, CORS, rate-limit,
  audit-redaction, AgentView, body-limit, and OpenAPI contract evidence.

## Tenant Recovery Evidence

`make tenant-recovery-check` starts a real `cortex-server`, writes the same
`cell_id` into the `default`, `tenant-alpha`, and `tenant-beta` realms, flushes
and validates each tenant, verifies invalid tenant IDs fail closed, backs up the
server root, restores it to a new root, restarts the server, and verifies the
tenant payloads remain isolated after restore.

The report is written to:

```text
target/tenant-recovery/report.json
```

This is still a Core Alpha local tenant boundary, not a zero-trust
multi-process isolation guarantee.

## Relation to threat model

See `docs/SECURITY_THREAT_MODEL.md` for detailed threat analysis and current
controls list.

The beta-facing baseline is maintained in
`docs/SECURITY_BETA_BASELINE.md`. That document is the release boundary between
implemented Core Alpha controls and future RBAC policy-store, per-token quota,
tamper-evident audit-chain, encrypted-backup, and dashboard-auth hardening work.
