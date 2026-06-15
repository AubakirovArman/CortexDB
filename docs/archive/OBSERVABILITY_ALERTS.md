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
- actor queue wait p95: `cortexdb_actor_queue_wait_p95_ms`;
- database busy/rejected requests: `cortexdb_request_rejected`;
- operational error rate:
  `(increase(cortexdb_request_rejected[5m]) + increase(cortexdb_validation_failures[5m])) / increase(cortexdb_request_count[5m])`;
- quota/rate-limit spikes:
  `cortexdb_principal_quota_requests_rejected`,
  `cortexdb_principal_quota_body_bytes_rejected`, and
  `cortexdb_principal_quota_queue_rejected`;
- stale backup evidence: `cortexdb_backup_latest_age_seconds`;
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
| `CortexDbActorQueueWaitP95High` | Reduce client concurrency, inspect actor queue pressure, then check disk/WAL latency before raising queue capacity. |
| `CortexDbDatabaseBusy` | Inspect rate limits and actor saturation; retry with backoff rather than tight loops. |
| `CortexDbOperationalErrorRateHigh` | Compare rejected requests with validation failures; reduce client load first, then inspect storage validation output. |
| `CortexDbRateLimitSpike` | Identify the principal that exceeded request/body/queue quota and adjust caller backoff before raising limits. |
| `CortexDbBackupStale` | Run backup validation, check backup destination health, and refresh backup evidence before promotion. |
| `CortexDbBackupEvidenceMissing` | Set `CORTEXDB_BACKUP_ROOT` or create the local backup evidence directory used by the server. |
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

1. Compare `actor_queue_depth` with `actor_queue_capacity` and
   `actor_queue_wait_p95_ms`.
2. Check `request_rejected` and application retry behavior.
3. Reduce client concurrency or add backoff.
4. Increase queue capacity only when disk/WAL latency is healthy.

### Operational Error Rate

1. Separate validation failures from request rejections.
2. If validation failures increased, stop release promotion and save
   `/v1/validate` output before taking repair actions.
3. If only request rejections increased, reduce caller concurrency and inspect
   rate-limit or actor queue alerts.
4. Re-run `make load-smoke-check` after tuning caller backoff or queue limits.

### Rate-limit Spike

1. Check which quota counter increased: request count, body bytes, or actor
   queue admission.
2. Verify the caller is using retry with backoff, not tight retry loops.
3. Raise per-principal limits only after confirming the workload is expected.
4. If queue quota is the cause, inspect actor queue pressure before changing
   limits.

### Backup Evidence

1. For stale backup evidence, run the backup drill and verify the destination
   can still be restored.
2. For missing backup evidence, set `CORTEXDB_BACKUP_ROOT` or create the local
   backup evidence directory next to the database root.
3. Treat `backup_latest_age_seconds = -1` as "unknown", not as a successful
   recent backup.
4. Re-run `make backup-offsite-check` before release promotion.

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

Core Alpha now exposes HTTP/queue/engine tracing spans, queue-wait and ANN
latency buckets, and alert-rule examples, but it does not provide managed
alert routing, paging, or long-term metric retention. Use Prometheus/Grafana
externally for those concerns.
