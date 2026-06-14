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
make continuous-benchmark-gate
make continuous-benchmark-hosted-gate
```

Artifacts:

```text
target/load-smoke/report.json
target/load-suite/report.json
target/single-node-performance/report.json
target/performance-trends/report.json
target/continuous-benchmark-gate/report.json
```

The hosted/nightly CI path is `.github/workflows/continuous-benchmark.yml`. It
runs `make continuous-benchmark-hosted-gate`, which generates fresh
`load-smoke`, `single-node-performance`, and CI-safe fixed-payload 10K/100K
scale reports before applying the same `continuous-benchmark-gate` ratio policy.
The workflow uploads the benchmark JSON and Markdown reports as the
`continuous-benchmark-reports` artifact.

The continuous benchmark gate preserves the 1.2 p95/p99 ratio threshold and
uses `CONTINUOUS_BENCHMARK_MIN_REGRESSION_DELTA_MS=25` by default. That keeps
small absolute jitter from sub-25ms measurements visible in the artifact without
failing the run; larger p95/p99 deltas still fail the gate.

Hosted runs compare against same-profile fixtures under:

```text
fixtures/performance/hosted-history/
```

This keeps GitHub-runner trend gating separate from the older local release
history while preserving the same ratio and delta policy.

Release history fixtures live under:

```text
fixtures/performance/history/<release>/
  load_smoke_report.json
  single_node_performance_report.json
```

The current local regression baseline is:

```text
fixtures/performance/history/v0.2.0-beta.2/
```

This keeps the p95/p99 regression gate comparing current runs against the latest
beta evidence instead of the older `v0.1.0-core-alpha.5` smoke profile.

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

## Full-Corpus Scale Evidence

The default trend gate stays small, but EPIC-16 has a separate local
EnterpriseRAG full-corpus retrieval artifact for scale tracking:

```text
target/enterprise-rag-bench/official-clean/50/epic16-full-corpus-slo/retrieval_report.json
```

That artifact is produced by the official-clean retrieval-only path. It strips
oracle fields from the question input, ingests the full 511,958-document corpus,
checkpoints it, loads the cached lexical retrieval index, and retrieves 50
questions without invoking an answer or judge model.

Latest local values:

| Metric | Value |
| --- | ---: |
| Ingest throughput | 16,262.927 docs/sec |
| Checkpoint duration | 686.543s |
| Retrieval throughput | 0.229 questions/sec |
| Peak RSS | 20,210,786,304 bytes |
| Total retrieval-stage duration | 936.918s |

This artifact is evidence, not a passing release gate. The current release trend
gate remains `make performance-trend-check`; the full-corpus report identifies
the next performance work: checkpoint/index publication, cached index load time,
and per-question retrieval latency.

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

It also records and gates:

- `put_batch` ingest throughput through `ingest.throughput_per_sec`;
- process `rss_bytes` and `peak_rss_bytes` through `resource_usage`;
- profile-level `slo.passed` and `slo.errors`.

The trend report compares current p50/p95/p99 values against the latest release
fixture and keeps ratios in `target/performance-trends/report.json`.
The current report must include ingest and RSS evidence even when older release
fixtures predate those fields.

`make continuous-benchmark-gate` applies the current local regression threshold:

```text
max p95/p99 ratio: 1.2
```

It also checks that A19 scale trends and C16 memory-estimate audit artifacts are
well-formed when present. Local A19 trend reports are expected to be `complete`;
the hosted gate may use the smaller `scale-bench-ci` 10K/100K fixed-payload
profile to keep nightly runtime and memory bounded.

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
