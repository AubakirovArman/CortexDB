# Storage Compatibility Evidence

Unified CortexDB public-surface versioning rules are defined in
[`VERSIONING_POLICY.md`](VERSIONING_POLICY.md). This document covers storage
format compatibility and restore evidence.

Last local storage compatibility run: 2026-06-01.

Run:

```bash
make storage-compat-check
```

Primary artifacts:

```text
target/storage-compat/report.json
target/storage-compat/*.log
target/migration-historical-restore/report.json
target/migration-upgrade-matrix-v2/report.json
target/backup-drill/report.json
target/crash-fault/report.json
target/chaos-restart/report.json
```

Companion Epic 4.1 soak artifact:

```text
target/storage-soak/report.json
target/storage-soak-history/report.json
target/storage-soak-history/history.jsonl
```

Latest local status: passed.

Latest retained 24-hour storage soak status:

```text
target/storage-soak-history/report.json
twenty_four_hour_evidence.met=true
total_duration_seconds=86476
run_count=981
total_cycles=19584
total_cells_written=979016
```

## Matrix

| Suite | Purpose |
| --- | --- |
| migration compatibility | Checks the machine-readable storage/API/SDK compatibility fixture. |
| historical restore fixture | Restores release-tagged backup fixtures with the current binary. |
| migration upgrade matrix v2 | Opens the previous-release direct database fixture, writes with the current binary, backs it up, restores it, and validates old plus new cells. |
| backup drill | Proves a current-version backup can be restored and validated by the checkout under test. |
| backup archive corruption | Proves corrupted backup segment and manifest archives are rejected on restore. |
| crash/fault | Runs interrupted checkpoint/compact, restart tail, corruption, and repair tests. |
| chaos restart | Kills/restarts the real server around writes, flushes, and compacts. |
| storage soak | Repeats write/flush/compact/backup/restore cycles and representative kill attempts. |
| storage soak history | Aggregates repeatable soak runs and reports whether accumulated 24-hour evidence exists. |
| repair dry-run | Proves repair dry-run reports planned cleanup without mutating files. |
| CLI repair dry-run | Proves the CLI exposes dry-run and apply paths. |

## Boundary

The local gate proves:

- storage compatibility evidence is repeatable locally;
- historical backup fixtures restore with the current binary;
- previous-release direct database fixtures open, accept current writes, and
  survive backup/restore with the current binary;
- current checkout can restore and validate current-version backups;
- corrupted backup archives are rejected during restore;
- known storage file corruption is detected;
- interrupted checkpoint/compact aftermath is covered by tests;
- repair dry-run and apply behavior are both covered.
- repeated storage cycles preserve data across backup/restore and partial WAL
  repair;
- storage soak history is accumulated in a de-duplicated local JSONL report.
- retained local storage soak history has crossed the 24-hour threshold.

The gate does not prove:

- online rolling upgrade;
- in-place downgrade;
- remote object-store restore;
- KMS-backed encrypted backup custody;
- kill injection at every internal checkpoint byte boundary.
