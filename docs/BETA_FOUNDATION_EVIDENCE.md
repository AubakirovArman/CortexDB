# Beta Foundation Evidence

Last local beta foundation run: 2026-05-31.

This document records the Epic 2 evidence gate from
[`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md). It is a developer-facing beta
foundation check, not a production readiness claim.

## Command

```bash
make beta-foundation-check
```

Result:

```text
passed
```

The command writes:

```text
target/beta-foundation/report.json
target/beta-foundation/*.log
```

## Suites

| Suite | Purpose |
| --- | --- |
| `sdk_contract` | Rust, Python, and TypeScript SDK e2e checks against a local server. |
| `openapi_contract` | OpenAPI contract and typed response compatibility. |
| `context_verify_quality` | ContextPack and VERIFY FACT deterministic quality fixture. |
| `search_quality` | BM25, field weighting, stopword, and multilingual analyzer checks. |
| `error_taxonomy` | Stable HTTP error code and status taxonomy. |
| `metrics_contract` | Metrics and ANN metrics response-shape checks. |
| `beta_delta` | Beta boundary docs and gate wiring. |

## Boundary

This gate proves that the Core Alpha developer surface is wired enough for
external experiments:

- SDKs decode real local-server responses;
- API snapshots and OpenAPI contract are current;
- ContextPack and VERIFY have deterministic quality fixtures;
- search quality fixtures run repeatably;
- error taxonomy and metrics contracts are covered.

It does not prove:

- large-scale beta traffic SLO history;
- production distributed consensus;
- unrestricted production HNSW/ANN without exact fallback;
- stable published SDK package availability.

Those remain later beta/product-layer work.
