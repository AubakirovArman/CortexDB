# CortexDB Metrics and Observability

Version: `v0.1.0-core-alpha`

This document defines the Core Alpha observability surface. It complements
[`OPERATIONS.md`](OPERATIONS.md), [`API.md`](API.md), and
[`ANN_PRODUCTION_TUNING.md`](ANN_PRODUCTION_TUNING.md).

## Scope

CortexDB exposes lightweight operational metrics for a local single-node
database process:

- storage and checkpoint progress;
- WAL writer activity;
- MemTable size;
- ANN/HNSW graph state;
- actor queue backpressure;
- request counters and duration totals.

These metrics are suitable for smoke tests, dashboards, and release evidence.
They are not yet a full production telemetry stack with tracing, histograms,
alert routing, or long-term retention.

## HTTP Endpoints

### `GET /v1/metrics`

Returns a unified JSON payload with storage, WAL, MemTable, ANN/HNSW, actor,
and request counters. This is the recommended endpoint for dashboards and SDK
health probes.

Example:

```bash
curl -s http://127.0.0.1:18080/v1/metrics | jq .
```

Important fields:

| Field | Meaning | Operator signal |
| --- | --- | --- |
| `current_seq` | Current database commit sequence. | Monotonic write progress. |
| `checkpoint_seq` | Last durable checkpoint sequence. | Lag vs `current_seq` shows checkpoint age. |
| `live_segments` | Active segment bundles. | High count suggests compaction pressure. |
| `retired_segments` | Retired segment bundles. | High count suggests GC/cleanup pressure. |
| `memtable_cells` | Unique cells in memory. | Growth indicates checkpoint/compaction need. |
| `memtable_versions` | MVCC versions in memory. | High ratio vs cells indicates patch/tombstone churn. |
| `wal_size_bytes` | Active WAL size. | Growth indicates checkpoint lag. |
| `wal_writer_records` | WAL records appended since writer start. | Write activity counter. |
| `wal_writer_bytes` | WAL bytes written since writer start. | Write volume counter. |
| `wal_writer_fsyncs` | WAL fsync calls. | Durability cost signal. |
| `wal_writer_batches` | WAL batches committed. | Group commit behavior signal. |
| `ann_graph_nodes` | Nodes in persisted ANN graph. | Zero means ANN graph is unavailable. |
| `ann_total_edges` | Base-layer ANN edges. | Sudden drops suggest graph rebuild/corruption checks. |
| `ann_persisted_segments` | Segments with persisted ANN data. | ANN coverage across live storage. |
| `ann_has_checkpoint` | Whether ANN has checkpointed graph evidence. | False forces exact/fallback behavior. |
| `ann_has_uncheckpointed_changes` | Whether WAL tail is newer than ANN graph. | True can disable evaluation evidence. |
| `ann_search_requests` | ANN-capable search responses observed by the HTTP surface. | Use with `ann_fallbacks` for fallback rate. |
| `ann_fallbacks` | ANN-capable searches that reported exact/fallback behavior. | Sustained growth means ANN is not meeting its runtime guardrails. |
| `ann_no_fallback_requests` | ANN responses that included a no-fallback rollout decision. | Confirms operators are explicitly exercising fallback-free guardrails. |
| `ann_no_fallback_allowed` | No-fallback rollout decisions that allowed serving. | Should only rise for proven profile-scoped rollout requests. |
| `ann_no_fallback_blocked` | No-fallback rollout decisions blocked by guardrails. | Any increase requires inspection before retrying fallback-free rollout. |
| `actor_queue_depth` | Current per-tenant actor queue depth. | Sustained high values show backpressure. |
| `actor_queue_capacity` | Configured actor queue capacity. | Used with depth to compute saturation. |
| `request_count` | Requests handled by the process. | Traffic counter. |
| `request_rejected` | Requests rejected by limits/backpressure. | Alert if nonzero under normal traffic. |
| `request_duration_ms_total` | Sum of request durations in ms. | Use with request count for rough mean latency. |
| `validation_failures` | `/v1/validate` responses that reported storage errors. | Any increase requires operator review. |

