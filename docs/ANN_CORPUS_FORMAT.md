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
  "hnsw_max_neighbors": 8,
  "hnsw_ef_search": 64,
  "hnsw_layer_count": 4,
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

It also includes a domain-like CortexDB fixture:

- `crates/cortex-engine/fixtures/ann_domain_vectors_v1.jsonl`
- `crates/cortex-engine/fixtures/ann_domain_queries_v1.jsonl`
- `crates/cortex-engine/fixtures/ann_domain_ground_truth_v1.jsonl`

The domain fixture uses `dot_product` vectors shaped like investment-project,
legal-risk, operations-error, and agent-memory cells. It is still small enough
for normal CI, but it catches regressions that a purely geometric smoke corpus
can miss.

Run:

```bash
make ann-corpus-smoke-check
make ann-corpus-smoke-report
make ann-domain-corpus-check
make ann-domain-corpus-report
make ann-demo-domain-corpus-run
```

This smoke corpus verifies the file contract and report shape. It is not a
production-quality benchmark. Real ANN tuning should use larger external
corpora and archived reports.

`ann-demo-domain-corpus-run` is a larger local demo gate. It does not commit the
generated corpus. Instead, it scans the checked-in example payloads, writes
`vectors.jsonl`, `queries.jsonl`, and `documents.jsonl` to
`target/ann/demo-domain-corpus/converted/`, generates exact ground truth, then
archives the report and machine profile under
`target/ann/demo-domain-corpus/runs/<run-id>/`.
Use `make ann-demo-domain-package-baseline` to turn the latest demo-domain run
into a release-ready baseline archive. That archive contains the report,
manifest, machine profile, exact ground truth, and checksum manifest needed for
future regression comparisons.

For rows that already contain vectors, use
`make ann-embedded-domain-corpus-run`. It expects JSONL payload rows that
already contain vectors and a JSONL query file whose queries also contain
vectors. Unlike the demo-domain builder, this path does not synthesize vectors
from text.

For rows that only contain payload/query text, use
`make ann-embedding-domain-corpus-run`. It calls an embedding command, exports
fixed-point vectors, then runs the same embedded-domain corpus gate.

## Recommended Workflow

1. Generate exact top-k ground truth offline for the selected metric.
2. Run `ann_corpus_check` in release mode.
3. Archive the output JSON with the commit SHA and machine profile.
4. Compare `min_observed_recall_q16`, `mean_recall_q16`, and p95/max latency
   across commits.
5. Treat recall regressions as correctness issues and latency regressions as SLO
   issues.

## Helper Scripts

The repository includes standard-library Python helpers under `scripts/ann`.
They are intentionally dependency-free so they can run in CI and on benchmark
machines without creating a Python environment.

### Convert Public Corpora

`convert_public_corpus.py` converts common public ANN benchmark files into the
CortexDB JSONL contract:

```bash
python3 scripts/ann/convert_public_corpus.py \
  --vectors-fvecs /data/sift/sift_base.fvecs \
  --queries-fvecs /data/sift/sift_query.fvecs \
  --ground-truth-ivecs /data/sift/sift_groundtruth.ivecs \
  --output-dir /data/sift/cortexdb-ann \
  --normalization unit \
  --limit 10
```

It writes `vectors.jsonl`, `queries.jsonl`, `ground_truth.jsonl`, and
`conversion_manifest.json`. The converter also accepts GloVe/word2vec-style
text rows:

```bash
python3 scripts/ann/convert_public_corpus.py \
  --vectors-text /data/glove/vectors.txt \
  --queries-text /data/glove/queries.txt \
  --output-dir /data/glove/cortexdb-ann \
  --normalization unit \
  --limit 10
```

Supported input formats:

- `fvecs` float vectors for base/query vectors;
- `ivecs` integer nearest-neighbor ids for ground truth;
- whitespace text rows with either `label value...` or `value...`.

The converter quantizes values to signed `i16` because the current ANN corpus
contract is fixed-point. Use `--normalization unit` for cosine-like public
embeddings, `--normalization max_abs` for preserving vector shape under large
coordinate ranges, or `--normalization none` when the source values are already
scaled.

### Build A Demo-Domain Corpus

`build_demo_domain_corpus.py` turns CortexDB example JSONL payloads into the
same ANN corpus contract:

```bash
python3 scripts/ann/build_demo_domain_corpus.py \
  --source-root examples/datasets \
  --source-root examples/rag_demo/data \
  --output-dir target/ann/demo-domain-corpus/converted \
  --dimension 64 \
  --limit 5
```

It uses deterministic hashed sparse vectors. This is not a replacement for real
embeddings, but it gives the HNSW gate a repeatable product-shaped corpus with
finance, legal, HR, support, SEC, and world-indicator payloads. The emitted
`documents.jsonl` maps generated candidate ids back to source files and lines,
which makes recall failures easier to inspect.

### Build A Real-Embedding Domain Corpus

