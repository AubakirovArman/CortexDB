# Operations Guide

## First 10 Minutes

Use this path when you are opening CortexDB for the first time and want the
shortest route from clone or release archive to a validated local database.

1. Install or build the binaries:

   ```bash
   cargo build --workspace
   ```

   Release binary install steps are in [`INSTALL.md`](INSTALL.md). A systemd
   service example is in [`SYSTEMD.md`](SYSTEMD.md).

2. Create one local database root and start the HTTP server:

   ```bash
   export CORTEXDB_AUTH_TOKEN=dev-token
   cargo run -p cortex-server -- ./data 127.0.0.1:8181
   ```

3. In another terminal, verify health and auth behavior:

   ```bash
   curl http://127.0.0.1:8181/v1/health
   curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/validate
   ```

4. Write, read, flush, and validate via CLI:

   ```bash
   cargo run -p cortex-cli -- put ./data 1 "scope=default
   status=ready
   type=fact
   source=first-run

   hello cortex"
   cargo run -p cortex-cli -- get ./data 1
   cargo run -p cortex-cli -- flush ./data
   cargo run -p cortex-cli -- stats ./data
   cargo run -p cortex-cli -- validate ./data
   ```

5. Run the local operator smoke gates:

   ```bash
   make deployment-upgrade-check
   make observability-check
   make public-claims-check
   ```

If these pass, the local single-node operator surface is wired. This does not
turn the Core Alpha into a general production or distributed deployment.

## 1) Local operation model

CortexDB is currently optimized for single-node Core Alpha operation.
Use one runtime process per database root.

- `cortex-server` provides HTTP access (`/v1/*`).
- `cortex-cli` provides local one-shot operations.
- backups, metrics, recovery scripts, and release gates are in `Makefile`.
- metrics fields, endpoints, and alert heuristics are documented in
  [`METRICS.md`](METRICS.md).

## 2) Start server

```bash
CORTEXDB_AUTH_TOKEN=dev-token \
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Health check:

```bash
curl http://127.0.0.1:8181/v1/health
```

## 3) Core sanity checks

```bash
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- stats ./data
cargo run -p cortex-cli -- wal-validate ./data
cargo run -p cortex-cli -- manifest-validate ./data
cargo run -p cortex-cli -- ann-validate ./data
```

For installed release binaries, the equivalent validation entrypoint is:

```bash
cortexdb validate ./data
```

Optional typed checks and smoke paths:

```bash
make openapi-contract-check
make sdk-contract-check
make sdk-smoke-test
make dashboard-smoke
make production-candidate-check
make production-v1-check
```

## 4) Operational Runbooks

The beta RC operator path is split by activity:

| Activity | Primary doc | Local gate or command |
| --- | --- | --- |
| install | [`INSTALL.md`](INSTALL.md) | `make binary-release-check` |
| service setup | [`SYSTEMD.md`](SYSTEMD.md) | `/v1/validate` health probe |
| validate | [`CLI.md`](CLI.md), [`API.md`](API.md) | `cortexdb validate ./data` |
| backup | [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) | `make backup-drill-check` |
| restore | [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) | `cortexdb restore <backup> <target>` |
| repair | [`CLI.md`](CLI.md), [`FAILURE_SCENARIOS.md`](FAILURE_SCENARIOS.md) | `cortexdb repair ./data --dry-run` |
| metrics | [`METRICS.md`](METRICS.md), [`OBSERVABILITY_ALERTS.md`](OBSERVABILITY_ALERTS.md) | `make observability-check` |
| upgrade | [`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md), [`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md) | `make deployment-upgrade-check` |
| rollback | [`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md) | restore previous backup and validate |

## 5) Backup and recovery

```bash
cargo run -p cortex-cli -- backup ./data ./backups/data-$(date -u +%Y%m%dT%H%M%SZ)
cargo run -p cortex-cli -- backup-prune ./backups cortexdb- 5
cargo run -p cortex-cli -- restore ./backups/data-... ./data-restored
cargo run -p cortex-cli -- validate ./data-restored
```

Offsite staging:

```bash
cargo run -p cortex-cli -- backup-offsite-stage ./backups/data.tar.gz ./offsite cortexdb-$(date -u +%Y%m%dT%H%M%SZ)
```

## 6) Troubleshooting

### Stale lock or `database_busy`

Symptom: `503 database_busy`, `DatabaseAlreadyOpen`, or startup reports a lock
conflict.

Action:

```bash
cargo run -p cortex-cli -- unlock ./data --force
cargo run -p cortex-cli -- validate ./data
```

Only use `unlock --force` after confirming no other process owns the same
database root.

### Corrupt WAL or partial WAL tail

Symptom: validation or startup reports WAL checksum/tail issues.

Action:

```bash
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- wal-dump ./data
cargo run -p cortex-cli -- wal-truncate ./data
cargo run -p cortex-cli -- validate ./data
```

### Corrupt segment or index bundle

Symptom: validation reports `.acs`, `.acb`, `.aci`, `.acv`, `.ach`, or manifest
corruption.

Action:

```bash
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- repair ./data --dry-run
cargo run -p cortex-cli -- repair ./data --best-effort
cargo run -p cortex-cli -- validate ./data
```

If the live segment is corrupt and repair cannot produce a safe plan, restore
from the latest validated backup as documented in
[`BACKUP_RESTORE.md`](BACKUP_RESTORE.md).

### Failed authentication

Symptom: `401 unauthorized` for missing or bad token, or `403 forbidden` when a
`data` token reaches an admin route.

Action:

```bash
curl http://127.0.0.1:8181/v1/health
curl -H "Authorization: Bearer $CORTEXDB_AUTH_TOKEN" http://127.0.0.1:8181/v1/validate
```

Then verify [`AUTH.md`](AUTH.md), `CORTEXDB_AUTH_TOKEN`,
`CORTEXDB_AUTH_TOKENS`, or `CORTEXDB_AUTH_TOKENS_FILE`.

### Tenant errors

Symptom: `400 invalid_tenant`.

Action: check [`TENANT_NAMING_RULES.md`](TENANT_NAMING_RULES.md). Tenant names
must be path-safe and must not contain traversal, slash, empty, or reserved
segments.

### Audit review

Audit review:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check
cargo run -p cortex-cli -- audit ./audit/http.jsonl --summary --redaction-check
cargo run -p cortex-cli -- audit ./audit/http.jsonl --action write --tenant-filter tenant-alpha
```

Use this during incident review to count route activity by action/status/tenant
and confirm the audit sink did not persist query strings or body-like fields.

## 7) Performance/reliability smoke

- CLI/HTTP smoke: `scripts/smoke_test.sh`
- Load and metrics smoke: `make load-smoke-check`
- ANN/recall drift: `make ann-history-regression-check`, `make ann-drift-check`
- Recovery/fault: `make crash-fault-check`, `make chaos-restart-check`
- Production boundary: `make production-candidate-check`,
  `make production-v1-check`

## 8) Known operational limits

- Single-node model first.
- Production multi-node is experimental.
- HNSW is guarded; exact vector path remains the correctness fallback.
- For distributed security/compliance needs, wait for dedicated production hardening milestone.
