# CortexDB Upgrade And Rollback Guide

Status: Core Alpha offline single-node procedure.

CortexDB does not promise in-place downgrade compatibility in Core Alpha.
Rollback means restoring the pre-upgrade backup and starting the previous
binary against that restored directory.

## Pre-Upgrade Checklist

1. Stop writers and the HTTP server.
2. Record current versions:

   ```bash
   cortexdb --version
   cortex-server --help
   ```

3. Validate the database:

   ```bash
   cortexdb validate ./data
   cortexdb stats ./data
   ```

4. Run a restore drill:

   ```bash
   cortexdb backup-drill ./data ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade
   ```

5. Keep the backup immutable until the upgraded binary passes validation.

The CLI can run the validation, backup, and restore-drill preflight as one
operator command:

```bash
cortexdb upgrade prepare ./data ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade
```

This command returns the follow-up `cortexdb upgrade validate` and
`cortexdb upgrade rollback` commands. Use `--json` when automation needs a
stable response shape:

```bash
cortexdb --json upgrade prepare ./data ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade
```

## Upgrade

Install the new archive as described in [`INSTALL.md`](INSTALL.md):

```bash
sha256sum -c cortexdb-<version>-<platform>.tar.gz.sha256
tar -xzf cortexdb-<version>-<platform>.tar.gz
cd cortexdb-<version>-<platform>
sha256sum -c SHA256SUMS
install -m 0755 bin/cortexdb ~/.local/bin/cortexdb
install -m 0755 bin/cortex-server ~/.local/bin/cortex-server
```

Open and validate:

```bash
cortexdb upgrade validate ./data
```

If running under systemd:

```bash
systemctl restart cortexdb
systemctl status cortexdb
```

## Rollback

Do not run the old binary against an upgraded directory. Restore instead:

```bash
mv ./data ./data.failed-upgrade
cortexdb upgrade rollback ./backups/cortexdb-pre-upgrade ./data.rollback
```

Install the previous binaries and start them against `./data.rollback`.

## Compatibility Gates

Release candidates should pass:

```bash
make migration-policy-check
make migration-compatibility-check
make binary-release-check
make deployment-upgrade-check
```

The format-level compatibility policy remains in
[`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md).
