# Context Pack v1

Context Pack is the first agent-ready retrieval surface above raw `Retrieve`.
The v1 contract keeps the current core deterministic while making the JSON
surface stable enough for SDK and UI consumers:

For the broader technology overview, see
[`CONTEXT_PACK_TECHNOLOGY.md`](archive/CONTEXT_PACK_TECHNOLOGY.md).

```text
AQL RETRIEVE CONTEXT
-> Engine AQL retrieve
-> ordered candidate cells
-> deterministic token estimate
-> budgeted ContextPack
-> citation anomaly report
```

## Current Scope

Implemented in `cortex-engine`:

- `ContextPackOptions`
- `ContextPack`
- `ContextPackCell`
- `ContextPackAnomaly`
- `Database::context_pack_from_aql`
- `estimate_tokens`
- `estimate_tokens_for_profile`
- `ContextTokenProfile`
- `ContextLargeCellPolicy`
- Optional sparse redundancy reduction using fixed-point Jaccard.
- Optional dense-vector redundancy reduction when payloads include `vector=`.
- Numeric guard coexistence: conflicting values for the same
  `project` + `metric` are kept together instead of deduplicated.
- Citation-aware token accounting: when citations are required and a selected
  cell has a citation/source, the deterministic estimate includes fixed
  citation overhead.
- Model-specific token profiles: `ContextTokenProfile` supports Cortex default,
  GPT-4o-like, DeepSeek-chat-like, Gemma-it-like, and BGE-M3-like deterministic
  budget estimates without calling external tokenizers.
- Large-cell policy: oversized cells can preserve the legacy first-cell
  behavior, truncate to fit, exclude, emit a deterministic summarize-placeholder,
  or emit a source-only reference.
- Span-level packing: when explicitly enabled through
  `ContextPackOptions.span_level_packing`, an oversized first candidate can be
  reduced to the query-relevant body span instead of preserving the full leading
  document text. The selector is deterministic, uses only query terms and cell
  text, preserves source metadata above the selected span, and records explicit
  span provenance with the source cell id, byte range, line range, and optional
  structured SourceRef.
- Value-per-token planning: when explicitly enabled through
  `ContextPackOptions.optimize_value_per_token`, ContextPack reorders the
  already retrieved candidate set by expected answerability value per token
  before budget packing. The deterministic cost model uses marginal query-term
  coverage, ContextPack BM25, source trust, source freshness, citation
  availability, feedback, redundancy, and configured token cost. See
  [`LLM_CONTEXT_VALUE_OPTIMIZATION.md`](LLM_CONTEXT_VALUE_OPTIMIZATION.md).
- Dedup-aware budget packing: redundant candidates are filtered before budget
  overload checks, and oversized middle candidates are skipped so smaller later
  candidates can still fit.
- Source trust categories: explain metadata includes `source_trust_q16` and
  `source_trust_category` (`unknown`, `low`, `medium`, `high`, `official`) so
  UI/SDK consumers can see provenance contribution without reinterpreting q16.
- Source freshness categories: explain metadata includes
  `source_freshness_q16` and `source_freshness_category` (`unknown`, `stale`,
  `older`, `recent`, `current`) derived from `created_unix_seconds` relative to
  the retrieved candidate set. It is deterministic and does not call wall-clock
  time during packing.
- Answerability score: `answerability_q16` estimates whether selected cells
  cover explicit query terms. If coverage is incomplete, ContextPack emits an
  `insufficient_context` anomaly instead of implying that an answer is safe.
- Conflict visibility score: `conflict_visibility_q16` and
  `visible_conflict_count` report whether selected cells contain visible
  conflicting `project` + `metric` values that survived packing.
- Answer grounding guard: `ContextPack::ground_answer` checks a generated answer
  against the selected pack cells, splits the answer into spans, reports
  `supported`/`unsupported` spans, matched/missing terms, supporting `cell_id`s,
  citations, and an optional `rejected` flag when callers enable
  `reject_unsupported`. This is a post-answer guard; it does not read benchmark
  gold labels or judge answers with an LLM.
- Access-decision trail: every AQL-built ContextPack cell records the readable
  scope decision that allowed it into the pack (`cell_id`, `scope`, `scope_id`,
  `agent_id`, `decision`, `policy`, and `reason`). HTTP JSON responses attach
  `principal_id` and `auth_role` from the authenticated request when present, so
  enterprise audit tooling can answer why a user saw a specific fact without
  exposing bearer tokens.

Public JSON responses include:

```json
{
  "schema_version": "context_pack.v1",
  "token_budget_tokens": 1000,
  "estimated_tokens": 42,
  "truncated": false,
  "citations_required": false,
  "answerability_q16": 0,
  "conflict_visibility_q16": 0,
  "visible_conflict_count": 0,
  "cells": [],
  "anomalies": [
    {
      "cell_id": null,
      "code": "insufficient_context",
      "message": "context answerability score 0/65535 is below the required threshold",
      "why_excluded": "covered_terms=[]; missing_terms=[budget]"
    }
  ]
}
```

