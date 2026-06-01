# Support Tickets Retrieval Corpus

This corpus is a checked-in real-domain style fixture for CortexDB retrieval
quality gates. It models enterprise support incidents with product, priority,
root cause, workaround, SLA, and tenant metadata.

It is intentionally synthetic and contains no customer data. The goal is to
exercise retrieval behavior on a second domain with realistic operational
language while keeping the beta gate reproducible without external services.

## Files

```text
corpus/documents.jsonl
corpus/chunks.jsonl
queries/queries.jsonl
queries/ground_truth.jsonl
sources/source_registry.json
```

## Validate

```bash
python3 examples/real_domains/support_tickets/scripts/validate_corpus.py
python3 examples/real_domains/support_tickets/scripts/validate_ground_truth.py
```

