# CortexDB Operations Runbook v1

Status: local single-node operator runbook for the current production epic
plan. This is not a managed-cloud, multi-node, or enterprise compliance
runbook.

Use this document when you need a direct operational sequence. Use
[`OPERATIONS.md`](OPERATIONS.md) for the broader guide and links.

## 1. Install

Install from release archive:

```bash
tar -xzf cortexdb-<platform>.tar.gz
install -m 0755 cortexdb ~/.local/bin/cortexdb
install -m 0755 cortex-server ~/.local/bin/cortex-server
cortexdb version
cortex-server --help
```

Source checkout path:

```bash
cargo build --workspace
```

Supporting docs: [`INSTALL.md`](INSTALL.md), [`SYSTEMD.md`](SYSTEMD.md), and
[`LAUNCHD.md`](LAUNCHD.md).

## 2. Startup

Start one server process per database root:

```bash
export CORTEXDB_AUTH_TOKEN=dev-token
cortex-server ./data 127.0.0.1:8181
```

Source checkout equivalent:

```bash
CORTEXDB_AUTH_TOKEN=dev-token \
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Verify health and auth:

```bash
curl http://127.0.0.1:8181/v1/health
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/validate
```

Write and read one smoke cell:

```bash
cortexdb put ./data 1 "scope=default
status=ready
type=fact
source=runbook

hello cortex"
cortexdb get ./data 1
```

## 3. Shutdown

Stop writers before file-level backup, upgrade, or manual repair.

For a foreground process, send `Ctrl-C` and wait for exit. For service
managers, use the manager-specific stop command:

```bash
systemctl stop cortexdb
launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.cortexdb.server.plist
```

After shutdown, confirm no process is serving that database root, then run:

```bash
cortexdb validate ./data
cortexdb doctor ./data
```

If startup later reports a stale lock, use forced unlock only after confirming
the old process is gone:

```bash
cortexdb unlock ./data --force
```

## 4. Validate

Run validation after startup, before backup, after restore, after repair, and
after upgrade:

```bash
cortexdb validate ./data
cortexdb stats ./data
cortexdb wal-validate ./data
cortexdb manifest-validate ./data
cortexdb ann-validate ./data
cortexdb doctor ./data
```

HTTP health and validation:

```bash
curl http://127.0.0.1:8181/v1/health
curl -H "Authorization: Bearer dev-token" http://127.0.0.1:8181/v1/validate
```

Supporting docs: [`CLI.md`](CLI.md), [`API.md`](API.md), [`METRICS.md`](METRICS.md),
and [`OBSERVABILITY_ALERTS.md`](OBSERVABILITY_ALERTS.md).

## 5. Backup

Create and drill local backups:

```bash
cortexdb backup ./data ./backups/cortexdb-$(date -u +%Y%m%dT%H%M%SZ)
cortexdb backup-drill ./data ./backups/drill ./drills/drill-restored
cortexdb backup-prune ./backups cortexdb- 7 --dry-run
cortexdb backup-prune ./backups cortexdb- 7
```

Encrypted local backup:

```bash
export CORTEXDB_BACKUP_PASSPHRASE="choose-a-long-local-passphrase"
cortexdb backup-encrypted ./data ./backups/data.cdbenc --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
cortexdb restore-encrypted ./backups/data.cdbenc ./data-encrypted-restored --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
```

Offsite staging after a validated local backup:

```bash
cortexdb backup-offsite-stage ./backups/cortexdb-20260602 ./offsite cortexdb-20260602
```

Production evidence gate:

```bash
make backup-restore-production-pack-check
```

Supporting docs: [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) and
[`RPO_RTO.md`](RPO_RTO.md).

## 6. Restore

Always restore into a new directory and validate before serving:

```bash
cortexdb restore ./backups/cortexdb-20260602 ./data-restored --dry-run
cortexdb restore ./backups/cortexdb-20260602 ./data-restored
cortexdb validate ./data-restored
```

If restore was part of rollback, start the server against the restored target,
not the failed upgraded directory.

## 7. Repair

Start with dry-run repair and keep the original directory or backup until the
repair result validates:

```bash
cortexdb validate ./data
cortexdb repair ./data --dry-run
cortexdb repair ./data --best-effort
cortexdb validate ./data
```

WAL tools are explicit:

```bash
cortexdb wal-dump ./data
cortexdb wal-truncate ./data
cortexdb wal-validate ./data
```

Use `restore from the latest validated backup` when repair cannot produce a
safe plan.

Supporting doc: [`FAILURE_SCENARIOS.md`](FAILURE_SCENARIOS.md).

## 8. Upgrade

Offline upgrade sequence:

1. Stop writers and the HTTP server.
2. Run `cortexdb validate ./data` and `cortexdb stats ./data`.
3. Create a validated backup and restore drill with
   `cortexdb upgrade prepare ./data ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade`.
4. Install the new binary archive.
5. Run `cortexdb upgrade validate ./data`.
6. Restart the server.

Compatibility evidence:

```bash
make deployment-upgrade-check
make migration-compatibility-check
```

Rollback requires restoring the pre-upgrade backup into a new directory:

```bash
cortexdb upgrade rollback ./backups/cortexdb-pre-upgrade ./data.rollback
```

Supporting docs: [`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md) and
[`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md).

## 9. Incidents

Use [`INCIDENT_PLAYBOOKS.md`](INCIDENT_PLAYBOOKS.md) for the detailed
corrupted storage, actor busy, backup failed, auth failure spike, and tenant
issue playbooks. The short flows below are the first triage path.

For `database_busy`:

1. Check `/v1/metrics` actor queue depth and rejected request counters.
2. Reduce caller concurrency or add retry backoff.
3. Validate the database before changing queue limits.

For `invalid_tenant`:

1. Check tenant name path safety.
2. Review [`TENANT_NAMING_RULES.md`](TENANT_NAMING_RULES.md).

For suspected corruption:

1. Stop promotion and save `cortexdb validate ./data` output.
2. Run `cortexdb repair ./data --dry-run`.
3. Restore from the latest validated backup if corruption is not safely
   repairable.

For audit review:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
```

## 10. Evidence Gates

Run these before release promotion or operator handoff:

```bash
make operations-runbook-check
make incident-playbooks-check
make service-manager-smoke-check
make deployment-upgrade-check
make observability-check
make migration-compatibility-check
make backup-restore-production-pack-check
make storage-soak-history-check
```

## 11. Boundaries

- Single-node model first.
- Production multi-node is experimental.
- 24-hour soak is claimed only when the storage-soak history report says the
  24-hour evidence was met.
- KMS-backed backup custody is future work; local encrypted backups are
  passphrase-based.
- Managed alert routing, managed cloud operations, and enterprise compliance
  evidence are separate future milestones.
