# Chunking Quality Benchmark

Epic 85 adds a local, repeatable quality gate for deterministic chunking
policies. The benchmark compares candidate text chunk sizes against real-domain
query sets and stores the selected per-domain settings in:

```text
examples/eval/chunking_quality_settings.json
```

Run:

```bash
make chunking-quality-benchmark-check
```

The report is written to:

```text
target/chunking-quality/report.json
```

## What It Measures

The benchmark rebuilds chunks from each domain's `documents.jsonl` using every
candidate `TextChunkPolicy`, ranks chunks with a deterministic local lexical
scorer, and evaluates document-level retrieval against each domain's
`ground_truth.jsonl`.

Metrics:

- `recall_at_k_q16`: mean doc-level recall at the domain's configured top-k.
- `mrr_q16`: mean reciprocal rank for the first relevant document.
- `chunk_count`: number of emitted chunks for the policy.
- `avg_chunk_chars`: average emitted chunk size.

The selected policy must pass the configured thresholds and match the benchmark
recommendation for the current corpus.

## Current Per-Domain Settings

```text
investment_projects: max_chars=1600 overlap_chars=160 min_chars=1
support_tickets:     max_chars=350  overlap_chars=40  min_chars=1
legal_policies:      max_chars=350  overlap_chars=40  min_chars=1
technical_docs:      max_chars=350  overlap_chars=40  min_chars=1
```

## Why This Exists

Chunking is not only a formatting choice. It changes retrieval behavior,
ContextPack citations, and ingestion provenance. This gate keeps policy changes
visible by requiring evidence for retrieval quality before settings are changed.

## Limitations

This is a local lexical benchmark, not an embedding benchmark. It is meant to
catch obvious chunk-size regressions and preserve domain-specific defaults. ANN
and embedding quality are covered by the separate ANN/real-domain gates.
