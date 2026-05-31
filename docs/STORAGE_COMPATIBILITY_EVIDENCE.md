# Storage Compatibility Evidence

Last local storage compatibility run: 2026-05-31.

Run:

```bash
make storage-compat-check
```

Primary artifacts:

```text
target/storage-compat/report.json
target/storage-compat/*.log
target/backup-drill/report.json
target/crash-fault/report.json
target/chaos-restart/report.json
```

Latest local status: passed.

## Matrix

| Suite | Purpose |
| --- | --- |
| migration compatibility | Checks the machine-readable storage/API/SDK compatibility fixture. |
| backup drill | Proves a current-version backup can be restored and validated by the checkout under test. |
| crash/fault | Runs interrupted checkpoint/compact, restart tail, corruption, and repair tests. |
| chaos restart | Kills/restarts the real server around writes, flushes, and compacts. |
| repair dry-run | Proves repair dry-run reports planned cleanup without mutating files. |
| CLI repair dry-run | Proves the CLI exposes dry-run and apply paths. |

## Boundary

The local gate proves:

- storage compatibility evidence is repeatable locally;
- current checkout can restore and validate current-version backups;
- known storage file corruption is detected;
- interrupted checkpoint/compact aftermath is covered by tests;
- repair dry-run and apply behavior are both covered.

The gate does not prove:

- online rolling upgrade;
- in-place downgrade;
- remote object-store restore;
- kill injection at every internal checkpoint byte boundary.
