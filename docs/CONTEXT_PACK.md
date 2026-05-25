# Context Pack v0

Context Pack is the first agent-ready retrieval surface above raw `Retrieve`.
It keeps the current core simple:

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
7. No vector search, HNSW, reranking, or LLM calls run in v0.

## Known Limits

- Token estimation is byte-based and approximate.
- Citations are recognized only from `source=` or `citation=` payload lines.
- Redundancy control is sparse term based; dense semantic scoring and
  contradiction detection are future milestones.