The frozen JSON Schema lives at
[`docs/schemas/context_pack.v1.json`](schemas/context_pack.v1.json). The
contract is guarded by `make context-pack-schema-contract-check`, which validates
the server snapshot against the schema and checks that OpenAPI and Rust SDK v1
types stay aligned with the same required fields. Until `context_pack.v2`, v1 is
additive-only: existing required fields, enum meanings, and `schema_version`
cannot be removed or renamed; new optional fields may be added only when schema,
OpenAPI, SDK, snapshots, and docs move together.

For selected cells, `cells[].access_decision` is the per-cell RBAC trail. A
typical HTTP response includes:

```json
{
  "cell_id": 1,
  "decision": "allowed",
  "policy": "agent_view_readable_scope",
  "reason": "cell scope was present in AgentView.readable_scopes before ContextPack packing",
  "scope": "project:investments",
  "scope_id": 1001,
  "agent_id": 7,
  "principal_id": "agent-a",
  "auth_role": "data"
}
```

`schema_version` is required. `cells[].access_decision.decision = "not_recorded"`
is part of v1 only for manually constructed packs that did not pass through an
AgentView-backed AQL retrieval. AQL-built packs must record the readable-scope
decision trail. Future incompatible ContextPack response changes must introduce
a new schema version and update the JSON Schema, OpenAPI, SDKs, snapshots, and
API changelog entries together.

The CLI exposes:

```bash
cargo run -p cortex-cli -- context ./data project:investments '<AQL>'
cargo run -p cortex-cli -- context ./data project:investments '<AQL>' --format prompt
cargo run -p cortex-cli -- context ./data project:investments '<AQL>' --format markdown
```

The HTTP server exposes:

```http
POST /v1/context?scope=project:investments
POST /v1/context?scope=project:investments&format=prompt
POST /v1/context?scope=project:investments&format=markdown

RETRIEVE CONTEXT ...
```

The Rust SDK exposes:

```rust
let json = client.context_response("project:investments", aql)?;
let prompt = client.context_prompt("project:investments", aql)?;
let markdown = client.context_markdown("project:investments", aql)?;
```

## Agent Exports

ContextPack v1 has two stable non-JSON export modes:

- `prompt` - plain text intended to be appended or injected into an agent
  prompt. It starts with `CortexDB ContextPack v1`, includes usage instructions,
  budget metadata, selected cells, citations/source refs, and anomalies.
- `markdown` - human-readable audit/report format with the same selected cells
  and anomalies, using deterministic headings and code blocks.

Both modes are generated by `cortex-engine`, so CLI and server output share the
same formatter. JSON remains the default HTTP response, while CLI keeps the
existing compact summary default unless `--json` or `--format` is passed.

## Invariants

1. Context packing never bypasses AQL policy, binder, bitmap VM, or AgentView.
2. Ordering follows the retrieve candidate order unless the caller explicitly
   enables `ContextPackOptions.optimize_value_per_token`.
3. Token estimates are deterministic integer estimates and include citation
   overhead when citations are required.
4. Requested budget is clamped by `AgentView::effective_budget`.
5. Citation requirements produce anomalies instead of silently passing.
6. Redundancy reduction, when enabled, reports skipped cells as anomalies before
   budget overload checks.
7. Numeric guard conflicts are preserved as context, not treated as duplicates.
8. `insufficient_context` is reported when deterministic answerability is below
   the full-coverage threshold.
9. `visible_conflict_count` counts selected conflict groups; it does not scan
   hidden or unreadable data outside the pack.
10. A forbidden scope cannot enter ContextPack through broad queries or public
    exports; `AgentView.readable_scopes` still constrains the runtime
    `AgentAllowed` mask after binding.
11. `AgentAllowed` is part of the bitmap candidate set before
    `LIMIT ... CANDIDATES` is applied. ContextPack must never retrieve a broad
    top-k set and then post-filter unreadable cells.
12. Span-level packing, when enabled, is derived from query text and cell text;
    it does not read benchmark gold labels or call an external model.
13. Span provenance always points back to the readable source cell that already
    passed AQL and AgentView checks; it cannot introduce a new source outside
    the retrieved candidate set.
14. No HNSW, reranking, or LLM calls run inside ContextPack v1 itself.
15. Large-cell summarize-placeholder mode is deterministic metadata/reference
    output, not an LLM-generated summary.
16. Answer grounding only checks against the cells already present in the pack.
    It cannot use hidden cells, source-type labels, question-type labels, or
    evaluator gold facts.
17. AQL-built ContextPack cells must expose an access-decision trail that links
    the selected `cell_id` to the readable scope decision used before packing.
    If a pack is manually constructed without an `AgentView`, the trail is
    explicitly absent or `not_recorded`; it must not pretend an RBAC decision
    happened.

## Known Limits

