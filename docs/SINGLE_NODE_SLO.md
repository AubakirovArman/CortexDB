# Single-Node SLO Boundaries

Status: local production-candidate evidence, not a public SLA.

This document defines the current single-node SLO signals used by release
checks. The numbers are local gates for repeatability, not universal cloud or
enterprise guarantees.

## Current SLO Signals

| Signal | Gate | Artifact |
| --- | --- | --- |
| Lifecycle duration | `make single-node-performance-check` | `target/single-node-performance/report.json` |
| Load smoke | `make load-smoke-check` | `target/load-smoke/report.json` |
| Crash/fault recovery | `make crash-fault-check` | `target/crash-fault/report.json` |
| Backup restore drill | `make backup-drill-check` | `target/backup-drill/report.json` |
| API contract | `make openapi-contract-check` | command output |
| SDK/API compatibility | `make sdk-release-contract-check` | command output |

## Local Candidate Thresholds

The current single-node performance gate uses:

```bash
make single-node-performance-check
```

Default inputs:

```text
SINGLE_NODE_PERF_CELLS=500
SINGLE_NODE_PERF_MAX_TOTAL_MS=30000
```

The gate exercises strict and balanced lifecycle paths and fails if the total
local duration exceeds the configured budget.

## Operational Interpretation

- Passing this gate means the local build remains within the configured
  production-candidate smoke budget.
- Failing this gate means the release needs investigation before claiming a
  production-like single-node candidate.
- This does not replace workload-specific benchmarking on the deployment
  hardware.

## Related SLO Documents

- ANN/HNSW recall and latency guardrails:
  [`ANN_PRODUCTION_TUNING.md`](ANN_PRODUCTION_TUNING.md).
- Experimental consensus/failover SLO gates:
  [`CONSENSUS_SLO.md`](CONSENSUS_SLO.md).
- RPO/RTO boundaries:
  [`RPO_RTO.md`](RPO_RTO.md).