### `GET /v1/metrics?format=prometheus`

Returns a minimal Prometheus text exposition for the main storage, WAL,
MemTable, ANN/HNSW, actor pressure, request rejection, ANN fallback, and
validation-failure counters. JSON remains the richer source for full typed
metrics.

Example:

```bash
curl -s 'http://127.0.0.1:18080/v1/metrics?format=prometheus'
```

Reference scrape configuration:

```text
examples/observability/prometheus.yml
```

The companion alert rules are in:

```text
examples/observability/alerts.yml
```

### `GET /v1/ann/metrics`

Returns ANN/HNSW-specific graph state:

| Field | Meaning |
| --- | --- |
| `graph_nodes` | Number of vectors represented in graph links. |
| `total_edges` | Number of base-layer graph edges. |
| `persisted_segments` | Segment bundles contributing ANN indexes. |
| `has_checkpoint` | True after at least one checkpoint writes ANN evidence. |
| `has_uncheckpointed_changes` | True when newer WAL data may require exact fallback. |
| `deleted_vectors` | Vectors removed but not yet rebuilt away. |
| `rebuild_count` | HNSW rebuild count in the active process/index state. |

Example:

```bash
curl -s http://127.0.0.1:18080/v1/ann/metrics | jq .
```

## Dashboard Examples

Grafana example JSON is checked in at:

```text
examples/observability/grafana-cortexdb-core-alpha.json
```

It includes panels for commit/checkpoint progress, WAL size, WAL write rate,
segment counts, ANN graph shape, actor queue pressure, ANN fallback rate, and
validation failures.

## CLI Probes

Use these commands during local operations:

```bash
cargo run -p cortex-cli -- stats ./data
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- ann-validate ./data
```

Use these gates for release evidence:

```bash
make load-smoke-check
make ann-fixture-check
make ann-drift-check
make ann-release-evidence-check
make release-check
```

## Basic Alert Guidelines

Treat these as Core Alpha operator heuristics, not production SLA guarantees:

- `request_rejected > 0`: inspect rate limit and actor queue pressure.
- `actor_queue_depth == actor_queue_capacity`: callers are overdriving the
  local database actor; expect `503 database_busy`.
- `wal_size_bytes` grows while `checkpoint_seq` does not advance: checkpoint is
  lagging or failing.
- `live_segments` keeps growing: compaction is not keeping up.
- `ann_fallbacks / ann_search_requests > 0.10` over five minutes: ANN is
  frequently falling back; inspect SLO violations and graph freshness.
- `ann_no_fallback_blocked` increases: keep fallback-free serving disabled for
  that profile, inspect `no_fallback_decision.reasons`, and re-run ANN evidence.
- `validation_failures` increases: stop promotion and inspect `/v1/validate`
  output before continuing writes.
- `ann_has_uncheckpointed_changes = true`: ANN evaluation can be unavailable;
  exact vector search remains the correctness path.
- ANN search responses with `production_safe=false`: do not treat HNSW as the
  reliable default for that query; inspect `slo_violations`.

More detailed alert examples and first-response actions are in
[`OBSERVABILITY_ALERTS.md`](OBSERVABILITY_ALERTS.md).

## Release Evidence

`make release-check` writes or refreshes local reports under `target/`, including
load smoke, ANN evidence, backup/offsite evidence, crash/fault evidence, chaos
restart evidence, and replication evidence. These files are local artifacts and
are not committed by default.

Current documented release evidence is summarized in
[`RELEASE_NOTES_v0.1.0-core-alpha.md`](RELEASE_NOTES_v0.1.0-core-alpha.md).

## Future Work

Post-Core Alpha observability should add:

- structured tracing spans for request, WAL, checkpoint, compact, and search;
- latency histograms instead of duration totals only;
- Prometheus coverage for actor/request counters;
- configurable alert profiles;
- long-running ANN recall/latency history;
- production failover and recovery SLO reports.
