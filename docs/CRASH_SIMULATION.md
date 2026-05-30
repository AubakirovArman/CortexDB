# Crash Simulation Matrix

This document tracks the Core Alpha crash/restart/corruption harness. The
matrix is intentionally deterministic and file-level; it does not require
process killing or external services.

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
| CLI repair removes orphan temp file and truncates a partial WAL tail. | `make crash-fault-check` |

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

## Current Limits

- The harness simulates crash aftermath by writing or corrupting files directly.
- It does not yet inject failures between every internal checkpoint step.
- It does not yet run randomized kill/restart loops.
