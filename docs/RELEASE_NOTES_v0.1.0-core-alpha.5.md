# CortexDB v0.1.0-core-alpha.5 Release Notes

`v0.1.0-core-alpha.5` is an audited Core Alpha prerelease. It refreshes the
public release assets and release notes after the local `make release-check`
evidence pass on 2026-06-01.

## Included

- Durable single-node database core: WAL, MemTable MVCC, restart replay,
  checkpoint, compact, validation, and repair.
- AQL parser/binder/retrieve execution over bitmap filters.
- ContextPack v1, VERIFY FACT, search, ingestion smoke paths, CLI, HTTP API,
  and SDK smoke coverage.
- Typed server JSON responses and OpenAPI contract checks.
- Guarded ANN/HNSW evidence with exact fallback and packaged release baselines.
- Dashboard developer console package for local operations and smoke review.

## Public Assets

The public GitHub prerelease attaches:

- `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz`
- `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz.sha256`
- `dashboard-v1.tar.gz`
- `v0.1.0-core-alpha.5-ann-smoke.tar.gz`
- `v0.1.0-core-alpha.5-ann-demo-domain.tar.gz`

The binary package is accompanied by an external `.sha256` file. ANN baseline
packages include internal package manifests with per-file SHA-256 checksums.

Archive SHA-256 values from the local artifact audit:

| Asset | SHA-256 |
| --- | --- |
| `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz` | `1b1d3522c0ec35dddcc37f3f4552b231057add9e68b4e327f13314fd48b5cf39` |
| `dashboard-v1.tar.gz` | `6599d3c145d5d977275a240cf98926b3154968cb2a7f7b78ebc2092d964624b1` |
| `v0.1.0-core-alpha.5-ann-smoke.tar.gz` | `6e3221e5aa5d7cfc9e6e8ad118d9bc7abeeee652f76e23cfeca690875bc3aa80` |
| `v0.1.0-core-alpha.5-ann-demo-domain.tar.gz` | `e230864ac25f71b5a113d152eac492e2c6b1e04ee44a15cf1414d533b6399b04` |

## Evidence

The latest local evidence run used:

```bash
make release-check
```

and passed on 2026-06-01. The evidence is summarized in
[`RELEASE_EVIDENCE.md`](RELEASE_EVIDENCE.md).

## Explicit Limits

This prerelease does not claim:

- production distributed consensus;
- production HNSW without exact fallback;
- legal-grade verification;
- managed cloud service readiness;
- enterprise RBAC/compliance readiness;
- production OCR or document-ingestion pipelines.

Use this release as a local Core Alpha and SDK/API experimentation baseline, not
as a production database service.
