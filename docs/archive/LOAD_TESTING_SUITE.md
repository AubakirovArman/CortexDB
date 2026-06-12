# CortexDB Load Testing Suite

Status: local single-node HTTP load suite for the current production epic plan.
This suite is intentionally small and repeatable; it is not a capacity
benchmark for a specific deployment.

Focused gate:

```bash
make load-suite-check
```

Primary report:

```text
target/load-suite/report.json
```

## Workloads

The suite starts a real `cortex-server` process and runs six workload classes:

| Workload | Purpose |
| --- | --- |
| `read_heavy` | Repeated `GET /v1/cell` lookups after a seed write phase. |
| `write_heavy` | Concurrent `POST /v1/cell` writes to exercise WAL and actor admission. |
| `context_heavy` | Repeated `POST /v1/context` calls over seeded cells. |
| `verify_heavy` | Repeated `POST /v1/verify` calls over seeded evidence. |
| `ingest_heavy` | Repeated `POST /v1/ingest/text` calls. |
| `mixed_tenant` | Repeated write/read cycles across multiple tenant realms. |

## Gate Behavior

The gate fails when:

- any workload returns request errors;
- `/v1/validate` is not healthy after the run;
- `request_rejected` reports actor busy / rejected requests;
- the total run exceeds its configured local smoke budget.

The report records per-workload latency summaries plus the final validation and
rejection status.

## Boundary

This is evidence that the HTTP surface can run the required workload classes on
a local machine without obvious regressions. It does not prove production
capacity, cloud autoscaling, multi-node HA, or hardware-specific SLOs.
