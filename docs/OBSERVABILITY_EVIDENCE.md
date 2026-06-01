# Observability Evidence

Last local observability run: 2026-06-01, passed.

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
- operator action guidance in `docs/OBSERVABILITY_ALERTS.md`;
- alert thresholds for WAL growth, checkpoint lag, actor queue pressure, ANN
  fallback rate, and validation failures.

## Boundary

This is operational visibility for a single-node Core Alpha process. It does
not claim production tracing, histogram-based latency SLOs, or managed alert
routing.

## Latest Local Checks

```text
metrics_fields_checked: 24
ann_fields_checked: 7
prometheus_scrape: true
alerts: true
grafana_dashboard_json: true
```
