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
  "verdict": "mixed_evidence",
  "supporting": [
    {
      "cell_id": 1,
      "citation": "report_q1.pdf#page=3",
      "matched_terms": 7,
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
      "payload_text": "Solar Plant budget is 1.4B KZT in Q2.",
      "source_trust_q16": 32768,
      "source_trust_category": "unknown"
    }
  ],
  "numeric_conflicts": [
    {
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
```

`VERIFY FACT` reports both `source_trust_q16` and
`source_trust_category` for supporting and contradicting evidence. Equal text
matches are sorted by higher source trust first, then by `cell_id`, so the most
trusted evidence is visible without hiding lower-trust evidence. The category
thresholds are documented in `SOURCE_TRUST_MODEL.md`.

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
cell. `conflicts_for_fact` reads both inline markers and persisted relation
cells, then applies the caller's `AgentView` readable-scope mask before
returning records.

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

Markdown is intended for human review. Audit text is deterministic line-based
output for diffing, archiving, or attaching to external review tooling.

## Limitations (Alpha)

- **Unit parsing** is heuristic, not a full SI unit converter.
- **Magnitude parsing** relies on explicit `B`/`M`/`K` suffixes or raw integers.
- **Currency** must be explicit in the fact or in cell metadata.
- **Temporal reasoning is validity-window based** — only explicit
  `valid_from`/`valid_to` headers and explicit fact dates are interpreted.
  Quarter names and natural-language relative dates are not parsed yet.
- **Source trust is deterministic but simple** — `source_trust_q16` is
  classified as `low`, `medium`, `high`, or `official`; missing values are
  reported as `unknown` with the default q16. It is not a full trust/provenance
  model.

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

This is still a deterministic alpha fixture, not a measured accuracy benchmark.
Future Verification v1 work should add larger labelled datasets and metric-aware
temporal reasoning.

Run the shared ContextPack/VERIFY quality gate directly with:

```bash
make context-verify-quality-check
```

Run the focused labelled verification evaluation gate with:

```bash
make verification-quality-check
```

That gate executes `examples/eval/verification_cases.jsonl` through the engine
and writes a confusion-matrix report to `target/verification-quality/report.json`.
Latest local evidence is tracked in
[`VERIFICATION_QUALITY_EVIDENCE.md`](VERIFICATION_QUALITY_EVIDENCE.md).
