# Technical Docs Real-Domain Retrieval Corpus

This local fixture models developer documentation retrieval for CortexDB-style
systems: API contracts, storage validation, SDK release gates, search explain,
and security configuration.

Run:

```bash
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py
```

The corpus is intentionally synthetic and small, but it has real documentation
shape: documents, chunks, queries, ground truth, and source registry.
