# Context Pack v1

Context Pack is the first agent-ready retrieval surface above raw `Retrieve`.
The v1 contract keeps the current core deterministic while making the JSON
surface stable enough for SDK and UI consumers:

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
- Optional sparse redundancy reduction using fixed-point Jaccard.

Public JSON responses include:

```json
{
  "schema_version": "context_pack.v1",
  "token_budget_tokens": 1000,
  "estimated_tokens": 42,
  "truncated": false,
  "citations_required": false,
  "cells": [],
  "anomalies": []
}
```

`schema_version` is required. Future incompatible ContextPack response changes
must introduce a new schema version and update OpenAPI, SDKs, snapshots, and
API changelog entries together.

The CLI exposes:

```bash
cargo run -p cortex-cli -- context ./data project:investments '<AQL>'
```

The HTTP server exposes:

```http
POST /v1/context?scope=project:investments

RETRIEVE CONTEXT ...
```

## Invariants

1. Context packing never bypasses AQL policy, binder, bitmap VM, or AgentView.
2. Ordering follows the retrieve candidate order.
3. Token estimates are deterministic integer estimates.
4. Requested budget is clamped by `AgentView::effective_budget`.
5. Citation requirements produce anomalies instead of silently passing.
6. Redundancy reduction, when enabled, reports skipped cells as anomalies.
7. No HNSW, reranking, or LLM calls run inside ContextPack v1 itself.

## Known Limits

- Token estimation is byte-based and approximate.
- Citations are recognized only from `source=` or `citation=` payload lines.
- Redundancy control is sparse term based; dense semantic scoring and
  contradiction detection are future milestones.

## Quality Gate

`crates/cortex-engine/tests/context_verify_quality.rs` is the Core Alpha
ContextPack/VERIFY golden fixture. It seeds a small investment-project corpus
with:

- supporting evidence for `Solar Plant budget is 1.2B KZT for 2025`;
- conflicting evidence for `1.4B KZT`;
- a private-scope distractor that must not leak into public retrieval.

The fixture asserts that ContextPack keeps both public numeric variants when
redundancy reduction is enabled, preserves citations, stays within budget, and
survives checkpoint/restart. It also asserts the matching VERIFY report returns
`mixed` evidence with a numeric mismatch guard.
