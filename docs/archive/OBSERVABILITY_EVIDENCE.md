# Observability Evidence

Last local observability run: 2026-06-15, passed.

Run:

```bash
make observability-check
```

Primary artifact:

```text
target/observability/report.json
```

## Coverage

This gate covers:

- `docs/METRICS.md` field coverage for `/v1/metrics` and `/v1/ann/metrics`;
- Prometheus scrape configuration in `examples/observability/prometheus.yml`;
- Prometheus alert examples in `examples/observability/alerts.yml`;
- Grafana dashboard JSON in
  `examples/observability/grafana-cortexdb-core-alpha.json`;
- operator action guidance in `docs/archive/OBSERVABILITY_ALERTS.md`;
- alert thresholds for WAL growth, checkpoint lag, stale backup evidence,
  actor queue pressure, actor queue wait p95, operational error rate,
  rate-limit spikes, ANN fallback rate, and validation failures.

## Boundary

This is operational visibility for a single-node Core Alpha process. It does
not claim managed alert routing or long-term metrics retention.

## Latest Local Checks

```text
metrics_fields_checked: 45
ann_fields_checked: 7
prometheus_scrape: true
alerts: true
grafana_dashboard_json: true
```
