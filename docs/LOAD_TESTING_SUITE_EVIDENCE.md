# Load Testing Suite Evidence

Focused gate:

```bash
make load-suite-check
```

Primary artifact:

```text
target/load-suite/report.json
```

## What It Proves

The gate verifies that CortexDB can run these local single-node HTTP workload
classes against a real `cortex-server` process:

- read-heavy;
- write-heavy;
- context-heavy;
- verify-heavy;
- ingest-heavy;
- mixed-tenant.

The gate also checks final validation health and fails if the server reports
request rejections / `database_busy` during the run.

## Boundary

This is a deterministic local suite for release regression evidence. It is not:

- a production load test for a specific customer deployment;
- a cloud autoscaling benchmark;
- a replacement for 24h/72h storage soak evidence;
- an external benchmark submission.
