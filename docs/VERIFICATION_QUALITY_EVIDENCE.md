# Verification Quality Evidence

Last local verification quality run: 2026-05-31, passed.

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
- missing citation guards;
- equal numeric values;
- ambiguous evidence;
- no-evidence insufficient verdicts.

## Latest Local Metrics

```text
case_count: 8
accuracy_q16: 65535
supported: 3 / 3
contradicted: 2 / 2
mixed: 1 / 1
insufficient: 2 / 2
guard_cases: 2
numeric_guard_cases: 1
citation_guard_cases: 1
```

## Boundary

This gate proves deterministic `VERIFY FACT` behavior against labelled local
fixtures. It does not prove real-world fact-checking accuracy, temporal
reasoning, or LLM answer quality.
