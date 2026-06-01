# Storage Soak

`make storage-soak-check` is the local long-running storage durability gate for
the production-candidate track. It complements `crash-fault-check` and
`chaos-restart-check` by repeating the normal database loop before fault
injection:

```bash
make storage-soak-check
```

Default output:

```text
target/storage-soak/report.json
```

The report records:

- repeated write, flush, compact, validate cycles;
- backup and restore verification after each cycle;
- partial WAL tail repair outcomes;
- process-kill attempts during checkpoint, compact, WAL replay, and restore;
- a release-tagged restore fixture from
  `fixtures/restore/v0.1.0-core-alpha/restore_fixture.json`.

The default run is intentionally short enough for local release evidence. Longer
runs can raise:

```bash
make storage-soak-check STORAGE_SOAK_CYCLES=20 STORAGE_SOAK_CELLS_PER_CYCLE=50
```

## Boundary

This gate proves the checkout can survive repeated local durability cycles and
recover from representative kill attempts. It does not prove every internal
byte-boundary crash point, remote/offsite restore, encrypted backups, online
rolling upgrades, or multi-process writer safety.
