# Legal Policies Real-Domain Retrieval Corpus

This local fixture models enterprise policy retrieval without claiming legal
advice. It is synthetic but domain-shaped: policy title, jurisdiction, effective
date, source URL, chunks, analyst-style queries, and labelled ground truth.

Run:

```bash
python3 scripts/validate_corpus.py
python3 scripts/validate_ground_truth.py
```

The corpus is used by `make retrieval-quality-check` as one of the beta
multi-domain retrieval fixtures.
