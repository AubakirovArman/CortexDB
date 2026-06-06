# Backup and Restore

Core Alpha backups are validated filesystem snapshots of a closed or exclusively
locked single-node database directory.

## Commands

```bash
cortexdb backup ./db ./db.backup
cortexdb restore ./db.backup ./db.restored --dry-run
cortexdb restore ./db.backup ./db.restored
cortexdb validate ./db.restored
cortexdb backup-drill ./db ./db.backup ./db.drill-restored
cortexdb backup-prune ./backups cortexdb- 7
cortexdb backup-offsite-stage ./db.backup ./offsite cortexdb-20260530T000000Z
cortexdb backup-encrypted ./db ./db.cdbenc --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
cortexdb restore-encrypted ./db.cdbenc ./db.encrypted-restored --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
make backup-drill-check
make backup-offsite-check
make backup-rpo-rto-profile-check
make backup-restore-production-pack-check
```

## Safety Rules

- Backup takes the source database lock.
- The WAL writer is shut down before copying and restarted afterward.
- Source storage is validated before copying.
- `db.lock` and known temporary files are excluded.
- Restore dry-run inspects backup files, storage checksums, format
  compatibility, manifest segments, indexes, and WAL readability without
  creating the target path.
- Restore only writes to a target path that does not already exist.
- Restore validates the copied database before reporting success.
- Backup drills run backup, restore, and restored validation as one operation.
- Offsite staging first restores the local backup as a preflight drill, then
  publishes an atomically renamed copy under the offsite root.
- Encrypted backup restore rejects wrong passphrases or corrupted ciphertext
  before trusting the restored database.
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

- Passphrase encrypted backups are a local MVP, not KMS-backed envelope
  encryption or a compliance custody workflow.
- Remote object-store upload is still delegated to external tools, but
  `backup-offsite-stage` now gives those tools a validated immutable directory
  to copy.
- There is no incremental backup yet.
- There is no built-in scheduler. Run drills and pruning from cron/systemd or
  an external orchestrator.
- The source database must be opened exclusively by this process.
- The restored target must be new; in-place overwrite is intentionally refused.
- Backup drill targets must also be new; the command leaves the restored copy in
  place for inspection.

Before a real restore, run a dry-run preflight:

```bash
cortexdb restore ./db.backup ./db.restore-target --dry-run
```

This command does not create `./db.restore-target`. It verifies the backup can
be read by the current binary and reports the files, bytes, manifest segments,
cells, and WAL records that would be restored.

## Operational Drill

Run a restore drill on the same release and host profile used for backups:

```bash
cortexdb backup-drill ./db ./db.backup.$(date +%Y%m%d) ./db.restore-drill
cortexdb validate ./db.restore-drill
```

The first command proves the backup can be copied, opened, replayed, and
validated. The second command is optional but useful in runbooks because it
prints the validation report directly.

For compatibility releases, the current-version backup restored by next-version code workflow is
offline: create the backup with the released binary, then open, restore, and
validate it with the candidate checkout. Local development gates model this by
using the current checkout as the next-version code under test.

## Retention and Offsite Policy

Use sortable backup names and prune only after a drill succeeds:

```bash
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
cortexdb backup-drill ./db "./backups/cortexdb-$stamp" "./drills/cortexdb-$stamp"
cortexdb backup-prune ./backups cortexdb- 7 --dry-run
cortexdb backup-prune ./backups cortexdb- 7
```

Core Alpha does not upload to remote storage itself. The supported offsite
adapter is `local_filesystem`: create a local validated backup, run a restore
drill, stage the validated backup into an external/offsite root, validate that
staged copy, atomically publish it with a final rename, then let an external
tool copy that staged directory to object storage or another host. Keep the
external copy immutable where possible and run a scheduled drill against a
freshly restored offsite copy before relying on it.

```bash
cortexdb backup-offsite-stage \
  "./backups/cortexdb-$stamp" \
  ./offsite-staging \
  "cortexdb-$stamp"
```

The command reports `adapter=local_filesystem` and `published=true`. It rejects
unsafe backup ids, refuses to overwrite an existing staged backup, removes its
preflight restore directory after validation, and publishes the final directory
with atomic rename from `<backup_id>.staging`.

The retention command reports:

- `dry_run` — whether the command only previewed deletions;
- `backups_seen` — matching backup directories under the root;
- `backups_kept` — latest matching directories preserved;
- `backups_removed` — older matching directories deleted;
- `bytes_removed` — approximate local bytes reclaimed.

## Release Evidence

`make backup-drill-check` is the repeatable local evidence gate. It creates a
temporary database under `target/backup-drill`, runs three restore drills,
previews a restore without creating the dry-run target, previews and applies
backup pruning, validates the latest restored copy, reads back the latest
payload, and writes:

```text
target/backup-drill/report.json
```

The report is a release artifact. Keep it with release evidence when promoting
a build, because it contains the local git SHA, the backup prefix, the retained
backup policy, restore dry-run output, dry-run prune output, applied prune
output, readback output, and a `restore_drill_trend` array. The trend is
local-current by default; release automation should archive each report so the
restore drill trend across
releases can be compared before beta promotion.

`make backup-offsite-check` is the repeatable offsite-staging evidence gate. It
creates a local backup, runs a restore drill, stages the backup under an offsite
root through the local filesystem adapter, validates the staged copy, reads back
the latest payload, proves staged-upload simulation, and writes:

```text
target/backup-offsite/report.json
```

`make backup-rpo-rto-profile-check` is the repeatable local RPO/RTO profile
gate. It creates small, medium, and large local datasets, measures backup,
restore dry-run, and restore duration for each profile, verifies readback, and
proves that writes after the backup are not claimed by the restored copy:

```text
target/backup-rpo-rto/report.json
```

`make backup-restore-production-pack-check` is the supported workflow gate. It
runs local drill, offsite staging, encrypted backup restore tests, and writes a
single release artifact:

```text
target/backup-restore-production-pack/report.json
```

The production-pack report records the supported workflow, RPO boundary, local
RTO profile evidence, encrypted-backup gate coverage, and paths to the
underlying drill, offsite, and profile reports.

Override paths in automation when needed:

```bash
make backup-drill-check \
  BACKUP_DRILL_ROOT=/var/tmp/cortexdb-backup-drill \
  BACKUP_DRILL_REPORT=/var/tmp/cortexdb-backup-drill/report.json \
  BACKUP_DRILL_KEEP_LATEST=7
```

## Backup Archive Corruption Evidence

Backup restore must reject corrupted archives instead of reporting a successful
restore. The current regression tests corrupt checkpointed backup files before
restore:

```bash
cargo test -p cortex-engine --test backup_restore corrupt_backup
```

These backup archive corruption tests cover:

- corrupted `.acs` segment file inside a backup directory;
- corrupted `manifest.acm` inside a backup directory.

## Encrypted Backup Boundary

Encrypted backup is available as a local passphrase archive MVP through
`backup-encrypted` and `restore-encrypted`. This supports local release
evidence and operator drills. KMS-backed envelope encryption, remote object
restore, and compliance-grade custody remain future work documented in
[`ENCRYPTED_BACKUPS_DESIGN.md`](ENCRYPTED_BACKUPS_DESIGN.md).
