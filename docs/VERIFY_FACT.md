# VERIFY FACT

`VERIFY FACT` is CortexDB's deterministic fact-checking primitive. Unlike LLM-based verification, it runs entirely inside the database engine and produces a structured report with evidence, contradictions, and numeric guards.

It is not legal proof and must not be presented as legal advice. Legal-grade
verification is a separate future epic that requires domain selection,
admissible-source rules, citation completeness, and human reviewer approval.

## How It Works

1. **Parse the fact** — extract numeric values, units, currencies, and dates from the statement.
2. **Search evidence** — find cells that support or contradict the fact.
3. **Compare numbers** — detect numeric mismatches between the claimed value and stored values.
4. **Check validity windows** — exclude evidence outside `valid_from` / `valid_to`.
5. **Produce a verdict** — `supported`, `insufficient`, `contradicted`, or `mixed_evidence`.

## AQL Syntax

```aql
VERIFY FACT "Solar Plant budget is 1.2B KZT"
IN BRAIN default;
```

## CLI Usage

```bash
cortexdb verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' --json
```

## Response Format

```json
{
  "fact": "Solar Plant budget is 1.2B KZT",
  "status": "mixed_evidence",
  "verdict": "mixed_evidence",
  "confidence_q16": 32768,
  "supporting": [
    {
      "cell_id": 1,
      "citation": "report_q1.pdf#page=3",
      "matched_terms": 7,
      "match_score_q16": 65535,
      "match_kind": "exact_text",
      "payload_text": "Solar Plant budget is 1.2B KZT in Q1.",
      "source_trust_q16": 32768,
      "source_trust_category": "unknown"
    }
  ],
  "contradicting": [
    {
      "cell_id": 2,
      "citation": "report_q2.pdf#page=5",
      "matched_terms": 4,
      "match_score_q16": 65535,
      "match_kind": "numeric_contradiction",
      "payload_text": "Solar Plant budget is 1.4B KZT in Q2.",
      "source_trust_q16": 32768,
      "source_trust_category": "unknown"
    }
  ],
  "numeric_conflicts": [
    {
      "kind": "numeric",
      "metric": "budget",
      "left": "1.2B KZT",
      "right": "1.4B KZT"
    }
  ]
}
```

## Verdicts

| Verdict | Meaning | When It Happens |
|---------|---------|-----------------|
| `supported` | All evidence agrees with the fact. | No contradictions, sufficient evidence. |
| `insufficient` | Not enough evidence to judge. | Few or no matching cells. |
| `contradicted` | Evidence directly contradicts the fact. | Strong contradicting evidence, no supporting. |
| `mixed_evidence` | Some evidence supports, some contradicts. | The most common case for real-world data. |

## Numeric Conflict Detection

CortexDB parses numbers from:
- The **fact statement** itself (e.g. `"1.2B KZT"` → `1200000000`).
- **Cell metadata** lines `value=` and `currency=`.
- **Cell body text** for fallback parsing.

If a cell contains a different numeric value for the same metric, a `numeric_conflict` is reported.

### Example Cell Metadata

```text
scope=project:investments
status=ready
type=fact
source=report_q1.pdf
project=Solar Plant
metric=budget
value=1200000000
currency=KZT

Solar Plant budget is 1.2B KZT in Q1.
```

The `metric=budget` line helps VERIFY FACT label precise
`numeric_conflicts`. Numeric comparison itself is computed in the engine from
typed `NumericValue` pairs extracted from the fact and evidence body text, so
CLI/server responses do not re-parse payloads independently.

The Rust engine exposes the same deterministic contract through
`parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`,
`compare_numeric_values`, and `normalized_numeric_equal`. These helpers keep
currency/unit/magnitude handling in one integer-only implementation and return
structured `VerificationNumericConflict` entries for API, CLI, SDK, markdown,
and audit exports.

Each `numeric_conflicts[]` row includes `kind`: `numeric` for ordinary
cross-evidence numeric disagreement, `temporal` for dated overlap classes, and
`citation` when the same structured `source_ref` disagrees on the value.

## Evidence Match Kinds

Every supporting or contradicting evidence item reports:

- `match_kind`: deterministic match class;
- `match_score_q16`: fixed-point confidence-like score used for stable sorting.

The report-level `confidence_q16` is a deterministic verdict confidence. For a
supported verdict it is the best supporting evidence score capped by source
trust; for a contradicted verdict it is the best contradicting evidence score
capped by source trust; for mixed evidence it is the weaker of the two best
sides; for insufficient evidence it is `0`.

Current match kinds are:

