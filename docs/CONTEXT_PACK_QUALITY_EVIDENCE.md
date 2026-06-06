# ContextPack Quality Evidence

Last local ContextPack quality run: 2026-06-06, passed.

Run:

```bash
make context-pack-quality-check
```

Primary artifacts:

```text
examples/eval/context_pack_quality.jsonl
target/context-pack-quality/report.json
fixtures/context_pack_quality_v3_datasets.json
fixtures/context_pack_quality_v3_thresholds.json
target/context-pack-quality/v3-report.json
target/context-pack-quality/explain-v2-report.json
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

## V3 Quality Gate

Run:

```bash
make context-pack-quality-v3-check
```

Latest local v3 metrics:

```text
case_count: 105
external_dataset_count: 4
external_domains: investment_projects, legal_policies, support_tickets, technical_docs
failure_category_count: 5
failure_categories: anomaly_pressure, citation_pressure, evidence_selection, redundancy_pressure, token_budget_pressure
evidence_coverage_q16: 65535
citation_coverage_q16: 65535
token_reduction_q16: 37126
redundancy_reduction_q16: 65535
anomaly_coverage_q16: 65535
deterministic_order_q16: 65535
```

Per-domain v3 thresholds are checked from:

```text
fixtures/context_pack_quality_v3_thresholds.json
```

The v3 dataset descriptor is checked from:

```text
fixtures/context_pack_quality_v3_datasets.json
```

## Explain V2 Gate

Run:

```bash
make context-pack-explain-v2-check
```

The gate writes:

```text
target/context-pack-quality/explain-v2-report.json
```

It proves that selected cells expose `why_selected`, structured
`score_components`, `source_trust_q16`, `source_trust_category`,
`source_trust_bonus`, and `redundancy_penalty`. It also proves that excluded
candidates expose `why_excluded` for redundancy control and
`token_budget_tokens` pressure, and that engine structs, server response
structs, OpenAPI, and docs keep the same explain contract.

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
- the quality gate requires at least 25 cases across at least 4 domains;
- the v3 quality gate requires at least 100 expanded cases, at least 4
  external real-domain datasets, at least 5 failure categories, and per-domain
  thresholds;
- the Explain v2 gate keeps `why_selected`, score components, source trust,
  redundancy penalty, `why_excluded`, and token-budget exclusion reasons
  present across engine, server, OpenAPI, and docs;
- the quality fixture records measurable evidence coverage, token reduction,
  citation coverage, redundancy reduction, anomaly coverage, and deterministic
  ordering.

This gate does not prove:

- answer quality from an external LLM;
- dense semantic reranking quality beyond the deterministic local vector
  redundancy checks;
- private customer-domain evidence quality.

## Export Evidence

Stable ContextPack prompt and Markdown export modes are covered by:

- `cargo test -p cortex-engine --test context_pack`, including prompt/Markdown
  formatting and Markdown fence preservation;
- `cargo test -p cortex-cli`, including `cortexdb context --format prompt` and
  `cortexdb context --format markdown`;
- `cargo test -p cortex-server v1_context_returns_prompt_and_markdown_exports`;
- `make openapi-contract-check`, which keeps `/v1/context?format=...` documented
  alongside the typed JSON contract.

## Budget Optimizer Evidence

ContextPack budget optimization is covered by
`cargo test -p cortex-engine --test context_pack` cases for:

- required-citation overhead in `estimated_tokens`;
- skipping an oversized middle candidate while keeping later smaller cells;
- applying redundancy reduction before budget overload checks.
