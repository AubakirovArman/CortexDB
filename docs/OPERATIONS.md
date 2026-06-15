# Operations Guide

## First 10 Minutes

Use this path when you are opening CortexDB for the first time and want the
shortest route from clone or release archive to a validated local database.

1. Install from release archive or build the binaries.

   Release archive path:

   ```bash
   tar -xzf cortexdb-<platform>.tar.gz
   sudo install -m 0755 cortexdb /usr/local/bin/cortexdb
   sudo install -m 0755 cortex-server /usr/local/bin/cortex-server
   cortexdb version
   cortex-server --help
   ```

   Source checkout path:

   ```bash
   cargo build --workspace
   ```

   Release binary install steps are in [`INSTALL.md`](INSTALL.md). Linux
   systemd and macOS launchd service examples are in [`SYSTEMD.md`](archive/SYSTEMD.md)
   and [`LAUNCHD.md`](archive/LAUNCHD.md).

2. Create one local database root and start the HTTP server:

   ```bash
   export CORTEXDB_AUTH_TOKEN=dev-token
   cortex-server ./data 127.0.0.1:8181
   ```

   From a source checkout, the equivalent command is:

   ```bash
   cargo run -p cortex-server -- ./data 127.0.0.1:8181
   ```

3. In another terminal, verify health and auth behavior:

   ```bash
   curl http://127.0.0.1:8181/v1/health
   curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/validate
   ```

4. Write, read, flush, and validate via CLI:

   ```bash
   cortexdb put ./data 1 "scope=default
   status=ready
   type=fact
   source=first-run

   hello cortex"
   cortexdb get ./data 1
   cortexdb flush ./data
   cortexdb stats ./data
   cortexdb validate ./data
   cortexdb doctor ./data
   ```

