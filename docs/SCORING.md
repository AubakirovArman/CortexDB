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

## Score Fusion

Retrieved-cell ranking (`rank_retrieved_cells`) blends four signals per
candidate: lexical (BM25), semantic (fixed-point i16 dot product), recency (from
`created_unix_seconds`), and source trust (`source_trust_q16`). These raw scales
are not comparable (BM25 magnitudes, a large i16 dot product, and two Q16
signals), so each signal is first **min-max normalized to `[0, 65535]` across
the candidate set**, then combined by the mode weights:

```text
score = w_lexical  * norm(lexical)
      + w_semantic * norm(semantic)
      + w_recency  * norm(recency)
      + w_trust    * norm(trust)
score = score * memory_decay_q16 / 65535
```

Normalization (`min_max_normalize_q16`) is what makes the per-mode weights
meaningful. A signal with no spread across candidates maps every candidate to
`65535`, so it adds the same constant to every base score (it does not change
relative order) while keeping the base score non-zero, so the memory-decay
multiplier can still differentiate fresh from stale memory. Weights come from
the `USING MODE` selection (Q16 fractions summing to `65535`):

| mode | lexical | semantic | recency | trust |
| --- | ---: | ---: | ---: | ---: |
| `fast` | 0.55 | 0.10 | 0.25 | 0.10 |
| `balanced` | 0.30 | 0.35 | 0.20 | 0.15 |
| `hybrid` | 0.35 | 0.35 | 0.15 | 0.15 |
| `semantic` | 0.15 | 0.55 | 0.15 | 0.15 |
| `audit` | 0.20 | 0.20 | 0.20 | 0.40 |

Ties are broken deterministically by candidate order. The fusion is guarded by
the `retrieval_recall_eval` regression suite
(`make retrieval-recall-baseline-check`), which asserts recall/MRR floors and
cross-mode ordering (e.g. `audit` prioritizing source trust).
