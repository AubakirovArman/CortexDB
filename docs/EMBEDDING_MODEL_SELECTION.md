# Embedding-Model Selection (A2.0)

Boundary: this records an **internal, data-driven model choice** from local
interim diagnostics — **not** a leaderboard result or production claim. The
retrieval numbers are the ERB doc-recall measurements in
[`ERB_EMBEDDING_EVIDENCE.md`](ERB_EMBEDDING_EVIDENCE.md) (local Gemma judge),
which are not comparable to other systems'. See
[`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md).

The chosen profile is recorded in
[`fixtures/embedding/model_selection_v1.json`](../fixtures/embedding/model_selection_v1.json)
and validated by `make embedding-model-selection-check`, so the profile that
A2.1/A2.2 record in the manifest is the actual measured winner rather than an
arbitrary default.

## Candidate matrix

Metric of record: **overall doc recall** (retrieval quality, judge-independent —
two judges agreed within ~1 point; see the evidence doc).

| Candidate | Model | Dim | Metric | Overall doc recall | Index bytes/vector (i16) | Coverage |
| --- | --- | ---: | --- | ---: | ---: | --- |
| lexical-bm25-baseline | — | 0 | none | 55.7 | 0 | EN, no vector index |
| **bge-m3-dense** | **BAAI/bge-m3** | **1024** | **dot_product** | **67.5** | **2048** | **EN + RU, single dense vector** |

## Selection rule

Maximize overall doc recall; on a near-tie (within 0.5) prefer the smaller
per-vector index. The harness (`scripts/embedding_model_eval.py`) recomputes the
winner from the candidate matrix and asserts it equals the recorded `chosen`
profile, so the two cannot drift.

## Chosen profile

**`BAAI/bge-m3`, dimension 1024, metric `dot_product`.**

Rationale:

- Highest overall doc recall (**67.5 vs 55.7**, +11.8 over lexical).
- Multilingual (EN + RU) single dense vector fits the engine's i16 vector index
  at ~2 KB/vector (Q15 i16).
- `dot_product` matches the engine's default metric for normalized bge-m3
  vectors, so it composes with the existing ANN/search path without re-tuning.

Honest caveat (carried from the evidence): dense embeddings do **not** lift the
weak `semantic` question pool (recall flat at ~33%). That pool is a query
decomposition / multi-hop problem (Track A3/A4), **not** a
choose-a-different-embedding-model problem. Selecting bge-m3 is the right dense
choice; it is not a fix for `semantic`.

## Configuring the chosen profile

A deployment records this profile so open-time provenance (A2.1) is enforced:

```bash
CORTEXDB_EMBEDDING_URL=https://<provider>/v1/embeddings
CORTEXDB_EMBEDDING_MODEL=BAAI/bge-m3
```

The dimension (1024) and metric (`dot_product`) are observed from the stored
vectors and recorded in the manifest `EMBD` section at the first checkpoint; see
[`STORAGE_FORMATS.md`](STORAGE_FORMATS.md).

## Reproduction

```bash
make embedding-model-selection-check
# Offline metric math over your own vectors:
python3 scripts/embedding_model_eval.py --live-eval corpus.jsonl queries.jsonl
```
