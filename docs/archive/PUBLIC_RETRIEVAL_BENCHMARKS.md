# Public Retrieval Benchmarks

Status: local, reproducible retrieval benchmark snapshot for the checked-in
CortexDB real-domain corpora.

Run:

```bash
make public-retrieval-benchmark-page-check
```

Primary artifacts:

```text
target/retrieval-quality/beta-report.json
target/retrieval-quality/history.json
target/public-retrieval-benchmarks/report.json
```

## Dataset Size

| Domain | Documents | Chunks | Queries | Ground truth rows |
| --- | ---: | ---: | ---: | ---: |
| `investment_projects` | 56 | 165 | 40 | 40 |
| `legal_policies` | 5 | 10 | 5 | 5 |
| `support_tickets` | 10 | 20 | 20 | 20 |
| `technical_docs` | 5 | 10 | 5 | 5 |
| Total | 76 | 205 | 70 | 70 |

## Latest Local Metrics

The gate runs each domain five times and fails on recall, ranking, exact-parity,
or latency regression beyond the configured local tolerance.

| Domain | Runs | Recall q16 | MRR q16 | nDCG q16 | Latency fields |
| --- | ---: | ---: | ---: | ---: | --- |
| `investment_projects` | 5 | 62258 | 49767 | 37590 | p95/p99/max in `history.json` |
| `legal_policies` | 5 | 65535 | 65535 | 65535 | p95/p99/max in `history.json` |
| `support_tickets` | 5 | 65535 | 65535 | 65535 | p95/p99/max in `history.json` |
| `technical_docs` | 5 | 65535 | 65535 | 65535 | p95/p99/max in `history.json` |

Summary:

```text
domain_count: 4
history_runs_per_domain: 5
run_count: 20
regression_count: 0
production_safe: true
```

## Exact Vs ANN

This public retrieval page separates correctness evidence from experimental ANN
evidence:

| Mode | What it proves | Public boundary |
| --- | --- | --- |
| Deterministic lexical fixture | Checked-in domains can be searched repeatedly with stable recall, MRR, nDCG, p95 latency, p99 latency, and exact top-k parity reports. | Local fixture evidence only; not private customer relevance. |
| Exact vector fallback | Vector search can use exact scan as the correctness fallback for persisted vectors. | Exact scan is not an ANN speed claim. |
| Guarded ANN / HNSW | ANN reports track recall, ranking, exact parity, graph shape, p95 latency, p99 latency, fallback decisions, and `production_safe`. | HNSW remains guarded; this page does not claim fallback-free production HNSW. |
| Endpoint-backed real embeddings | Local `investment_projects` history exists for `BAAI/bge-m3` real embeddings. | Hosted GitHub Actions embedding runs are deferred until beta quota and cadence are finalized. |

## Limitations

This page does not claim:

- production SLA latency;
- hosted embedding CI execution;
- quality on private customer corpora;
- fallback-free production HNSW;
- leaderboard placement;
- legal-grade or financial-grade answer correctness;
- managed cloud readiness.

## Refresh Flow

```bash
make retrieval-quality-history-check
make public-retrieval-benchmark-page-check
```

Use [`RETRIEVAL_QUALITY_EVIDENCE.md`](RETRIEVAL_QUALITY_EVIDENCE.md) for the
long-form evidence notes and [`PUBLIC_BENCHMARKS.md`](PUBLIC_BENCHMARKS.md) for
the release-by-release benchmark summary.
