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
| Retrieval quality | 4 domains: `investment_projects`, `legal_policies`, `support_tickets`, `technical_docs`; beta fixture `production_safe=true`; no regression in local history. |
| Real embedding history | 3 local `investment_projects` runs; `vectors=221`, `queries=40`, `mean_recall_q16=65535`, `mean_mrr_q16=65535`, `mean_ndcg_q16=65535`, `exact_parity_q16=65535`, `production_safe=true`. |
| ContextPack quality | 25 cases across 5 domains; `evidence_coverage_q16=65535`, `citation_coverage_q16=65535`, `redundancy_reduction_q16=65535`, deterministic ordering coverage. |
| Verification quality | 50 deterministic cases across 5 domains with support, contradiction, mixed evidence, insufficient evidence, numeric guards, and source trust coverage. |
| Single-node performance | Local p95/p99 lifecycle matrix via `make single-node-performance-check`; trend comparison via `make performance-trend-check`. |
| LongMemEval official local run | Official LongMemEval v1 small split: retrieval `session recall_all@10=0.9021`, `session ndcg_any@10=0.7873`; official GPT-4o QA accuracy `0.7660`. |
| MultiHop-RAG local retrieval run | Official MultiHop-RAG retrieval evaluator over full 2556-query dataset: `Hits@10=0.9902`, `Hits@4=0.9295`, `MAP@10=0.4503`, `MRR@10=0.7906`; no leaderboard claim. |

## Evidence Sources

| Evidence | Command | Document |
| --- | --- | --- |
| Beta bundle | `make beta-release-check` | [`BETA_RELEASE.md`](BETA_RELEASE.md) |
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
make retrieval-quality-check
make context-pack-quality-check
make verification-quality-check
make single-node-performance-check
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