`export_embedding_domain_corpus.py` is the bridge from text rows to a
real-embedding ANN corpus. The embedding backend is intentionally external: the
script sends raw UTF-8 text to a command on stdin and expects a JSON numeric
array on stdout. This keeps model clients, API keys, and vendor-specific code
out of the repository.

Example:

```bash
make ann-embedding-domain-corpus-run \
  ANN_EMBEDDING_SOURCE_ROOT=/data/cortexdb/exported-text-cells \
  ANN_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_EMBEDDING_COMMAND='python3 tools/embed_text.py' \
  ANN_EMBEDDING_METRIC=cosine \
  ANN_EMBEDDING_NORMALIZATION=unit
```

The repository also includes a dependency-free OpenAI-compatible command
wrapper:

```bash
export CORTEXDB_EMBEDDING_URL='https://embedding-gateway.example/v1/embeddings'
export CORTEXDB_EMBEDDING_MODEL='text-embedding-model'
export CORTEXDB_EMBEDDING_API_KEY='...'

make ann-embedding-domain-corpus-run \
  ANN_EMBEDDING_SOURCE_ROOT=/data/cortexdb/exported-text-cells \
  ANN_EMBEDDING_QUERIES=/data/cortexdb/query_text.jsonl \
  ANN_EMBEDDING_COMMAND='python3 scripts/ann/embed_text_command.py --require-model' \
  ANN_EMBEDDING_METRIC=cosine
```

The command contract is:

```text
stdin:  one payload or query text, UTF-8
stdout: JSON array of numeric vector coordinates
stderr: diagnostics only
```

`embed_text_command.py` reads `CORTEXDB_EMBEDDING_URL`,
`CORTEXDB_EMBEDDING_MODEL`, and `CORTEXDB_EMBEDDING_API_KEY` by default. It
accepts OpenAI-compatible `{"data":[{"embedding":[...]}]}` responses and also
simple `{"embedding":[...]}`, `{"vector":[...]}`, or
`{"embeddings":[[...]]}` responses. Use environment variables or a local config
file outside the repository for provider keys. Do not hardcode API keys in the
corpus source or embedding command.

The exported files are written under `ANN_EMBEDDING_OUTPUT_DIR`:

```text
payloads/cells.jsonl
queries.jsonl
manifest.json
```

Keep `ANN_EMBEDDING_OUTPUT_DIR` outside the source directory being scanned.
Generated benchmark output is skipped when it lives under common build folders
like `target/`, but production runs should keep raw source and generated
reports separate.

`--provider hash-smoke` exists only for script self-tests and local plumbing
checks. Production recall/latency reports should use `--provider command` with
the exact embedding client used by the product path.

For real production-style runs, prefer `make ann-real-embedding-preflight`
followed by `make ann-real-embedding-benchmark`. The preflight layer checks the
source/query JSONL contract and required environment variables before any
embedding API calls are made, then the benchmark target delegates to the same
export and HNSW report pipeline documented here.
After a real baseline report exists, run `make ann-real-embedding-compare` or
`make ann-real-embedding-benchmark-and-compare` so release candidates fail on
recall, graph-shape, production-safety, HNSW parameter, or latency regressions.
Set `ANN_REAL_EMBEDDING_SLO_PROFILE` to `fast`, `balanced`, `semantic`, or
`audit` to make the selected recall, latency, and HNSW graph policy explicit in
the run command.

`build_embedded_domain_corpus.py` is the path for data that already carries
model-generated embeddings. Payload rows may include either a top-level
`vector` field or a payload metadata line like `vector=10,0,-3`. Queries are a
separate JSONL file with `name`, `vector`, and optional `limit`:

```json
{"name":"budget-query","vector":[120,0,-8],"limit":10}
```

Run it through Make:

```bash
make ann-embedded-domain-corpus-run \
  ANN_EMBEDDED_DOMAIN_SOURCE_ROOT=/data/cortexdb/exported-cells \
  ANN_EMBEDDED_DOMAIN_QUERIES=/data/cortexdb/query_vectors.jsonl \
  ANN_EMBEDDED_DOMAIN_METRIC=cosine
```

This mode fails closed when a document row is missing a vector. Use
`build_embedded_domain_corpus.py --skip-missing-vectors` only for exploratory
audits where the skipped count is reviewed in `manifest.json`.

### Run A Public Corpus End To End

`run_public_corpus.py` wraps download/extract, conversion, ANN evaluation, and
run-manifest writing:

```bash
python3 scripts/ann/run_public_corpus.py \
  --source-url ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz \
  --dataset-id siftsmall \
  --format fvecs \
  --metric l2 \
  --normalization none \
  --scale 1 \
  --max-neighbors 16 \
  --ef-search 256 \
  --run-id siftsmall-l2
```

The source may also be a local archive or an already extracted directory:

```bash
make ann-public-corpus-run \
  ANN_PUBLIC_SOURCE=/data/ann/siftsmall.tar.gz \
  ANN_PUBLIC_DATASET_ID=siftsmall \
  ANN_PUBLIC_FORMAT=fvecs \
  ANN_PUBLIC_METRIC=l2 \
  ANN_PUBLIC_NORMALIZATION=none \
  ANN_PUBLIC_SCALE=1 \
  ANN_PUBLIC_MAX_NEIGHBORS=16 \
  ANN_PUBLIC_EF_SEARCH=256
```

