# Verification Quality Evidence

Last local verification quality run: 2026-06-01, passed.

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
- no-evidence insufficient verdicts.
- support-ticket operational facts;
- legal-policy approval/effective-date facts;
- technical-doc API/SDK/security facts;
- world-indicator numeric and currency facts.

## Latest Local Metrics

```text
case_count: 50
accuracy_q16: 65535
supported: 14 / 14
contradicted: 20 / 20
mixed: 8 / 8
insufficient: 8 / 8
domain_counts: investment_projects=30, legal_policies=5, support_tickets=5, technical_docs=5, world_indicators=5
guard_cases: 20
numeric_guard_cases: 16
citation_guard_cases: 4
false_positive_count: 0
false_negative_count: 0
```

## Boundary

This gate proves deterministic `VERIFY FACT` behavior against labelled local
fixtures. It does not prove real-world fact-checking accuracy, temporal
reasoning, or LLM answer quality.
