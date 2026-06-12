# CortexDB Chaos Scenario Matrix

This matrix is the EPIC-E11 source map for crash, restart, fault-injection, and
shutdown evidence. It keeps the harnesses distinct so new reliability work does
not duplicate an existing test path.

## Harnesses

| Harness | Command | Scope |
|---|---|---|
| Engine crash matrix | `cargo test -p cortex-engine --test crash_matrix` | Direct storage files around checkpoint/compact publication windows. |
| Engine fault injection | `cargo test -p cortex-engine --test crash_consistency_fault_injection` | 1000 deterministic WAL/checkpoint/compact interruption scenarios. |
| HTTP chaos restart | `make chaos-restart-check` | Real server process, API writes, flush, compact, forced kill/restart, repair, readback. |
| Storage soak | `make storage-soak-check` | Repeated write/flush/compact/validate cycles with kill-delay process interruption. |
| Graceful shutdown | `make chaos-restart-check` plus pending load harness | SIGTERM restart/readback is covered; in-flight drain under concurrent load is still open. |

## Current Coverage

- orphan segment or bundle before manifest publication;
- corrupt or partial manifest handling;
- interrupted checkpoint and compact paths;
- stale WAL/archive replay safety;
- partial WAL tail recovery;
- corrupt persisted files fail-closed or validate bad;
- server kill/restart around acknowledged HTTP writes;
- server SIGTERM/restart/readback around acknowledged HTTP writes;
- post-kill unlock, repair, validate, and readback.

## Known Gaps

- SIGTERM restart/readback is covered by `make chaos-restart-check`, but there
  is not yet a dedicated concurrent in-flight drain harness.
- Process-level reliability scripts share server lifecycle and HTTP helpers
  through `scripts/cortexdb_server_harness.py`.
- Long-running 24h/72h evidence belongs to soak campaigns, not the short E11
  correctness gate.

## Machine-Readable Report

Generate the current inventory:

```bash
make chaos-scenario-map-check
```

Default output:

```text
target/chaos-scenario-map/report.json
```

The report uses `schema_version = cortexdb.chaos_scenario_map.v1` and lists
scenario groups, duplicate scenario names, and known gaps.
