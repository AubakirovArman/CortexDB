# Context Pack Technology

Context Pack is CortexDB's agent-facing retrieval product. It is not a raw
vector-search result and not an LLM answer. It is a deterministic package of
cells selected from AQL retrieval, bounded by token budget, annotated with
citations and anomaly reports, and shaped for an external agent or model.

## Why It Exists

Agents rarely need "the nearest 10 chunks" by itself. They need bounded,
permission-safe, evidence-carrying context that can be inserted into a prompt
without leaking private scopes, exceeding the model window, or hiding
contradictions.

Context Pack turns:

```text
RETRIEVE CONTEXT ...
```

into:

```text
selected cells
token budget accounting
source/citation fields
explain metadata
anomaly reports
stable JSON
```

## Pipeline

The Core Alpha pipeline is:

```text
AQL string
-> parser
-> binder and policy checks
-> bitmap VM retrieval
-> candidate cells
-> feedback/scoring order
-> token budget packing
-> redundancy checks
-> citation/anomaly checks
-> ContextPack JSON
```

Context Pack never bypasses AQL permissions. It consumes the same
`AgentView`, scope policy, candidate masks, and retrieval limits as the normal
retrieve path.

## Data Model

The engine-level structures are implemented in `crates/cortex-engine`:

- `ContextPackOptions`
- `ContextPack`
- `ContextPackCell`
- `ContextPackAnomaly`
- `ContextPackAnomalyCode`
- `Database::context_pack_from_aql`

The HTTP/API shape is documented in:

- [`CONTEXT_PACK.md`](CONTEXT_PACK.md)
- [`API_JSON_SCHEMAS.md`](API_JSON_SCHEMAS.md)
- [`openapi.yaml`](openapi.yaml)

## Token Budget

Core Alpha uses deterministic integer token estimates. The estimate is
approximate, but stable: the same payload and options produce the same budget
decision.

When a candidate would exceed the requested budget, the pack marks truncation
or emits an anomaly instead of silently overflowing the pack.

## Citations

Context Pack treats citations as evidence markers extracted from payload
metadata, currently from lines such as:

```text
source=report_q1.pdf#page=3
citation=report_q1.pdf#page=3
```

If citations are required and a selected cell has none, the pack emits a
`missing_citation` anomaly. It does not invent provenance.

## Redundancy Control

When redundancy reduction is enabled, Context Pack skips near-duplicate cells
and emits `redundant_cell` anomalies for visibility.

Core Alpha supports sparse term redundancy checks and vector-aware comparison
when vector payloads are available. Numeric guards prevent distinct numeric
claims for the same project/metric from being incorrectly collapsed as
duplicates.

## Explain Metadata

Each selected cell can include explain fields such as:

- matched terms;
- selection reason;
- lexical score component;
- source-trust bonus;
- redundancy penalty.

This is meant for debugging and UI display. It is not a legal proof or a
production-grade factual-certification score.

## Security Invariants

1. AQL policy, binder, bitmap VM, and `AgentView` are applied before packing.
2. Context Pack cannot expand readable scopes.
3. Token budget decisions are deterministic.
4. Missing citations are reported, not fabricated.
5. Redundancy decisions are visible through anomalies.
6. Context Pack does not call an LLM inside the database core.
7. Exact retrieval remains the correctness fallback for guarded ANN/HNSW.

## Interfaces

Rust:

```rust
db.context_pack_from_aql(aql, &agent_view, options)?;
```

CLI:

```bash
cargo run -p cortex-cli -- context ./data project:investments '<AQL>' --json
```

HTTP:

```http
POST /v1/context?scope=project:investments
```

SDKs expose the same HTTP contract through typed client helpers.

## What It Is Not

Context Pack is not:

- an LLM answer generator;
- a replacement for final human review;
- a production-grade contradiction engine;
- a guarantee that all relevant knowledge has been retrieved;
- a vector database result dump.

It is the deterministic context compiler layer between CortexDB retrieval and
an external agent/model.

## Quality Gates

The main quality gate is:

```bash
make context-verify-quality-check
```

The fixture validates budget behavior, citation anomalies, redundancy handling,
numeric-conflict preservation, and checkpoint/restart safety for a small
investment-project corpus.

## Next Milestones

- stronger token estimation;
- richer source-reference model;
- field-aware ranking;
- calibrated Context Pack quality reports;
- UI explain views for selected and rejected cells;
- SDK contract snapshots for every schema version.
