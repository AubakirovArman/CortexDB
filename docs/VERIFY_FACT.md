# VERIFY FACT

`VERIFY FACT` is CortexDB's deterministic fact-checking primitive. Unlike LLM-based verification, it runs entirely inside the database engine and produces a structured report with evidence, contradictions, and numeric guards.

It is not legal proof and must not be presented as legal advice. Legal-grade
verification is a separate future epic that requires domain selection,
admissible-source rules, citation completeness, and human reviewer approval.

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
- **No temporal reasoning** — "budget was 1.2B in Q1" and "budget is 1.4B in Q2" are treated as a conflict, not as a timeline update.
- **Source trust is deterministic but simple** — `source_trust_q16` is
  classified as `low`, `medium`, `high`, or `official`; missing values are
  reported as `unknown` with the default q16. It is not a full trust/provenance
  model.

## Future (Verification v1)

- Metric-aware comparison (budget vs revenue vs cost).
- Richer source trust policy inputs beyond q16 thresholds.
- Contradiction index output.

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
