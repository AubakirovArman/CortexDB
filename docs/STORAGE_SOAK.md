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

The repeatable history gate runs a fresh soak, appends a de-duplicated entry to
local history, and writes an aggregate report:

```bash
make storage-soak-history-check
```

```text
target/storage-soak-history/report.json
target/storage-soak-history/history.jsonl
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

For the 24-hour acceptance threshold, run the history gate with an explicit
duration requirement after enough soak history has accumulated:

```bash
make storage-soak-history-check STORAGE_SOAK_HISTORY_MIN_HOURS=24
```

To actively accumulate that evidence, run the resumable campaign target:

```bash
make storage-soak-24h-campaign
```

The campaign repeatedly runs storage soak cycles and updates:

```text
target/storage-soak-history/campaign.json
target/storage-soak-history/report.json
target/storage-soak-history/history.jsonl
```

For a quick local smoke of the campaign wiring without claiming 24-hour
evidence:

```bash
make storage-soak-24h-campaign \
  STORAGE_SOAK_CAMPAIGN_TARGET_HOURS=0 \
  STORAGE_SOAK_CAMPAIGN_MAX_RUNS=1 \
  STORAGE_SOAK_CAMPAIGN_CYCLES=1 \
  STORAGE_SOAK_CAMPAIGN_CELLS_PER_CYCLE=1
```

Check campaign progress with:

```bash
make storage-soak-campaign-status
```

For machine-readable monitoring:

```bash
make storage-soak-campaign-status STORAGE_SOAK_CAMPAIGN_STATUS_FORMAT=json
```

The status includes accumulated soak duration, run/cycle/cell counts,
`progress_percent`, and the explicit `twenty_four_hour_met` boolean.

The default history gate does not pretend to be a 24-hour proof. It records
`twenty_four_hour_evidence.met=false` until accumulated local soak duration
crosses 24 hours.

## Boundary

This gate proves the checkout can survive repeated local durability cycles and
recover from representative kill attempts. It does not prove every internal
byte-boundary crash point, remote/offsite restore, encrypted backups, online
rolling upgrades, 24-hour soak unless the history report says the 24-hour
threshold is met, or multi-process writer safety.
