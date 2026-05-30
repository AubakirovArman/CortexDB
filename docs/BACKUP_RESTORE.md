# Backup and Restore

Core Alpha backups are validated filesystem snapshots of a closed or exclusively
locked single-node database directory.

## Commands

```bash
cortexdb backup ./db ./db.backup
cortexdb restore ./db.backup ./db.restored
cortexdb validate ./db.restored
cortexdb backup-drill ./db ./db.backup ./db.drill-restored
cortexdb backup-prune ./backups cortexdb- 7
make backup-drill-check
```

## Safety Rules

- Backup takes the source database lock.
- The WAL writer is shut down before copying and restarted afterward.
- Source storage is validated before copying.
- `db.lock` and known temporary files are excluded.
- Restore only writes to a target path that does not already exist.
- Restore validates the copied database before reporting success.
- Backup drills run backup, restore, and restored validation as one operation.
- Retention pruning only removes directories whose names start with an
  explicit non-empty prefix and keeps at least one latest backup.

## What Is Copied

The backup copies the database root recursively, including:

- `db.aclog`;
- `manifest.acm`;
- `segments/*.acs`, `*.acb`, `*.aci`, `*.acv`, `*.ach`;
- `agent_views/*.view`;
- any future regular files under the same root.

Symlinks and other non-regular files are rejected.

## Current Limitations

- This is a local filesystem backup, not an encrypted backup format.
- There is no incremental backup or remote object-store target yet.
- There is no built-in scheduler. Run drills and pruning from cron/systemd or
  an external orchestrator.
- The source database must be opened exclusively by this process.
- The restored target must be new; in-place overwrite is intentionally refused.
- Backup drill targets must also be new; the command leaves the restored copy in
  place for inspection.

## Operational Drill

Run a restore drill on the same release and host profile used for backups:

```bash
cortexdb backup-drill ./db ./db.backup.$(date +%Y%m%d) ./db.restore-drill
cortexdb validate ./db.restore-drill
```

The first command proves the backup can be copied, opened, replayed, and
validated. The second command is optional but useful in runbooks because it
prints the validation report directly.

## Retention and Offsite Policy

Use sortable backup names and prune only after a drill succeeds:

```bash
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
cortexdb backup-drill ./db "./backups/cortexdb-$stamp" "./drills/cortexdb-$stamp"
cortexdb backup-prune ./backups cortexdb- 7
```

Core Alpha does not upload to remote storage itself. The supported offsite
policy is: create a local validated backup, run a restore drill, then let an
external tool copy the backup directory to object storage or another host. Keep
the external copy immutable where possible and run a scheduled drill against a
freshly restored offsite copy before relying on it.

The retention command reports:

- `backups_seen` — matching backup directories under the root;
- `backups_kept` — latest matching directories preserved;
- `backups_removed` — older matching directories deleted;
- `bytes_removed` — approximate local bytes reclaimed.

## Release Evidence

`make backup-drill-check` is the repeatable local evidence gate. It creates a
temporary database under `target/backup-drill`, runs three restore drills,
prunes old backup directories, validates the latest restored copy, reads back
the latest payload, and writes:

```text
target/backup-drill/report.json
```

Override paths in automation when needed:

```bash
make backup-drill-check \
  BACKUP_DRILL_ROOT=/var/tmp/cortexdb-backup-drill \
  BACKUP_DRILL_REPORT=/var/tmp/cortexdb-backup-drill/report.json \
  BACKUP_DRILL_KEEP_LATEST=7
```
