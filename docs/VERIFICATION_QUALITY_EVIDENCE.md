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
- missing citation guards;
- equal numeric values;
- ambiguous evidence;
- no-evidence insufficient verdicts.

## Latest Local Metrics

```text
case_count: 9
accuracy_q16: 65535
supported: 3 / 3
contradicted: 3 / 3
mixed: 1 / 1
insufficient: 2 / 2
guard_cases: 3
numeric_guard_cases: 2
citation_guard_cases: 1
false_positive_count: 0
false_negative_count: 0
```

## Boundary

This gate proves deterministic `VERIFY FACT` behavior against labelled local
fixtures. It does not prove real-world fact-checking accuracy, temporal
reasoning, or LLM answer quality.
