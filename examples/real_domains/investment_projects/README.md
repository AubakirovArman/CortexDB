# Investment Projects Real-Domain Corpus

This fixture closes the first local version of the CortexDB
`Real-domain embedding promotion` milestone.

Domain:

```text
investment_projects
region: Kazakhstan / Central Asia
language: English first
```

The corpus uses public project metadata and short generated benchmark notes.
It intentionally does not copy full copyrighted news articles or large source
documents into the repository.

## Files

```text
corpus/documents.jsonl
corpus/chunks.jsonl
queries/queries.jsonl
queries/ground_truth.jsonl
sources/source_registry.json
scripts/build_corpus.py
scripts/build_chunks.py
scripts/build_queries.py
scripts/validate_corpus.py
scripts/validate_ground_truth.py
.env.example
```

## Schemas

`documents.jsonl` rows:

```json
{"doc_id":"wb_p500565","source":"world_bank","country":"Kazakhstan","title":"...","sector":"...","url":"...","text":"..."}
```

`chunks.jsonl` rows:

```json
{"chunk_id":"wb_p500565_c001","doc_id":"wb_p500565","source":"world_bank","country":"Kazakhstan","sector":"Transport","title":"...","text":"..."}
```

`queries.jsonl` rows:

```json
{"query_id":"q001","name":"q001","query":"Kazakhstan airport infrastructure financing project","text":"Kazakhstan airport infrastructure financing project","intent":"find_project_by_sector_country","limit":5}
```

`ground_truth.jsonl` rows:

```json
{"query_id":"q001","name":"q001","relevant_doc_ids":["edb_almaty_airport_001"],"relevant_chunk_ids":["edb_almaty_airport_001_c001"]}
```

## Validate

From this directory:

```bash
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py
```

From the repository root:

```bash
make ann-real-embedding-readiness \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl
```

The readiness target requires:

```bash
CORTEXDB_EMBEDDING_URL
CORTEXDB_EMBEDDING_MODEL
```

`CORTEXDB_EMBEDDING_API_KEY` is needed only when the endpoint requires it.

## Benchmark

With a real OpenAI-compatible embedding endpoint configured:

```bash
make ann-real-embedding-benchmark \
  ANN_REAL_EMBEDDING_SOURCE_ROOT=examples/real_domains/investment_projects/corpus \
  ANN_REAL_EMBEDDING_QUERIES=examples/real_domains/investment_projects/queries/queries.jsonl \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1
```

After a successful benchmark:

```bash
make ann-real-embedding-publish-baseline \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1

make ann-real-embedding-package-baseline \
  ANN_REAL_EMBEDDING_RUN_ID=investment-projects-v1
```

Current local baseline:

```text
run_id: investment-projects-v1
model: BAAI/bge-m3
dimension: 1024
vectors: 221
queries: 40
production_safe: true
archive: target/ann/real-embedding/release-baselines/investment-projects-v1.tar.gz
```

## Regenerate

`build_corpus.py` fetches current Kazakhstan project metadata from the World
Bank project API and combines it with a small hand-curated metadata set for
EDB/EBRD/Reuters-reference examples.

```bash
python3 scripts/build_corpus.py
```

The Reuters rows are metadata-only reference rows for query coverage; the news
article bodies are not copied into this repository.
