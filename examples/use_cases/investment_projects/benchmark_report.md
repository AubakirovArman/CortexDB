# Investment Projects Benchmark Report

Status: local beta evidence.

This report connects the Investment Projects use-case pack to the existing
real-domain retrieval and embedding evidence. It is intentionally local-only
for Core Alpha; hosted embedding benchmark promotion is deferred until beta
quota and release cadence are stable.

## Dataset

```text
domain: investment_projects
documents: 56
chunks: 165
queries: 40
ground_truth_rows: 40
fixture: examples/real_domains/investment_projects
```

## Local Real-Embedding Baseline

```text
run_id: investment-projects-v1
model: BAAI/bge-m3
dimension: 1024
vectors: 221
queries: 40
production_safe: true
archive: target/ann/real-embedding/release-baselines/investment-projects-v1.tar.gz
```

## Repeatable Checks

```bash
cd examples/real_domains/investment_projects
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py

make use-case-pack-check
```

Optional local embedding benchmark, when an OpenAI-compatible endpoint is
configured:

```bash
make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1
```

## Boundary

This evidence proves that the pack has a validated corpus, query set, ground
truth rows, runnable CLI smoke flow, and an archived local embedding baseline.
It does not prove investment accuracy, source freshness, or production hosted
embedding availability.
