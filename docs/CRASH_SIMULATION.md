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

## Current Limits

- The harness simulates crash aftermath by writing or corrupting files directly.
- It does not yet inject failures between every internal checkpoint step.
- It does not yet run randomized kill/restart loops.
