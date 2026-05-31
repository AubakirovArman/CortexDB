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
   - Token policies are role-scoped (`admin`/`data`) and can be rotated from a file.
   - Tenant ID is validated and used for filesystem realm isolation.
   - `make tenant-recovery-check` verifies tenant payload boundaries before
     and after backup/restore using a real HTTP server.

3. **Authorization in query execution**
   - AQL policies and runtime `AgentAllowed` masks prevent scope privilege expansion.

4. **Error hardening**
   - Public API errors use stable machine-readable codes.
   - Policy errors avoid internal names like brain/scope identifiers.

5. **Operational safety**
   - Database lock prevents concurrent local writers.
   - Validation and repair tools run before recovery-critical operations.

## What is intentionally out of scope

The following are not production guarantees in Core Alpha:

- per-user identity, fine-grained RBAC, and distributed policy service,
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
