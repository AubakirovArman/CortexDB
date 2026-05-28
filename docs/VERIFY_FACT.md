# VERIFY FACT

`VERIFY FACT` is CortexDB's deterministic fact-checking primitive. Unlike LLM-based verification, it runs entirely inside the database engine and produces a structured report with evidence, contradictions, and numeric guards.

## How It Works

1. **Parse the fact** — extract numeric values, units, and currencies from the statement.
2. **Search evidence** — find cells that support or contradict the fact.
3. **Compare numbers** — detect numeric mismatches between the claimed value and stored values.
4. **Produce a verdict** — `supported`, `insufficient`, `contradicted`, or `mixed_evidence`.

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
      "source_trust_q16": 32768
    }
  ],
  "contradicting": [
    {
      "cell_id": 2,
      "citation": "report_q2.pdf#page=5",
      "matched_terms": 4,
      "payload_text": "Solar Plant budget is 1.4B KZT in Q2.",
      "source_trust_q16": 32768
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

The `metric=budget` and `value=1200000000` lines help VERIFY FACT produce precise `numeric_conflicts`.

## Limitations (Alpha)

- **Unit parsing** is heuristic, not a full SI unit converter.
- **Magnitude parsing** relies on explicit `B`/`M`/`K` suffixes or raw integers.
- **Currency** must be explicit in the fact or in cell metadata.
- **No temporal reasoning** — "budget was 1.2B in Q1" and "budget is 1.4B in Q2" are treated as a conflict, not as a timeline update.
- **No source trust scoring** — all sources are treated equally in alpha.

## Future (Verification v1)

- `NumericValue` struct with normalized unit representation.
- Metric-aware comparison (budget vs revenue vs cost).
- Source trust scoring and evidence ranking.
- Contradiction index output.
