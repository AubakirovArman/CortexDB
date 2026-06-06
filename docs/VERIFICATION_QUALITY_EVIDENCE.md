# Verification Quality Evidence

Last local verification quality run: 2026-06-06, passed.

Run:

```bash
make verification-quality-check
```

Primary artifacts:

```text
examples/eval/verification_cases.jsonl
target/verification-quality/report.json
```

## Coverage

The release fixture covers:

- supported evidence;
- contradiction markers;
- mixed supporting and contradicting evidence;
- numeric mismatch guards;
- currency mismatch guards;
- date mismatch guards;
- missing citation guards;
- equal numeric values;
- magnitude-normalized equal values;
- same-company / different-project hard cases;
- same-project / different-period hard cases;
- updated-value vs old-value hard cases;
- natural-language negation and antonym contradictions;
- ambiguous evidence;
- unreadable-scope evidence;
- no-evidence insufficient verdicts;
- support-ticket operational facts;
- legal-policy approval/effective-date facts;
- technical-doc API/SDK/security facts;
- world-indicator numeric and currency facts;
- 200-case v3 coverage with temporal, numeric, currency, source,
  ambiguous, and outdated evidence cases.

## Latest Local Metrics

```text
case_count: 200
accuracy_q16: 65535
supported: 57 / 57
contradicted: 85 / 85
mixed: 29 / 29
insufficient: 29 / 29
domain_counts: investment_projects=60, legal_policies=35, support_tickets=35, technical_docs=35, world_indicators=35
v3_category_counts: temporal=70, numeric=114, currency=75, source=25, ambiguous=23, outdated=24
guard_cases: 127
numeric_guard_cases: 102
citation_guard_cases: 25
false_positive_count: 0
false_negative_count: 0
```

## Boundary

This gate proves deterministic `VERIFY FACT` behavior against labelled local
fixtures. It does not prove real-world fact-checking accuracy, temporal
reasoning, or LLM answer quality.
