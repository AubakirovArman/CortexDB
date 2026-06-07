# CortexDB Public Benchmarks

Status: public release-by-release benchmark history for local evidence gates.

This page summarizes the benchmark evidence that can be published without
claiming production SLA behavior. It points to machine-readable gates and
evidence docs rather than replacing them.

Run the page gate:

```bash
make public-benchmarks-check
```

## Release Summary

| Release | Status | Benchmark evidence | Public boundary |
| --- | --- | --- | --- |
| `v0.1.0-core-alpha.5` | Core Alpha prerelease | ANN smoke baseline package, demo-domain ANN baseline package, local binary package validation, dashboard package, and Core Alpha release notes. | Local single-node evidence only; no production consensus, managed cloud, legal-grade verification, or fallback-free HNSW claim. |
| `v0.2.0-beta.1` | Beta target candidate | `make beta-release-check`, `make retrieval-quality-check`, `make context-pack-quality-check`, `make verification-quality-check`, `make single-node-performance-check`, `make performance-trend-check`, and `make public-claims-check`. | Local single-node developer/API beta evidence; no hosted embedding CI requirement, no public registry publication claim, no production SLA. |

## Latest Public Metrics Snapshot

These are the latest checked-in documentation snapshots from local gates.
Regenerate the source reports before cutting a release.

| Area | Latest snapshot |
| --- | --- |
| Retrieval quality | 4 domains: `investment_projects`, `legal_policies`, `support_tickets`, `technical_docs`; beta fixture `production_safe=true`; no regression in local history. See [`PUBLIC_RETRIEVAL_BENCHMARKS.md`](PUBLIC_RETRIEVAL_BENCHMARKS.md). |
| Real embedding history | 3 local `investment_projects` runs; `vectors=221`, `queries=40`, `mean_recall_q16=65535`, `mean_mrr_q16=65535`, `mean_ndcg_q16=65535`, `exact_parity_q16=65535`, `production_safe=true`. |
| ContextPack quality | 25 cases across 5 domains; `evidence_coverage_q16=65535`, `citation_coverage_q16=65535`, `redundancy_reduction_q16=65535`, deterministic ordering coverage. |
| Verification quality | 50 deterministic cases across 5 domains with support, contradiction, mixed evidence, insufficient evidence, numeric guards, and source trust coverage. |
| Single-node performance | Local p95/p99 lifecycle matrix via `make single-node-performance-check`; trend comparison via `make performance-trend-check`. |
| LongMemEval official local run | Official LongMemEval v1 small split: retrieval `session recall_all@10=0.9021`, `session ndcg_any@10=0.7873`; official GPT-4o QA accuracy `0.7660`. |
| MultiHop-RAG local run | Official MultiHop-RAG retrieval evaluator over full 2556-query dataset: `Hits@10=0.9902`, `Hits@4=0.9295`, `MAP@10=0.4503`, `MRR@10=0.7906`; DeepSeek Flash QA with official `qa_evaluate.py`: overall `F1=0.75`, `Accuracy=0.67`; repeat 50-query prompt-cache hit rate `95.94%`; no leaderboard claim. |

## Epic 136 Task Coverage

This section is the public benchmark page contract. It maps each published
benchmark category to its refresh command and source document.

| Task | Published evidence | Refresh command | Source |
| --- | --- | --- | --- |
| Storage benchmarks | Single-node lifecycle matrix for put/get/search/ContextPack/Verify under local durability profiles; storage evidence is local, not an SLA. | `make single-node-performance-check` | [`BENCHMARKS.md`](BENCHMARKS.md) |
| Retrieval benchmarks | Multi-domain retrieval table with dataset sizes, recall, MRR, nDCG, p95/p99/max latency fields, exact-vs-ANN boundary, and limitations. | `make public-retrieval-benchmark-page-check` | [`PUBLIC_RETRIEVAL_BENCHMARKS.md`](PUBLIC_RETRIEVAL_BENCHMARKS.md) |
| ContextPack benchmarks | 25-case ContextPack quality fixture across 5 domains with evidence coverage, citation coverage, redundancy, anomaly, and deterministic ordering metrics. | `make context-pack-quality-check` | [`CONTEXT_PACK_QUALITY_EVIDENCE.md`](CONTEXT_PACK_QUALITY_EVIDENCE.md) |
| Verify benchmarks | 203-case deterministic verification fixture across 5 domains with supported/contradicted/mixed/insufficient labels and guard coverage; latest raw metric `case_count: 203`, `accuracy_q16=65535`. | `make verification-quality-check` | [`VERIFICATION_QUALITY_EVIDENCE.md`](VERIFICATION_QUALITY_EVIDENCE.md) |
| LongMemEval results | Official local LongMemEval v1 retrieval metrics plus clearly separated local QA score; this is not a leaderboard entry. | `make longmemeval-v1-official-retrieval-metrics` | [`LONGMEMEVAL_OFFICIAL.md`](LONGMEMEVAL_OFFICIAL.md) |
| Release trends | Release trend comparison for local smoke and single-node performance evidence, including p50/p95/p99 fields and regression boundaries. | `make performance-trend-check` | [`PERFORMANCE_TREND_HISTORY.md`](PERFORMANCE_TREND_HISTORY.md) |

## Evidence Sources

| Evidence | Command | Document |
| --- | --- | --- |
| Beta bundle | `make beta-release-check` | [`BETA_RELEASE.md`](BETA_RELEASE.md) |
| Public retrieval benchmarks | `make public-retrieval-benchmark-page-check` | [`PUBLIC_RETRIEVAL_BENCHMARKS.md`](PUBLIC_RETRIEVAL_BENCHMARKS.md) |
| Retrieval quality | `make retrieval-quality-check` | [`RETRIEVAL_QUALITY_EVIDENCE.md`](RETRIEVAL_QUALITY_EVIDENCE.md) |
| ContextPack quality | `make context-pack-quality-check` | [`CONTEXT_PACK_QUALITY_EVIDENCE.md`](CONTEXT_PACK_QUALITY_EVIDENCE.md) |
| Verification quality | `make verification-quality-check` | [`VERIFICATION_QUALITY_EVIDENCE.md`](VERIFICATION_QUALITY_EVIDENCE.md) |
| Single-node performance | `make single-node-performance-check` | [`BENCHMARKS.md`](BENCHMARKS.md) |
| LongMemEval official local run | `make longmemeval-v1-official-retrieval-metrics`, then official generation/eval | [`LONGMEMEVAL_OFFICIAL.md`](LONGMEMEVAL_OFFICIAL.md) |
| MultiHop-RAG local preparation | `make multihop-rag-local-50-check` | [`MULTIHOP_RAG_BENCHMARK.md`](MULTIHOP_RAG_BENCHMARK.md) |
| Performance trend | `make performance-trend-check` | [`PERFORMANCE_TREND_HISTORY.md`](PERFORMANCE_TREND_HISTORY.md) |
| Public claims | `make public-claims-check` | [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) |

## How To Refresh Before Release

```bash
make single-node-performance-check
make retrieval-quality-check
make context-pack-quality-check
make verification-quality-check
make longmemeval-v1-official-retrieval-metrics
make performance-trend-check
make public-benchmarks-check
```

For beta publication, run:

```bash
make beta-release-check
```

## Non-claims

This page does not claim:

- production distributed consensus;
- production SLA or hosted latency guarantees;
- managed cloud readiness;
- MultiHop-RAG leaderboard placement;
- legal-grade verification;
- audited financial assurance;
- fallback-free production HNSW;
- quality on private customer datasets.

Public wording must stay aligned with [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md).
