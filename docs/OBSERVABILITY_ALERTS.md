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
- actor queue pressure: `cortexdb_actor_queue_depth / cortexdb_actor_queue_capacity`;
- database busy/rejected requests: `cortexdb_request_rejected`;
- missing persisted ANN graph: `cortexdb_ann_graph_nodes == 0`;
- ANN fallback rate:
  `increase(cortexdb_ann_fallbacks[5m]) / increase(cortexdb_ann_search_requests[5m])`;
- no-fallback rollout blocks:
  `increase(cortexdb_ann_no_fallback_blocked[5m])`;
- ANN runtime p99 latency:
  `histogram_quantile(0.99, sum(rate(cortexdb_ann_search_latency_ms_bucket[5m])) by (le))`;
- validation failures: `cortexdb_validation_failures`.

## Suggested Actions

| Alert | First action |
| --- | --- |
| `CortexDbWalCheckpointLag` | Run `cortexdb validate`, inspect server logs, then checkpoint/compact during a maintenance window. |
| `CortexDbWalGrowth` | Check whether checkpoint is stuck and confirm free disk before restart. |
| `CortexDbActorQueuePressure` | Reduce client concurrency or raise `--actor-queue-capacity` only after checking disk/WAL latency. |
| `CortexDbDatabaseBusy` | Inspect rate limits and actor saturation; retry with backoff rather than tight loops. |
| `CortexDbAnnGraphUnavailable` | Use exact vector search as the correctness path; checkpoint/compact to rebuild ANN evidence. |
| `CortexDbAnnFallbackRate` | Inspect ANN search reports for SLO violations, graph freshness, and visit-budget fallback reasons. |
| `CortexDbAnnNoFallbackBlocked` | Keep fallback-free serving disabled for that profile, inspect `no_fallback_decision.reasons`, and re-run ANN release evidence before retrying rollout. |
| `CortexDbAnnSearchLatencyP99High` | Keep no-fallback rollout disabled, inspect traffic shape and graph profile, then re-run ANN latency evidence. |
| `CortexDbValidationFailures` | Stop release promotion, save `/v1/validate` output, and run `cortexdb validate` plus backup/restore checks. |

## Operator Playbooks

### WAL Checkpoint Lag

1. Run `cortexdb validate <db-path>` and save the output.
2. Check free disk and WAL size.
3. Run `cortexdb flush <db-path>` during a quiet window.
4. If lag returns immediately, run `make storage-soak-check` locally before
   release promotion.

### Actor Queue Pressure

1. Compare `actor_queue_depth` with `actor_queue_capacity`.
2. Check `request_rejected` and application retry behavior.
3. Reduce client concurrency or add backoff.
4. Increase queue capacity only when disk/WAL latency is healthy.

### ANN Fallback Rate

1. Inspect `/v1/search` ANN reports for `fallback_reason` and SLO violations.
2. Check `ann_has_uncheckpointed_changes` and checkpoint if graph evidence is
   stale.
3. Use exact vector search for correctness while graph tuning is investigated.
4. Re-run ANN release evidence before promoting a release.

### ANN No-fallback Blocks

1. Inspect `no_fallback_decision.reasons` from the blocked request.
2. If the reason is recall, graph topology, stale graph, or fallback-enabled
   policy, keep `no_fallback_profile=active` disabled for production traffic.
3. Rebuild/checkpoint/compact the graph only after validating corpus and vector
   generation consistency.
4. Re-run `make ann-production-no-fallback-check` and the relevant history
   gate before re-enabling the operator profile.

### Validation Failures

1. Treat any validation failure as release-blocking.
2. Save `/v1/validate` and `cortexdb validate` output.
3. Run backup/restore drill against the latest validated backup.
4. Do not continue writes on suspected corruption until the failure mode is
   understood.

## Boundary

Core Alpha does not yet provide latency histograms, tracing spans, alert
routing, or long-term metric retention. Use Prometheus/Grafana externally for
those concerns.