| Match Kind | Meaning |
|------------|---------|
| `exact_text` | The normalized fact text appears directly in the evidence body. |
| `semantic_entailment` | All non-numeric fact terms are covered by the evidence body. |
| `numeric_entailment` | Numeric values match after magnitude/unit/currency normalization, with required non-numeric terms present. |
| `semantic_contradiction` | The evidence body or inline marker contradicts the fact text. |
| `numeric_contradiction` | A comparable numeric value conflicts with the fact value. |
| `graph_contradiction` | A readable graph relation marks the fact as contradicted. |

This is deterministic lexical/numeric/graph matching, not LLM-based semantic
judgement. Partial term overlap is intentionally not counted as supporting
evidence.

## Temporal Validity Detection

Evidence cells may include validity headers:

```text
scope=project:investments
status=ready
type=fact
source=report_2024.pdf
valid_from=2024-01-01
valid_to=2024-12-31

Solar Plant budget is 1.2B KZT.
```

`VERIFY FACT` extracts an explicit fact date from `YYYY-MM-DD`, `YYYY/MM/DD`,
or a four-digit year. If the fact date falls outside the evidence validity
window, the cell is treated as stale evidence:

- it is not counted as supporting evidence;
- it is not counted as contradicting evidence;
- a `stale_fact` guard is emitted.

The Rust engine exposes this deterministic date contract through
`parse_temporal_date` and `extract_temporal_query_range`. Year-only facts are
treated as a full-year range, while `valid_from=2025` and `valid_to=2025` are
expanded to the beginning and end of the year respectively.

## Source Trust

Evidence cells may include deterministic provenance trust:

```text
source_trust_q16=60000
source_trust_class=official
```

`VERIFY FACT` reports both `source_trust_q16` and
`source_trust_category` for supporting and contradicting evidence. Equal text
matches are sorted by higher source trust first, then by `cell_id`, so the most
trusted evidence is visible without hiding lower-trust evidence. The category
thresholds and calibrated class weights are documented in
`SOURCE_TRUST_MODEL.md`. If both fields are present, explicit
`source_trust_q16` wins.

## Contradiction Index

`Database::conflict_index` exposes deterministic contradiction records from two
sources:

1. Inline fact markers such as `contradicts=ABC budget approved`.
2. Persisted Relation cells with `predicate=contradicts`.

Persisted contradiction relations are durable database cells. They use the
existing Relation cell model:

```text
scope=project:investments
status=ready
type=relation
source=reviewer

subject=cell:42
predicate=contradicts
object=ABC budget approved
source_cell_id=42
```

The relation cell is written through WAL and survives restart like any other
cell. `conflict_index` reads both inline markers and persisted relation cells,
then applies the caller's `AgentView` readable-scope mask before returning
records.

Conflict records include deterministic query facets:

```text
entity: project=... or entity=... from the evidence cell body
metric: metric=... from the evidence cell body
source: source/source_id/citation metadata from the evidence cell
```

The engine exposes focused index lookups:

```rust
db.conflicts_for_fact("ABC Airport budget approved", view);
db.conflicts_for_entity("ABC Airport", view);
db.conflicts_for_metric("budget", view);
db.conflicts_for_source("ifc", view);
```

For persisted relation cells, entity/metric/source facets are inherited from
the readable `source_cell_id` evidence cell. If the source cell is outside the
caller's `AgentView`, the relation remains visible only through its own
readable scope and does not leak hidden evidence facets.

## Graph Verification

`VERIFY FACT` also uses the local knowledge graph relation layer:

- `predicate=contradicts`, `predicate=fact_contradicts_fact`, and compatible
  aliases are treated as graph contradiction evidence and are not counted as
  supporting evidence just because the relation object contains the fact text.
- `predicate=source_supports_fact` edges can enrich existing supporting
  evidence with citation/source metadata and a higher source-trust score.

Example source-support edge:

```text
scope=project:investments
status=ready
type=relation
source=ifc:disclosure-001
source_trust_q16=60000

subject=source:ifc:disclosure-001
predicate=source_supports_fact
object=cell:42
```

This edge does not create support by itself. The fact cell still needs to match
the verified claim text. The relation only explains and strengthens provenance
for evidence the caller can already read through `AgentView`.

## Report Exports

`VerificationReport` has engine-level stable exports:

```bash
cortexdb verify ./db project:investments '<VERIFY AQL>' --format markdown
cortexdb verify ./db project:investments '<VERIFY AQL>' --format audit
```

HTTP uses the same formats:

```http
POST /v1/verify?scope=project:investments&format=markdown
POST /v1/verify?scope=project:investments&format=audit
```