5. Run the local operator smoke gates:

   ```bash
   make operations-runbook-check
   make service-manager-smoke-check
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
cortexdb stats ./data
cortexdb wal-validate ./data
cortexdb manifest-validate ./data
cortexdb doctor ./data
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

## 4) Backpressure and Tenant Quotas

The server has two local backpressure layers:

- `CORTEXDB_ACTOR_QUEUE_CAPACITY` sets each tenant actor's bounded command
  queue. Default: `1024`.
- `CORTEXDB_TENANT_QUEUE_QUOTA` optionally caps in-flight actor commands per
  tenant below the actor capacity.

Tenant storage quotas are also optional and enforced before write routes:

```bash
export CORTEXDB_TENANT_MAX_CELLS=1000000
export CORTEXDB_TENANT_MAX_MEMORY_BYTES=$((8 * 1024 * 1024 * 1024))
export CORTEXDB_TENANT_QUEUE_QUOTA=128
export CORTEXDB_ACTOR_QUEUE_CAPACITY=256
```

Quota exhaustion returns `429 quota_exceeded`. The process-wide request-rate
guard remains `429 rate_limited`.

Capacity sizing:

```text
tenant_queue_quota <= actor_queue_capacity
actor_queue_capacity >= ceil(tenant_peak_rps * tolerated_queue_window_ms / 1000)
```

Use A19/C17 evidence as the starting safety margin: keep
`actor_queue_wait_p95_ms <= max(50ms, baseline_p95_ms * 1.2 + 25ms)`. The
`25ms` term is the existing C17 jitter floor. If p95 crosses that bound while
CPU/RSS are still healthy, raise `CORTEXDB_ACTOR_QUEUE_CAPACITY` and
`CORTEXDB_TENANT_QUEUE_QUOTA` together. If RSS or `estimated_total_memory_bytes`
is the limiting factor, lower tenant queue/storage quotas instead.

Local gate:

```bash
cargo test -p cortex-server tenant_quota_50_tenant_load_smoke --all-features
make load-suite-check
```

## 5) Operational Runbooks

The beta RC operator path is split by activity:

| Activity | Primary doc | Local gate or command |
| --- | --- | --- |
| full runbook | [`OPERATIONS_RUNBOOK_V1.md`](archive/OPERATIONS_RUNBOOK_V1.md) | `make operations-runbook-check` |
| install | [`INSTALL.md`](INSTALL.md) | `make binary-release-check` |
| service setup | [`SYSTEMD.md`](archive/SYSTEMD.md), [`LAUNCHD.md`](archive/LAUNCHD.md) | `make service-manager-smoke-check` and `/v1/validate` health probe |
| validate | [`CLI.md`](CLI.md), [`API.md`](API.md) | `cortexdb validate ./data` |
| backup | [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) | `make backup-drill-check` |
| restore | [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) | `cortexdb restore <backup> <target>` or `--to-seq <N>` |
| backup pack | [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md), [`RPO_RTO.md`](archive/RPO_RTO.md) | `make backup-restore-production-pack-check` |
| repair | [`CLI.md`](CLI.md), [`FAILURE_SCENARIOS.md`](archive/FAILURE_SCENARIOS.md) | `cortexdb repair ./data --dry-run` |
| metrics | [`METRICS.md`](METRICS.md), [`OBSERVABILITY_ALERTS.md`](archive/OBSERVABILITY_ALERTS.md) | `make observability-check` |
| upgrade | [`UPGRADE_ROLLBACK.md`](archive/UPGRADE_ROLLBACK.md), [`UPGRADE_MIGRATION.md`](archive/UPGRADE_MIGRATION.md) | `make deployment-upgrade-check` |
| rollback | [`UPGRADE_ROLLBACK.md`](archive/UPGRADE_ROLLBACK.md) | restore previous backup and validate |

## 6) Backup and recovery

```bash
cortexdb backup ./data ./backups/data-$(date -u +%Y%m%dT%H%M%SZ)
cortexdb backup-prune ./backups cortexdb- 5
cortexdb restore ./backups/data-20260602T000000Z ./data-restored
cortexdb restore ./backups/data-20260602T000000Z ./data-restored-seq42 --to-seq 42
cortexdb validate ./data-restored
export CORTEXDB_BACKUP_PASSPHRASE="choose-a-long-local-passphrase"
cortexdb backup-encrypted ./data ./backups/data.cdbenc --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
cortexdb restore-encrypted ./backups/data.cdbenc ./data-encrypted-restored --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
```

Point-in-time restore requires WAL archiving before backups are created:

```bash
export CORTEXDB_WAL_ARCHIVE=true
export CORTEXDB_WAL_ARCHIVE_MAX_FILES=1024
```

Closed WAL files are copied under `wal_archive/` after checkpoint or compact.
`restore --to-seq N` stages retained archived WAL files, caps newer manifest
segments, replays records through `N`, and rejects incomplete archives or a
target sequence that would split an atomic write batch.

Offsite staging:

```bash
cortexdb backup-offsite-stage ./backups/data-20260602 ./offsite cortexdb-$(date -u +%Y%m%dT%H%M%SZ)
```

Release evidence:

```bash
make backup-restore-production-pack-check
```

## 7) Troubleshooting

### Stale lock or `database_busy`

Symptom: `503 database_busy`, `DatabaseAlreadyOpen`, or startup reports a lock
conflict.

Action:

```bash
cortexdb unlock ./data --force
cortexdb validate ./data
```

Only use `unlock --force` after confirming no other process owns the same
database root.

### Corrupt WAL or partial WAL tail

Symptom: validation or startup reports WAL checksum/tail issues.

Action:

```bash
cortexdb validate ./data
cortexdb wal-dump ./data
cortexdb wal-truncate ./data
cortexdb validate ./data
```

### Corrupt segment or index bundle

Symptom: validation reports `.acs`, `.acb`, `.aci`, `.acv`, `.ach`, or manifest
corruption.

Action:

```bash
cortexdb validate ./data
cortexdb repair ./data --dry-run
cortexdb repair ./data --best-effort
cortexdb validate ./data
```

If the live segment is corrupt and repair cannot produce a safe plan, restore from the
latest validated backup as documented in
[`BACKUP_RESTORE.md`](BACKUP_RESTORE.md).
Operator action: restore from the latest validated backup.

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

Action: check [`TENANT_NAMING_RULES.md`](archive/TENANT_NAMING_RULES.md). Tenant names
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
Use [`AUDIT_LOG_FORMAT.md`](AUDIT_LOG_FORMAT.md) for the JSONL schema,
`scope_decision` semantics, rotation, fsync, and SIEM export boundary.

## 8) Performance/reliability smoke

- Runbook coverage: `make operations-runbook-check`
- Service manager examples: `make service-manager-smoke-check`
- CLI/HTTP smoke: `scripts/smoke_test.sh`
- Load and metrics smoke: `make load-smoke-check`
- ANN/recall drift: `make ann-history-regression-check`, `make ann-drift-check`
- Recovery/fault: `make crash-fault-check`, `make chaos-restart-check`
- Migration compatibility: `make migration-compatibility-check`
- Storage soak history: `make storage-soak-history-check`
- Production boundary: `make production-candidate-check`,
  `make production-v1-check`

## 9) Known operational limits

- Single-node model first.
- Production multi-node is experimental.
- HNSW is guarded; exact vector path remains the correctness fallback.
- 24-hour soak is only claimed when
  `target/storage-soak-history/report.json` reports
  `twenty_four_hour_evidence.met=true`.
- Local encrypted backups are passphrase-based; KMS-backed backup custody is
  future work.
- For distributed security/compliance needs, wait for dedicated production
  hardening milestone.

## 10) Operator evidence bundle

Before publishing or handing a build to another operator, collect:

```bash
make operations-runbook-check
make service-manager-smoke-check
make deployment-upgrade-check
make observability-check
make migration-compatibility-check
make backup-restore-production-pack-check
make storage-soak-history-check
```

Primary reports:

```text
target/operations-runbook/report.json
target/service-manager-smoke/report.json
target/deployment-upgrade/report.json
target/observability/report.json
target/migration-upgrade-matrix-v2/report.json
target/backup-restore-production-pack/report.json
target/storage-soak-history/report.json
```
