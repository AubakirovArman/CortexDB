# Release Regression Dashboard

Status: Core Alpha release regression gate.

The release regression dashboard compares the current release candidate against
the previous checked-in release fixture across the public release surfaces:

- storage;
- search;
- ContextPack;
- VERIFY;
- HTTP/API contract;
- SDK release contract.

Run:

```bash
make release-regression-dashboard-check
```

Artifacts:

```text
target/release-regression-dashboard/report.json
target/release-regression-dashboard/dashboard.md
```

The default baseline is:

```text
fixtures/release_regression/history/v0.1.0-core-alpha.5/report.json
```

## Metrics

The dashboard tracks:

- backup drill count and single-node lifecycle duration;
- retrieval recall, MRR, and p95 latency;
- ContextPack token reduction, evidence coverage, citation coverage, and
  deterministic ordering;
- verification accuracy and false positive/negative counts;
- HTTP contract checks passed;
- SDK package count and release checks passed.

Quality and contract metrics must stay at or above the previous release.
Latency and duration metrics may regress only within the configured local smoke
ratio. This is a release-candidate comparison, not a production SLA.

## Release Bundle

When present, the JSON report and Markdown dashboard are included in the unified
release evidence bundle by `make release-evidence-bundle-check`.
