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
target/context-pack-quality/prompt-export-report.json
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
`source_trust_bonus`, `source_freshness_q16`,
`source_freshness_category`, `source_freshness_bonus`, and
`redundancy_penalty`. It also proves that excluded candidates expose
`why_excluded` for redundancy control and
`token_budget_tokens` pressure, and that engine structs, server response
structs, OpenAPI, and docs keep the same explain contract.

## Prompt Export Gate

Run:

```bash
make context-pack-prompt-export-check
```

The gate writes:

```text
target/context-pack-quality/prompt-export-report.json
```

It proves that ContextPack has stable JSON, prompt, and Markdown export
formats; that prompt export includes citation instructions and conflict
handling instructions; and that CLI, server, OpenAPI, and docs expose the same
public export formats.

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
  source freshness, redundancy penalty, `why_excluded`, and token-budget exclusion reasons
  present across engine, server, OpenAPI, and docs;
- the prompt export gate keeps JSON, prompt, and Markdown exports available
  and keeps citation/conflict instructions visible to downstream agents;
- the answerability gate keeps `answerability_q16` and the
  `insufficient_context` anomaly visible across engine, JSON contracts, OpenAPI,
  and docs;
- the conflict visibility gate keeps `conflict_visibility_q16` and
  `visible_conflict_count` visible across engine, JSON contracts, OpenAPI, and
  docs;
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
- `cargo test -p cortex-engine --test context_pack_prompt_export`, including
  JSON export, prompt citation instructions, prompt conflict instructions, and
  Markdown citation/source trust output;
- `cargo test -p cortex-cli`, including `cortexdb context --format prompt` and
  `cortexdb context --format markdown`;
- `cargo test -p cortex-server v1_context_returns_prompt_and_markdown_exports`;
- `make openapi-contract-check`, which keeps `/v1/context?format=...` documented
  alongside the typed JSON contract.

## Answerability Evidence

ContextPack answerability is covered by:

- `cargo test -p cortex-engine --test context_pack_answerability`, including
  full coverage, partial coverage, empty context, and export visibility cases;
- `make context-pack-answerability-check`, which writes
  `target/context-pack-quality/answerability-report.json`;
- OpenAPI and typed JSON schemas exposing `answerability_q16` and the
  `insufficient_context` anomaly code.

## Conflict Visibility Evidence

ContextPack conflict visibility is covered by:

- `cargo test -p cortex-engine --test context_pack_conflict_visibility`,
  including no-conflict, one-conflict, multi-conflict, and export visibility
  cases;
- `make context-pack-conflict-visibility-check`, which writes
  `target/context-pack-quality/conflict-visibility-report.json`;
- OpenAPI and typed JSON schemas exposing `conflict_visibility_q16` and
  `visible_conflict_count`.

## Private Scope Leak Evidence

ContextPack private scope leak resistance is covered by:

- `cargo test -p cortex-engine --test context_pack_private_scope`, including a
  broad `WHERE status = "ready"` query that contains both public and forbidden
  ready cells;
- checkpoint/restart and compact/restart checks proving persisted indexes do
  not reintroduce the forbidden scope;
- JSON, prompt, and Markdown export assertions proving private payload, source,
  and scope identifiers are absent from public ContextPack surfaces;
- `make context-pack-private-scope-check`, which writes
  `target/context-pack-quality/private-scope-report.json`.

## Budget Optimizer Evidence

ContextPack budget optimization is covered by
`cargo test -p cortex-engine --test context_pack` cases for:

- required-citation overhead in `estimated_tokens`;
- skipping an oversized middle candidate while keeping later smaller cells;
- applying redundancy reduction before budget overload checks.

## Token Estimator v2 Evidence

ContextPack model-specific token estimation is covered by:

- `cargo test -p cortex-engine --test context_pack_token_estimator`, including
  deterministic default estimates, model-specific multilingual profile
  differences, model-name alias mapping, selected-profile pack accounting, and
  invalid UTF-8 fallback behavior;
- `make context-pack-token-estimator-check`, which writes
  `target/context-pack-quality/token-estimator-report.json`;
- `ContextTokenProfile` variants for Cortex default, GPT-4o-like,
  DeepSeek-chat-like, Gemma-it-like, and BGE-M3-like budgeting.

## Large Cell Policy Evidence

ContextPack large-cell handling is covered by:

- `cargo test -p cortex-engine --test context_pack_large_cell_policy`, including
  truncate, exclude, summarize-placeholder, and source-only reference behavior;
- `make context-pack-large-cell-policy-check`, which writes
  `target/context-pack-quality/large-cell-policy-report.json`;
- `ContextLargeCellPolicy` default compatibility through `PreserveFirst`, plus
  explicit deterministic alternatives for oversized candidates.

## Span-Level Packing Evidence

ContextPack span-level packing is covered by:

- `cargo test -p cortex-engine --test context_pack_span_packing`, including
  coverage lift over prefix truncation under the same token budget and citation
  metadata preservation;
- `cargo test -p cortex-engine --test context_pack`, including default
  compatibility when `span_level_packing` is disabled;
- `make context-pack-span-packing-check`, which writes
  `target/context-pack-quality/span-packing-report.json`;
- `examples/eval/context_pack_span_packing.jsonl`, which records span coverage
  lift and token savings versus prefix truncation;
- deterministic payload markers of the form
  `[context_pack_span=true line_start=... line_end=...]`;
- preserved source/header metadata so citations continue to work after the body
  is reduced to a relevant span;
- structured span provenance in engine, JSON, prompt, Markdown, CLI, server,
  SDK, and OpenAPI surfaces, including source cell id, byte offsets, line
  range, and nested SourceRef metadata when available.
