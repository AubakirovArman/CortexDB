# Crash Simulation Matrix

This document tracks the Core Alpha crash/restart/corruption harness. The
matrix combines deterministic file-level tests with a repeatable local server
kill/restart loop.

## Covered Scenarios

| Scenario | Test file |
| --- | --- |
| Interrupted checkpoint leaves orphan segment/index bundle. | `crash_matrix.rs` |
| Corrupt `manifest.acm.tmp` after checkpoint. | `crash_matrix.rs` |
| Interrupted compact writes a bundle but never publishes manifest. | `crash_matrix.rs` |
| Checkpoint plus patch WAL tail survives restart. | `restart_matrix.rs` |
| Checkpoint plus tombstone WAL tail survives restart. | `restart_matrix.rs` |
| Compact plus patch WAL tail survives restart. | `restart_matrix.rs` |
| Compact plus tombstone WAL tail survives restart. | `restart_matrix.rs` |
| Corrupt live `.acs` blocks open. | `corruption_matrix.rs` |
| Corrupt live `.acm` blocks open. | `corruption_matrix.rs` |
| Corrupt live `.acb` is reported by validation. | `corruption_matrix.rs` |
| Corrupt live `.aci` is reported by validation. | `corruption_matrix.rs` |
| corruption of `.acs`, `.acb`, `.aci`, `.acv`, and `.ach` is detected by open or validation. | `corruption_matrix.rs` |
| CLI repair removes orphan temp file and truncates a partial WAL tail. | `make crash-fault-check` |
| HTTP server survives repeatable forced kill/restart cycles after writes, flushes, and compacts. | `make chaos-restart-check` |

## Release Evidence

Run the deterministic gate:

```bash
make crash-fault-check
```

It runs the targeted crash/restart/corruption/repair test files, injects a
partial WAL tail and orphan temp file through the CLI repair path, validates the
database, reads the preserved cell payload back, and writes:

```text
target/crash-fault/report.json
```

The GitHub `Rust` workflow runs this gate on stable Rust and uploads
`crash-fault-evidence` with the JSON report and targeted test logs.

Run the repeatable process-level restart gate:

```bash
make chaos-restart-check
```

It starts the real `cortex-server` binary, writes cells through the HTTP API,
randomly mixes puts, flushes, compacts, and forced process kills using a fixed
seed, explicitly unlocks stale `db.lock`, repairs the database, restarts the
server, verifies every expected cell after each restart, and writes:

```text
target/chaos-restart/report.json
```

The GitHub `Rust` workflow runs this gate on stable Rust and uploads
`chaos-restart-evidence` with the JSON report and server log.

## Current Limits

- `crash-fault-check` simulates crash aftermath by writing or corrupting files
  directly.
- `chaos-restart-check` kills the server between completed API operations; it
  does not yet kill exactly between every internal checkpoint/compact step.
- It does not yet inject failures between every internal checkpoint step.
- The default chaos loop is deterministic for release reproducibility; increase
  `CHAOS_RESTART_STEPS` or change `CHAOS_RESTART_SEED` for longer local runs.
