# Backup and Restore

Core Alpha backups are validated filesystem snapshots of a closed or exclusively
locked single-node database directory.

## Commands

```bash
cortexdb backup ./db ./db.backup
cortexdb restore ./db.backup ./db.restored
cortexdb validate ./db.restored
```

## Safety Rules

- Backup takes the source database lock.
- The WAL writer is shut down before copying and restarted afterward.
- Source storage is validated before copying.
- `db.lock` and known temporary files are excluded.
- Restore only writes to a target path that does not already exist.
- Restore validates the copied database before reporting success.

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
- The source database must be opened exclusively by this process.
- The restored target must be new; in-place overwrite is intentionally refused.
