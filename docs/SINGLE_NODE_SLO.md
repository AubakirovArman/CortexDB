# Single-Node SLO Boundaries

Status: local production-candidate evidence, not a public SLA.

This document defines the current single-node SLO signals used by release
checks. The numbers are local gates for repeatability, not universal cloud or
enterprise guarantees.

## Current SLO Signals

| Signal | Gate | Artifact |
| --- | --- | --- |
| Lifecycle duration, embedded latency, ingest throughput, RSS | `make single-node-performance-check` | `target/single-node-performance/report.json` |
| Load smoke | `make load-smoke-check` | `target/load-smoke/report.json` |
| Load suite | `make load-suite-check` | `target/load-suite/report.json` |
| Performance trends | `make performance-trend-check` | `target/performance-trends/report.json` |
| Dashboard SLO panel | `make single-node-slo-dashboard-check` | `target/dashboard/single-node-slo-report.json` |
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
SINGLE_NODE_PERF_MIN_INGEST_CELLS_PER_SEC=1
SINGLE_NODE_PERF_MAX_RSS_BYTES=1073741824
```

The gate exercises strict and balanced lifecycle paths and fails if the total
local duration exceeds the configured budget, if profile-level ingest throughput
falls below the configured minimum, or if observed process peak RSS exceeds the
configured RSS budget.

`make load-smoke-check` also records p50/p95/p99 latency for write, read,
search, ContextPack, and VerifyFact flows. It records actor queue saturation
and fails if the local smoke workload observes `database_busy` / rejected
requests.

`make single-node-performance-check` records p50/p95/p99 latency for embedded
`put_single`, `get_latest`, `keyword_search`, `context_pack`, and
`verify_fact` flows in both Strict and Balanced durability profiles. The same
report records `put_batch` throughput as the embedded ingest proxy and process
`rss_bytes` / `peak_rss_bytes` from the local process status.

`make performance-trend-check` compares current p50/p95/p99 values with
checked-in release history under `fixtures/performance/history/` and writes:

```text
target/performance-trends/report.json
```

It also validates that the current single-node report contains profile-level
SLO status, ingest throughput, and RSS evidence. Older release fixtures may not
contain those fields, so the trend comparison only treats them as mandatory for
the current report.

Workload classes and local RPO/RTO expectations are defined in
`fixtures/performance/workload_classes.json`.

## EnterpriseRAG Full-Corpus Evidence

EPIC-16 also tracks a larger full-corpus retrieval path over the local
EnterpriseRAG fixture. This is not part of the small default release smoke gate,
but it is the current scale evidence for the 500k+ document path.

Latest local evidence:

```text
command:
python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
  --size 50 \
  --run-label epic16-full-corpus-slo \
  --answer-provider deepseek \
  --judge-provider deepseek \
  --stage retrieval \
  --top-k 10 \
  --batch-size 1000 \
  --retrieval-progress-every 50000

artifact:
target/enterprise-rag-bench/official-clean/50/epic16-full-corpus-slo/retrieval_report.json
```

Observed on the local machine:

| Signal | Value |
| --- | ---: |
| Documents indexed | 511,958 |
| Ingest duration | 31.480s |
| Ingest throughput | 16,262.927 docs/sec |
| Checkpoint duration | 686.543s |
| Retrieval questions | 50 |
| Retrieval duration | 217.994s |
| Retrieval throughput | 0.229 questions/sec |
| Final RSS | 18,062,696,448 bytes |
| Peak RSS | 20,210,786,304 bytes |
| Total retrieval-stage duration | 936.918s |

Interpretation: the full-corpus ingest path is fast enough for local evidence,
while checkpoint publication, cached lexical index loading, and per-question
retrieval are the current EPIC-16 bottlenecks to optimize before treating the
500k+ path as a strong production SLO.

## Dashboard SLO View

The dashboard emits a browser-side `dashboard_slo.v1` summary on the Overview
route after the operator runs Refresh Status. It is derived from `/v1/health`,
`/v1/compatibility`, `/v1/stats`, `/v1/validate`, and `/v1/metrics`.

The view intentionally shows the five local single-node signals together:

- availability;
- request latency;
- backup freshness;
- validation status;
- error budget.

The dashboard check is structural: it verifies that these fields are wired into
the shipped static dashboard and server asset bundle. It does not replace the
load, performance trend, backup, or validation gates that produce the underlying
evidence.

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
- Performance trend history:
  [`PERFORMANCE_TREND_HISTORY.md`](PERFORMANCE_TREND_HISTORY.md).
