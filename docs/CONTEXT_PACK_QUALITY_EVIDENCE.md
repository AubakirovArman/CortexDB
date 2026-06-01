# ContextPack Quality Evidence

Last local ContextPack quality run: 2026-06-01, passed.

Run:

```bash
make context-pack-quality-check
```

Primary artifacts:

```text
examples/eval/context_pack_quality.jsonl
target/context-pack-quality/report.json
```

## Latest Local Metrics

```text
case_count: 25
domain_count: 5
domains: investment_projects, legal_policies, support_tickets, technical_docs, world_indicators
evidence_coverage_q16: 65535
token_reduction_q16: 36915
context_pack_token_savings_vs_classic_q16: 36915
context_pack_cell_reduction_vs_classic_q16: 31881
classic_rag_chunks: 148
classic_rag_duplicate_chunks: 45
classic_rag_duplicate_rate_q16: 19926
citation_coverage_q16: 65535
redundancy_reduction_q16: 65535
anomaly_coverage_q16: 65535
deterministic_order_q16: 65535
```

## Boundary

This gate proves:

- ContextPack behavior tests pass for budget truncation, required citations,
  source refs, sparse/dense duplicate suppression, deterministic ordering, and
  explain fields;
- the real-domain investment-project fixture produces cited selected cells;
- support-ticket, legal-policy, and world-indicator fixtures prove the metric
  gate is no longer limited to one domain;
- the technical-docs domain covers API contracts, storage runbooks, versioned
  docs, SDK quickstarts, and security configuration context;
- ContextPack output is measured against classic raw chunk retrieval for token
  savings, cell reduction, duplicate pressure, and anomaly coverage;
- the report includes per-domain metrics under `per_domain_metrics`;
- the quality fixture records measurable evidence coverage, token reduction,
  citation coverage, redundancy reduction, anomaly coverage, and deterministic
  ordering.

This gate does not prove:

- answer quality from an external LLM;
- dense semantic reranking quality beyond the deterministic local vector
  redundancy checks;
- private customer-domain evidence quality.
