# Search v1

Core Alpha search helpers are deterministic integer-only building blocks.

## Implemented

- `Bm25Index` lexical ranking with integer scoring.
- Unicode-aware tokenizer for Russian, Kazakh, and English text.
- Basic stopword filtering.
- Field-weighted lexical documents through `add_document_fields`.
- `VectorIndex` exact integer dot-product search.
- `SearchIndexes` public API for keyword, vector, and hybrid modes.
- Hybrid fusion uses reciprocal-rank fusion over lexical and vector results.

## Not Yet

- Persistent vector pages.
- Production BM25 analyzers, stemming, and doc-length tuning.
- HNSW persistence and rebuild policy.
- Reranker integration.
- Search HTTP endpoint.