- Token Estimator v2 is deterministic and profile-based, not a real tokenizer.
  It supports `ContextTokenProfile` variants for Cortex default, GPT-4o-like,
  DeepSeek-chat-like, Gemma-it-like, and BGE-M3-like budgeting. The profiles are
  guardrails for stable budget decisions, not exact vendor token counts.
- Citations are recognized from `citation=`, `source=`, or structured
  `source_id=` SourceRef metadata. When `REQUIRE confidence >= ...` is present
  in AQL, ContextPack retrieval filters candidates by `confidence_q16` or the
  SourceRef confidence derived from `source_trust_q16`.
- Redundancy control is deterministic and local to the pack. It supports sparse
  term overlap and exact fixed-point vector similarity from payload vectors, but
  does not call an external semantic model.
- Full contradiction detection is handled by VERIFY FACT; ContextPack only keeps
  numeric guard variants together so an agent can see conflicting values.
- `answerability_q16` is a deterministic coverage signal over explicit query
  terms and selected cell text/metadata. It is not an external LLM judgment.
- `conflict_visibility_q16` is a visibility metric for conflicts already
  selected into the pack. It is not a full contradiction detector; use VERIFY
  FACT for fact-level contradiction analysis.
- `source_freshness_q16` is a relative recency signal over the retrieved
  candidate set. Missing `created_unix_seconds` metadata maps to `unknown` with
  no freshness bonus. This is not a legal source-freshness certification.
- The private scope leak gate checks a broad `WHERE status = "ready"` query, a
  checkpoint/restart path, a compact/restart path, and all ContextPack export
  formats. It proves that a forbidden scope is excluded even when the AQL query
  itself does not include a scope predicate.
- The candidate-limit leak gate checks `LIMIT 1 CANDIDATES` with an unreadable
  lower candidate id ahead of the visible cell. It proves that the
  `AgentAllowed` bitmap is applied before candidate limiting and before
  ContextPack JSON, prompt, or Markdown export.
- `ContextLargeCellPolicy` is an explicit runtime option. `PreserveFirst`
  remains the default for compatibility. `Truncate`, `Exclude`,
  `SummarizePlaceholder`, and `SourceOnlyReference` keep reported token usage
  within the available budget when they include a transformed large cell.
- `ContextPackOptions.span_level_packing` remains disabled by default for
  compatibility. When enabled, it can select a relevant body span from an
  oversized first cell and records a deterministic
  `[context_pack_span=true line_start=... line_end=...]` marker in the packed
  payload. The public JSON, prompt, and Markdown exports also expose a
  `provenance` object for that selected span, including `source_cell_id`,
  `source_byte_start`, `source_byte_end`, `source_line_start`,
  `source_line_end`, and nested `source_ref` when the original cell had
  structured SourceRef metadata.

## Quality Gate

`crates/cortex-engine/fixtures/context_verify_quality_v1.cells` is the Core
Alpha ContextPack/VERIFY golden dataset. The test gate in
`crates/cortex-engine/tests/context_verify_quality.rs` loads this fixture and
seeds a small investment-project corpus with:

- supporting evidence for `Solar Plant budget is 1.2B KZT for 2025`;
- conflicting evidence for `1.4B KZT`;
- a private-scope distractor that must not leak into public retrieval.

The fixture asserts that ContextPack keeps both public numeric variants when
redundancy reduction is enabled, preserves citations, stays within budget, and
survives checkpoint/restart. It also asserts the matching VERIFY report returns
`mixed` evidence with a numeric mismatch guard.

The same gate also samples checked-in real-domain investment-project chunks from
`examples/real_domains/investment_projects/corpus/chunks.jsonl` to prove that
ContextPack can build cited, explained packs from the project corpus used by
the ANN/HNSW embedding baseline work, not only from tiny synthetic cells.

Run the gate directly with:

```bash
make context-verify-quality-check
```

For the focused ContextPack quality gate, run:

```bash
make context-pack-quality-check
make context-pack-answerability-check
make context-pack-conflict-visibility-check
make context-pack-private-scope-check
make context-pack-token-estimator-check
make context-pack-large-cell-policy-check
make context-pack-span-packing-check
```

That gate runs the ContextPack behavior tests, the ContextPack/VERIFY fixture,
and validates `examples/eval/context_pack_quality.jsonl`. The fixture records
measured release metrics:

- evidence coverage;
- token reduction versus raw chunks;
- citation coverage;
- duplicate suppression;
- deterministic ordering.
- answerability score and `insufficient_context` anomaly coverage.
- conflict visibility score and selected conflict-count coverage.
- private scope leak resistance across retrieval, checkpoint, compact, JSON,
  prompt, and Markdown exports.
- deterministic model-specific token profile coverage for multilingual payloads,
  citation overhead, model-name aliases, and invalid UTF-8 fallback behavior.
- large-cell policy coverage for truncate, exclude, summarize-placeholder, and
  source-only reference behavior.

Latest local evidence is tracked in
[`CONTEXT_PACK_QUALITY_EVIDENCE.md`](archive/CONTEXT_PACK_QUALITY_EVIDENCE.md).
