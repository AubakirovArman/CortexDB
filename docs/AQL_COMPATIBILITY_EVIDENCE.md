# AQL Compatibility Evidence

Last local AQL compatibility run: 2026-05-31, passed.

Run:

```bash
make aql-compat-check
```

Primary artifacts:

```text
target/aql-compat/report.json
target/aql-compat/*.log
```

Latest report:

```text
status: passed
report: target/aql-compat/report.json
finished_at: 2026-05-31T19:35:50Z
```

## Matrix

| Suite | Purpose |
| --- | --- |
| AQL v0.4 golden | Freezes AST shape, bytecode, explain, and stable error messages. |
| Parser contract | Covers malformed AQL, `LIMIT`, `REQUIRE`, precedence, and integer errors. |
| Binder contract | Covers forbidden scope, safe diagnostics, and filter compilation. |
| AQL stabilization | Covers defaults, clamps, `REQUIRE` thresholds, and decimal behavior. |
| HTTP invalid AQL | Confirms SDK-visible `invalid_aql` error code. |
| HTTP unknown field | Confirms SDK-visible `unknown_field` error code. |
| HTTP unsupported operator | Confirms SDK-visible `unsupported_operator` error code. |
| HTTP permission denied | Confirms SDK-visible permission-denied class. |

## Boundary

The local gate proves:

- AQL v0.4 parser and binder compatibility tests pass;
- `EXPLAIN RETRIEVE CONTEXT` parses, binds, and exposes a stable HTTP/OpenAPI
  explain shape;
- malformed AQL, permission denied, unknown field, unsupported operator,
  `LIMIT`, and `REQUIRE` are covered;
- HTTP error codes remain distinguishable for SDK callers.

The gate does not prove:

- future AQL v0.5 compatibility;
- semantic ranking quality;
- new comparators beyond the documented v0.4 binder surface.
