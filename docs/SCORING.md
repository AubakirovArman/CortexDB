# Scoring

CortexDB lexical ranking uses canonical BM25 with deterministic fixed-point
arithmetic. Production scoring does not use floating point.

## BM25

Default parameters:

- `k1 = 1.2` (`DEFAULT_BM25_K1_Q16 = 78643`)
- `b = 0.75` (`DEFAULT_BM25_B_Q16 = 49152`)
- score scale: Q16 (`65536`)

For a query term:

```text
idf = ln((N + 1) / (df + 0.5))
tf_norm = tf * (k1 + 1) / (tf + k1 * (1 - b + b * doc_len / avg_doc_len))
score = idf * tf_norm * query_weight * field_weight
```

The implementation keeps the same formula in fixed-point helpers:

- `bm25_idf_q16`
- `bm25_term_score_q16`
- `bm25_term_score_with_idf_q16`
- `Bm25Config`

`crates/cortex-engine/src/search/bm25.rs` contains float-reference tests for the
fixed-point implementation.

## Field Weights

Field-aware lexical scoring computes BM25 per field when field term frequencies
are available, then multiplies by the configured field weight:

| field | weight |
| --- | ---: |
| `title` | 8 |
| `table` | 6 |
| `path` | 5 |
| `entity` | 4 |
| `chunk` | 2 |
| `body` and unknown fields | 1 |

Snapshot search, persisted `.aci` search, AQL candidate ranking, retrieved-cell
ranking, ContextPack `base_bm25`, and the enterprise retrieval benchmark path
all use the shared BM25 helpers. Persisted search computes document frequency
and average field lengths after applying the `AgentView` allowed-candidate set.

## Query Weights

`analyze_search_query` assigns deterministic query weights for literal query
terms, enterprise anchors, quoted phrases, and built-in query expansions. BM25
multiplies the term score by this query weight after IDF, TF saturation, length
normalization, and field weighting.