Use the metric and scaling that match the source corpus ground truth. For
SIFT/TEXMEX-style `ivecs` ground truth, that usually means L2 over raw SIFT
coordinates, so `--metric l2 --normalization none --scale 1` is the safer
default. Cosine-style embedding corpora usually need `--metric cosine` and
`--normalization unit`.

The script writes:

- `target/ann/public-corpora/<dataset-id>/converted/*.jsonl`;
- `target/ann/public-corpora/<dataset-id>/public_corpus_manifest.json`;
- `target/ann/corpus-runs/<run-id>/report.json`;
- `target/ann/corpus-runs/history.json`.

Use `--no-run` when you only want to materialize the converted JSONL contract.
Use `--max-vectors` and `--max-queries` for a fast sample run before launching a
full corpus benchmark.

Use `--max-neighbors`, `--ef-search`, and `--layer-count` to tune the graph
without changing the corpus files. The generated report records these values so
baseline comparisons do not hide HNSW parameter changes.

The same path is available as the manual `ANN Public Corpus` GitHub Actions
workflow. Use it for public archives that should be evaluated on hosted CI and
kept as downloadable artifacts instead of only local `target/ann` files.

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

### Summarize Report History

`summarize_history.py` scans archived run directories and writes a compact
history report:

```bash
python3 scripts/ann/summarize_history.py \
  --run-root target/ann/corpus-runs \
  --output target/ann/corpus-runs/history.json
```

The summary includes every `run_id`, commit SHA, corpus shape, recall,
latency, graph-shape fields, and adjacent regressions for each corpus key. Use
`--fail-on-regression` when you want the history pass to fail on any detected
recall, latency, graph-shape, or production-safety regression.

### Publish A Baseline Bundle

`publish_baseline.py` copies one archived run into a release-ready baseline
bundle:

```bash
python3 scripts/ann/publish_baseline.py \
  --run-root target/ann/corpus-runs \
  --run-id smoke \
  --baseline-id v0.1.0-core-alpha-smoke \
  --output-root target/ann/release-baselines
```

The bundle contains `baseline_manifest.json`, the selected run manifest,
`report.json`, `machine_profile.json`, `history.json`, and generated ground
truth when it exists. Use this for release artifacts or for pinning a known-good
external corpus report before comparing later candidate runs.

Package the bundle as a GitHub Release-ready artifact:

```bash
make ann-package-baseline \
  ANN_BASELINE_ID=v0.1.0-core-alpha-smoke \
  ANN_BASELINE_ARCHIVE=target/ann/release-baselines/v0.1.0-core-alpha-smoke.tar.gz
```

The archive includes `package_manifest.json` with SHA-256 checksums for every
file inside the bundle. GitHub Actions creates the same tarball on the stable
toolchain and uploads it as the `ann-release-baseline-package` artifact.

Run helper self-tests and smoke ground-truth generation with:

```bash
make ann-scripts-check
```

### One-Command Corpus Runs

`run_external_corpus.sh` ties the full workflow together:

```bash
scripts/ann/run_external_corpus.sh \
  --vectors /data/ann/vectors.jsonl \
  --queries /data/ann/queries.jsonl \
  --metric cosine \
  --baseline-report reports/main.json \
  --output-root target/ann/corpus-runs
```

The script creates `target/ann/corpus-runs/<run-id>/` and writes:

- `ground_truth.jsonl` if `--ground-truth` was omitted;
- `machine_profile.json` with OS, CPU, memory, Rust, and Cargo versions;
- `manifest.json` with the run id, commit SHA, metric, input paths, and report
  path;
- `report.json` from `ann_corpus_check`;
- `comparison.json` when `--baseline-report` is provided.
- `../history.json` summarizing all archived runs under the output root.

Run the built-in smoke workflow with:

```bash
make ann-corpus-run-smoke
```

You can rebuild the history file without rerunning ANN evaluation:

```bash
make ann-history-report
```

You can publish the latest smoke run as a baseline artifact:

```bash
make ann-publish-baseline
```

You can compare a candidate run against a published baseline bundle:

```bash
make ann-compare-baseline-bundle \
  ANN_BASELINE_BUNDLE=target/ann/release-baselines/smoke \
  ANN_CANDIDATE_RUN_ID=smoke \
  ANN_MAX_P95_REGRESSION_NANOS=5000000 \
  ANN_MAX_MAX_REGRESSION_NANOS=10000000
```

The comparison is written to
`target/ann/corpus-runs/<candidate-run-id>/baseline_comparison.json`.
The comparison also fails when `hnsw_max_neighbors`, `hnsw_ef_search`, or
`hnsw_layer_count` changes, because recall and latency are only comparable under
the same graph/search profile.
Latency baselines should also be interpreted with `machine_profile.json`; p95
from different CPU/OS/Rust environments is useful as signal, but not as a strict
apples-to-apples regression unless the machine profile matches.
