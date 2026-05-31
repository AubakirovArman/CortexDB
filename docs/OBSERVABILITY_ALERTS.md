# Observability Alert Examples

Core Alpha alerting is intentionally simple. Use these rules as local operator
heuristics, not as production SLO contracts.

## Prometheus Rules

Example rules live in:

```text
examples/observability/alerts.yml
```

They cover:

- checkpoint lag: `cortexdb_current_seq - cortexdb_checkpoint_seq`;
- large WAL size: `cortexdb_wal_size_bytes`;
- missing persisted ANN graph: `cortexdb_ann_graph_nodes == 0`.

## Suggested Actions

| Alert | First action |
| --- | --- |
| `CortexDbWalCheckpointLag` | Run `cortexdb validate`, inspect server logs, then checkpoint/compact during a maintenance window. |
| `CortexDbWalGrowth` | Check whether checkpoint is stuck and confirm free disk before restart. |
| `CortexDbAnnGraphUnavailable` | Use exact vector search as the correctness path; checkpoint/compact to rebuild ANN evidence. |

## Boundary

Core Alpha does not yet provide latency histograms, tracing spans, alert
routing, or long-term metric retention. Use Prometheus/Grafana externally for
those concerns.