Markdown is intended for human review. It includes:

- a report summary table;
- supporting evidence;
- contradicting evidence;
- numeric conflicts;
- guards;
- limitations for deterministic review.

Audit text is deterministic line-based output for diffing, archiving, or
attaching to external review tooling.

## Measured Conflict Coverage

The DV7 CI-safe recall gate is `make verify-conflict-recall-check`. It runs a
labeled corpus through `Database::verify_fact_aql` and writes
`target/verification-quality/conflict-recall-report.json`. The report schema
`cortexdb.verify_conflict_recall.report.v1` is then checked by
`make docs-claims-check` so the numbers in this section stay tied to the
measured report.

Latest local measured report:

- Case count: 180
- Conflict cases: 150
- must-NOT-conflict controls: 30
- Conflict recall: 100.00% (`recall_q16=65535`; gate minimum
  `recall_q16>=58981`, equivalent to recall >= 0.90)
- Precision: 100.00% (`precision_q16=65535`)
- False-conflict rate: 0.00% (`false_conflict_rate_q16=0`; gate maximum
  `false_conflict_rate_q16<=3276`, equivalent to false-conflict <= 0.05)

Supported conflict classes exercised by the measured gate:

- **magnitude/numeric** conflicts over explicit `B`/`M`/`K` suffixes and raw
  integer equivalents;
- **unit-class time conversion** conflicts and agreements such as `60 min`,
  `1h`, and `2 h`;
- **currency-mismatch** conflicts when both sides declare incompatible
  currencies;
- **temporal same-date** conflicts when a dated fact overlaps an evidence
  `valid_from`/`valid_to` window with a different value;
- **citation same-source** conflicts when two structured `source_ref` entries
  for the same document location disagree;
- **format variants** such as `$1.2M`, `1.4 million USD`, and explicit
  `currency=` body fields;
- **must-NOT-conflict controls** for normalized equal values, equal same-source
  citations, in-window temporal support, and equivalent formatted amounts.

Scope notes:

- Unit parsing is heuristic and limited to the parser aliases covered by the
  gate; it is not a full SI converter.
- Currency mismatch is detected, but there is no FX conversion.
- Currency or unit values missing on one side are treated as incomparable
  rather than guessed.
- Temporal reasoning is validity-window based: explicit `valid_from` /
  `valid_to` headers and explicit fact dates are interpreted. Quarter names and
  natural-language relative dates are not parsed yet.
- Source trust is deterministic but simple: `source_trust_q16` is classified as
  `low`, `medium`, `high`, or `official`; missing values are reported as
  `unknown` with the default q16. It is not a full trust/provenance model.

## Future (Verification v1)

- Metric-aware comparison (budget vs revenue vs cost).
- Richer source trust policy inputs beyond q16 thresholds.
- Public API output for the durable contradiction index if product consumers
  need it outside the engine API.

## Quality Gate

`crates/cortex-engine/fixtures/context_verify_quality_v1.cells` is the shared
ContextPack/VERIFY regression dataset. The gate in
`crates/cortex-engine/tests/context_verify_quality.rs` proves that the same
public evidence set can be packed for an agent and verified deterministically:

- `1.2B KZT` supporting evidence is returned as supporting evidence;
- `1.4B KZT` evidence for the same project/metric is returned as
  contradicting evidence;
- the report status is `mixed`;
- the numeric mismatch guard is emitted for the conflicting cell;
- private-scope evidence is excluded by the AgentView.

The ContextPack/VERIFY fixture is still a deterministic regression fixture. The
measured conflict recall numbers above come from the DV7 labeled conflict
benchmark, not from the single ContextPack quality fixture. Future Verification
v1 work should add larger external labeled datasets and metric-aware temporal
reasoning.

Run the shared ContextPack/VERIFY quality gate directly with:

```bash
make context-verify-quality-check
```

Run the focused labelled verification evaluation gate with:

```bash
make verification-quality-check
```

That gate also runs the measured conflict coverage lane:

```bash
make verify-conflict-recall-check
make docs-claims-check
```

The broader labelled status gate executes
`examples/eval/verification_cases.jsonl` through the engine and writes a
confusion-matrix report to `target/verification-quality/report.json`. It also
writes `target/verification-quality/dashboard.json` and
`target/verification-quality/dashboard.md` with false-positive/false-negative
counts and per-domain quality tables for release review.
Latest local evidence is tracked in
[`VERIFICATION_QUALITY_EVIDENCE.md`](archive/VERIFICATION_QUALITY_EVIDENCE.md).
