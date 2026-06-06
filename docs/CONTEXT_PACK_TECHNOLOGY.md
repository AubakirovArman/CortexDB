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
-> redundancy checks
-> token budget packing
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

When citations are required and a selected cell has a source/citation, the
estimate includes fixed citation overhead so the reported budget accounts for
evidence markers, not only payload text.

When a middle candidate would exceed the requested budget, the pack marks
truncation, emits a `token_overload` anomaly, and continues scanning later
smaller candidates. The first candidate can still be included even when it is
larger than the requested budget, preserving the old "return at least one
candidate" behavior.

## Citations

Context Pack treats citations as evidence markers extracted from payload
metadata, currently from lines such as:

```text
source=report_q1.pdf#page=3
citation=report_q1.pdf#page=3
source_id=ifc:project-1
source_url=https://example.test/projects/1
doc_id=doc-1
chunk_id=chunk-7
confidence_q16=60000
```

If citations are required and a selected cell has none, the pack emits a
`missing_citation` anomaly. Structured SourceRef metadata is treated as valid
provenance; `doc_id` is normalized to `document_id`, `chunk_id` is normalized to
`cell_range`, and `source_url`/`url` is preserved for API consumers. AQL
`REQUIRE confidence >= ...` filters retrieval candidates by SourceRef
`confidence_q16` before packing. ContextPack does not invent provenance.

## Redundancy Control

When redundancy reduction is enabled, Context Pack skips near-duplicate cells
before budget overload checks and emits `redundant_cell` anomalies for
visibility. This prevents a large duplicate from prematurely stopping the pack.

Core Alpha supports sparse term redundancy checks and vector-aware comparison
when vector payloads are available. Numeric guards prevent distinct numeric
claims for the same project/metric from being incorrectly collapsed as
duplicates.

## Explain Metadata

Each selected cell can include explain fields such as:

- `matched_terms`;
- `why_selected`;
- structured `score_components`;
- `base_bm25`;
- `source_trust_q16`;
- `source_trust_category`;
- `source_trust_bonus`;
- `redundancy_penalty`.

Excluded candidates are reported through anomalies. `why_excluded` explains
whether the candidate was removed by redundancy control or by
`token_budget_tokens` pressure. This keeps selected and rejected context
debuggable through the same public JSON surface.

Source trust categories are deterministic q16 bands: missing metadata is
`unknown`, then explicit values classify as `low`, `medium`, `high`, or
`official`. The category is explanatory; the numeric bonus remains the q16 value
so ranking stays deterministic and backward compatible.

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
cargo run -p cortex-cli -- context ./data project:investments '<AQL>' --format prompt
cargo run -p cortex-cli -- context ./data project:investments '<AQL>' --format markdown
```

HTTP:

```http
POST /v1/context?scope=project:investments
POST /v1/context?scope=project:investments&format=prompt
POST /v1/context?scope=project:investments&format=markdown
```

SDKs expose the same HTTP contract through typed client helpers, including
agent prompt and Markdown export helpers where available.

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
