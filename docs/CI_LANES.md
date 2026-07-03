# CI Lanes

CortexDB CI is split by cost and release risk.

| Lane | Trigger | Checks |
| --- | --- | --- |
| PR | `pull_request`, `push` to `main` | file-size, docs-link, stable `cargo check`, full workspace tests, fmt, clippy, live examples, migration policy, storage-format change note, EnterpriseRAG fixture quality/parity/query-understanding gates |
| Benchmark-validation | nightly schedule, manual dispatch (`nightly.yml` job `benchmark-validation`) | benchmark_report/floor/results-page/AAB-mini schema+score gates, LTR corpus + learned-ranker held-out lift, retrieval candidate-pool / corpus-BM25 / vector-metric / two-stage-rerank / temporal-supersede / AQL-diversity / structure-chunking gates, anti-absorption moat proof (AAB-conformance, GCE-spec, receipt threat-model, receipt-emission budget) |
| Nightly-heavy / ANN-scale | daily schedule, manual dispatch | beta toolchain check/test/fmt/clippy, load smoke, crash/fault, backup offsite, chaos restart, replication partition/lifecycle, dashboard package/smoke/screenshots, ANN regression/release evidence, continuous scale benchmark gate |
| Release | `v*` tag, manual dispatch on tag | full `make release-check`, release evidence bundle, ANN baseline package, container image, binary release artifacts |

The **machine-readable** lane map is [`fixtures/benchmarks/lanes.v1.json`](../fixtures/benchmarks/lanes.v1.json):
each benchmark / retrieval / moat gate is pinned to exactly one lane with its
`required_env`. `make benchmark-lane-audit-check` (F1.3) fails CI if a gate is
defined but never scheduled, scheduled but undocumented, or listed in two lanes —
enforced bidirectionally against the `benchmark-validation` job in `nightly.yml`.

Specialized ANN and continuous benchmark workflows remain manual entry points,
but their scheduled coverage is owned by `nightly.yml`.
