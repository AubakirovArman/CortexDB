# Retrieval Quality Evidence

Last local retrieval-quality run: 2026-06-01, passed.

Run:

```bash
make retrieval-quality-check
```

Primary artifacts:

```text
target/retrieval-quality/report.json
target/ann/real-embedding/runs/history.json
target/ann/real-embedding/investment_projects_readiness_epic25.json
target/ann/real-embedding/runs/<run-id>/report.json
```

## Latest Local History

```text
latest_run_id: investment-projects-v2-metrics
run_count: 3
corpus_count: 1
regression_count: 0
vectors: 221
queries: 40
mean_recall_q16: 65535
mean_mrr_q16: 65535
mean_ndcg_q16: 65535
exact_parity_q16: 65535
p95_latency_nanos: 5271976
production_safe: true
modes: lexical, vector, hybrid, guarded_ann
query_level_rows: 40
```

## Boundary

This gate proves:

- the checked-in `investment_projects` corpus, queries, and ground truth are
  structurally valid;
- real-domain embedding readiness passed with `BAAI/bge-m3` against the local
  OpenAI-compatible endpoint configured in `.env`;
- real-domain embedding history has at least three local runs for the same corpus
  group;
- recall, MRR, nDCG, exact parity, graph shape, latency, and
  `production_safe=true` remain visible in machine-readable reports;
- lexical, vector, hybrid, and guarded ANN evidence are tracked separately;
- query-level guarded ANN rows are available for review;
- the local history has no adjacent regression under the configured latency
  tolerance.

This gate does not prove:

- hosted GitHub Actions embedding execution, which is deferred to beta;
- quality on private customer corpora;
- production relevance judgments beyond the checked-in investment-project
  ground truth.
