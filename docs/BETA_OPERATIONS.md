# CortexDB Beta Operations Pack

This runbook is for the `v0.2.0-beta.1` local single-node developer/API beta.
It covers Linux and macOS only. It does not describe a managed cloud service,
distributed production deployment, or enterprise compliance posture.

## Install

Build from a clean checkout:

```bash
cargo build --workspace --all-targets
```

For release packaging, use the binary-release docs and gates:

```bash
make binary-release-check
```

See also:

- [`INSTALL.md`](INSTALL.md)
- [`BINARY_RELEASES.md`](BINARY_RELEASES.md)

## Run Server

Run a local server rooted at a database directory:

```bash
cargo run -p cortex-server -- ./data 127.0.0.1:8080
```

The beta server is an async HTTP surface over a local blocking single-node
database core. Use a trusted reverse proxy for TLS and external network access.

## Auth

For local beta use, configure bearer tokens before exposing any endpoint:

```bash
export CORTEXDB_AUTH_TOKENS="admin:admin-token,data:data-token"
```

For token rotation, use a file-backed token policy:

```bash
export CORTEXDB_AUTH_TOKENS_FILE=./auth.tokens
```

Review configured policy without printing raw token values:

```bash
cortexdb auth-review --tokens-file ./auth.tokens
```

See [`AUTH.md`](AUTH.md) and [`SECURITY_THREAT_MODEL.md`](SECURITY_THREAT_MODEL.md).

## Tenants

Tenant IDs are local realm names. They are validated for path safety and are not
a substitute for zero-trust multi-tenant isolation.

Use:

```text
?tenant=default
?tenant=tenant-alpha
```

Run tenant recovery evidence before release:

```bash
make tenant-recovery-check
```

## Backup

Create and validate local backups:

```bash
cortexdb backup ./data ./backups/db-001
cortexdb backup-drill ./data ./backups/drill-001
make backup-drill-check
```

For offsite staging, publish only a validated local backup directory:

```bash
cortexdb backup-offsite-stage ./data ./offsite-stage/db-001
```

## Restore

Restore into a new directory, then validate before serving:

```bash
cortexdb restore ./backups/db-001 ./restore/db-001
cortexdb validate ./restore/db-001
```

See [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md).

## Validate

Run validation after restore, before release packaging, and after any manual
repair:

```bash
cortexdb validate ./data
```

For broader evidence:

```bash
make beta-release-check
```

## Repair

Use repair only for local recovery workflows after reviewing validation output:

```bash
cortexdb repair ./data
```

Best-effort recovery can stop at safe WAL offsets. Strict recovery should fail
on corruption. Keep the original directory or backup until validation passes.

## Upgrade

For beta, upgrade by:

1. stopping the server;
2. taking a validated backup;
3. replacing the binary;
4. running `cortexdb validate`;
5. starting the server;
6. running smoke checks.

Run:

```bash
make deployment-upgrade-check
```

See [`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md) and
[`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md).

## Rollback

Rollback requires a validated backup from before the upgrade:

```bash
cortexdb restore ./backups/pre-upgrade ./rollback/data
cortexdb validate ./rollback/data
```

Do not mix files from different database roots manually.

## Metrics

Use the metrics endpoint for local operational checks:

```bash
curl -H "Authorization: Bearer admin-token" \
  http://127.0.0.1:8080/v1/metrics
```

Prometheus text output is available with:

```text
/v1/metrics?format=prometheus
```

See [`METRICS.md`](METRICS.md).

## Logs And Audit

Enable route-level audit logging when access review matters:

```bash
export CORTEXDB_AUDIT_LOG=true
export CORTEXDB_AUDIT_LOG_FILE=./audit/http.jsonl
```

Review the audit file:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
```

## Before Using Beta

- Set admin and data auth tokens.
- Keep the server behind trusted network controls.
- Run `cortexdb validate` on the database root.
- Run a validated backup or `cortexdb backup-drill`.
- Enable audit logging if route-level review is needed.
- Run `make security-check`.
- Run `make beta-release-check` before publishing beta artifacts.
- Do not expose CortexDB directly to the public internet.

## Known Limits

- Single-node local durability only.
- No production distributed consensus guarantee.
- No managed cloud control plane.
- No enterprise RBAC/compliance certification.
- No built-in TLS lifecycle; terminate TLS outside CortexDB.
- No encrypted backup support in the current beta boundary.
- HNSW/ANN remains guarded by exact fallback for beta claims.

