# Performance Trend History

Status: local production-candidate evidence, not a public SLA.

This document defines the Epic 4.3 performance trend gate. Its purpose is to
make regressions visible before release by keeping current machine-readable
reports next to release history fixtures.

## Gates

Run the gates in order:

```bash
make load-smoke-check
make load-suite-check
make single-node-performance-check
make performance-trend-check
```

Artifacts:

```text
target/load-smoke/report.json
target/load-suite/report.json
target/single-node-performance/report.json
target/performance-trends/report.json
```

Release history fixtures live under:

```text
fixtures/performance/history/<release>/
  load_smoke_report.json
  single_node_performance_report.json
```

## Workload Classes

The workload contract is stored in:

```text
fixtures/performance/workload_classes.json
```

Current classes:

| Class | Purpose |
| --- | --- |
| `local_http_smoke` | Actor-backed HTTP burst with writes, reads, search, ContextPack, and VerifyFact. |
| `local_single_node_lifecycle` | Embedded engine lifecycle for Strict and Balanced durability profiles. |

These are release smoke workloads. They are intentionally small and repeatable;
they do not replace deployment-specific stress tests.

## Percentile Gates

`load-smoke-check` records p50/p95/p99 latency and gates p95/p99 latency for:

- write;
- read;
- search;
- context;
- verify.

`single-node-performance-check` records p50/p95/p99 latency and gates p95/p99
latency for:

- `put_single`;
- `get_latest`;
- `keyword_search`;
- `context_pack`;
- `verify_fact`.

The trend report compares current p50/p95/p99 values against the latest release
fixture and keeps ratios in `target/performance-trends/report.json`.

## Actor Pressure

The HTTP load report records:

- actor queue depth;
- actor queue capacity;
- queue saturation;
- `database_busy` / rejected request count.

For the local smoke workload, any `database_busy` count is a failure. This keeps
queue pressure visible before release even when functional requests otherwise
succeed.

## RPO/RTO Boundary

The workload classes link to the single-node RPO/RTO boundary in
[`RPO_RTO.md`](RPO_RTO.md). Current RPO/RTO evidence is local and comes from:

```bash
make backup-drill-check
make storage-soak-check
make storage-soak-history-check
make single-node-performance-check
```

The trend gate does not claim production SLA behavior. It proves that the local
release candidate has comparable performance evidence to prior release fixtures.
The storage soak history report is the source of truth for whether accumulated
local soak duration has crossed the 24-hour threshold.
