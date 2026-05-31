# CLI Product Evidence

Last local CLI product run: 2026-05-31, passed.

Run:

```bash
make cli-product-check
```

Primary artifact:

```text
target/cli-product/report.json
```

## Coverage

This gate covers:

- `cortexdb --help`;
- `cortexdb version`;
- `cortexdb doctor`;
- `cortexdb completions bash`;
- CLI docs coverage for common operator commands.

## Latest Local Checks

```text
doctor: true
completions: true
stats: true
validate: true
context: true
verify: true
search: true
search-vector-eval: true
audit: true
```

## Boundary

This gate proves the local CLI contract and diagnostics are usable in Core
Alpha. It does not prove installer packaging, shell installation on every OS,
or published SDK package behavior.
