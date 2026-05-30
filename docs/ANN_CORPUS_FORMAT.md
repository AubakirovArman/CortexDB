# ANN Corpus Format

`ann_corpus_check` is the external-corpus harness for HNSW/ANN recall and
latency validation. It is intentionally file-based so large benchmark corpora do
not need to be committed to this repository.

Use it when you want to answer:

- did ANN return the expected nearest candidates for this corpus?
- did recall drop compared with a known ground-truth set?
- did p95/max latency exceed the configured gate?
- does the graph still report production-safe behavior?

The command accepts three JSONL files:

```bash
cargo run --release -p cortex-engine --bin ann_corpus_check -- \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --ground-truth /data/ann/ground_truth.jsonl \
  --metric cosine \
  --min-recall-q16 49151 \
  --min-mean-recall-q16 49151 \
  --output target/ann/large_corpus_report.json
```

## Files

### vectors.jsonl

Each non-empty, non-comment line is one vector:

```json
{"candidate":1,"vector":[100,0,0,0]}
{"candidate":2,"vector":[96,4,0,0]}
```

Rules:

- `candidate` is a `u32`.
- `candidate = 0` is invalid and reserved.
- candidate ids must be unique.
- `vector` must be a non-empty array of `i16`.
- every vector and query must have the same dimension.

### queries.jsonl

Each line is one named query:

```json
{"name":"axis-a","vector":[100,0,0,0],"limit":2}
{"name":"axis-b","vector":[0,100,0,0],"limit":2}
```

Rules:

- `name` must be non-empty and unique.
- `vector` follows the same dimension rule as corpus vectors.
- `limit` must be greater than zero.

### ground_truth.jsonl

Each line maps a query name to expected top candidates:

```json
{"name":"axis-a","candidates":[1,2]}
{"name":"axis-b","candidates":[3,4]}
```

Rules:

- every query must have a matching ground-truth row.
- `candidates` must be non-empty.
- candidate ids must exist in `vectors.jsonl`.
- candidate id `0` is invalid.
- recall is computed against `candidates[..limit]`.

## Metrics

`--metric` accepts:

- `dot_product`
- `cosine`
- `l2`

The selected metric is used for graph construction and query execution. Ground
truth must be generated with the same metric, otherwise the report will show
false recall regressions.

## Thresholds

Recall is represented as Q16:

| Value | Meaning |
| --- | --- |
| `65535` | 100 percent recall |
| `49151` | roughly 75 percent recall |
| `32767` | roughly 50 percent recall |

Supported gates:

```bash
--min-recall-q16 49151
--min-mean-recall-q16 49151
--max-p95-latency-nanos 100000000
--max-max-latency-nanos 250000000
```

By default, `production_safe` must be true. Use `--allow-unsafe` only for
exploratory runs where you want a report even if the graph violates the gate.

## Report

The output is a single JSON object:

```json
{
  "passed": true,
  "failures": [],
  "metric": "dot_product",
  "vector_count": 12,
  "query_count": 4,
  "dimension": 8,
  "graph_nodes": 12,
  "graph_edges": 120,
  "upper_layers": 3,
  "upper_graph_edges": 2,
  "min_observed_recall_q16": 65535,
  "mean_recall_q16": 65535,
  "p50_latency_nanos": 8548,
  "p95_latency_nanos": 10628,
  "max_latency_nanos": 17382,
  "production_safe": true,
  "queries": [
    {
      "name": "axis-1",
      "limit": 2,
      "truth_count": 2,
      "returned_count": 2,
      "recall_q16": 65535,
      "latency_nanos": 17382,
      "production_safe": true
    }
  ]
}
```

Use `passed=false` as a release blocker for guarded corpora. Query-level rows
help identify whether the regression is broad or concentrated in one cluster.

## CI Smoke Corpus

The repository includes a tiny contract fixture:

- `crates/cortex-engine/fixtures/ann_corpus_vectors_v1.jsonl`
- `crates/cortex-engine/fixtures/ann_corpus_queries_v1.jsonl`
- `crates/cortex-engine/fixtures/ann_corpus_ground_truth_v1.jsonl`

Run:

```bash
make ann-corpus-smoke-check
make ann-corpus-smoke-report
```

This smoke corpus verifies the file contract and report shape. It is not a
production-quality benchmark. Real ANN tuning should use larger external
corpora and archived reports.

## Recommended Workflow

1. Generate exact top-k ground truth offline for the selected metric.
2. Run `ann_corpus_check` in release mode.
3. Archive the output JSON with the commit SHA and machine profile.
4. Compare `min_observed_recall_q16`, `mean_recall_q16`, and p95/max latency
   across commits.
5. Treat recall regressions as correctness issues and latency regressions as SLO
   issues.

## Helper Scripts

The repository includes two standard-library Python helpers under `scripts/ann`.
They are intentionally dependency-free so they can run in CI and on benchmark
machines without creating a Python environment.

### Generate Ground Truth

`exact_ground_truth.py` computes exact top-k candidates from `vectors.jsonl` and
`queries.jsonl` using the same fixed-point metric rules as the Rust engine:

```bash
python3 scripts/ann/exact_ground_truth.py \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --metric cosine \
  --output /data/ann/ground_truth.jsonl
```

Use this when preparing a new corpus. The script sorts by descending score and
then ascending candidate id, matching the engine's deterministic tie-break.

### Compare Reports

`compare_reports.py` compares two `ann_corpus_check` JSON reports:

```bash
python3 scripts/ann/compare_reports.py \
  --baseline reports/main.json \
  --candidate reports/pr.json \
  --max-p95-regression-nanos 5000000 \
  --max-max-regression-nanos 10000000 \
  --output target/ann/ann_report_comparison.json
```

It fails on recall regressions, corpus-shape changes, production-safety loss, or
latency regressions beyond the configured budget.

Run helper self-tests and smoke ground-truth generation with:

```bash
make ann-scripts-check
```
