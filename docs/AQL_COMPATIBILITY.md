# AQL Compatibility

Status: local Epic 9 evidence gate.

AQL v0.4 is the Core Alpha query contract. This document maps the frozen syntax
and error behavior to the tests and reports that prove compatibility for
clients.

## Compatibility Surface

The current grammar and binder behavior are defined in:

```text
docs/AQL_V0_4.md
```

The focused gate is:

```bash
make aql-compat-check
```

It writes:

```text
target/aql-compat/report.json
target/aql-compat/*.log
```

## Covered Client Cases

`make aql-compat-check` covers:

- malformed AQL through parser diagnostics and HTTP `invalid_aql`;
- forbidden scope through bind policy denial and HTTP `permission_denied`;
- unknown field through `FieldNotFilterable` and HTTP `unknown_field`;
- unsupported comparator through `UnsupportedComparator` and HTTP
  `unsupported_operator`;
- LIMIT and REQUIRE parsing, policy clamp, and quality-threshold binding;
- explain snapshots through `EXPLAIN RETRIEVE CONTEXT` parse and bind behavior.

The runtime `EXPLAIN RETRIEVE CONTEXT` response shape is covered by
`make openapi-contract-check`.

The stable distinction for SDK callers is:

| Client situation | Stable class |
| --- | --- |
| invalid syntax or invalid mode | parse error / HTTP `invalid_aql` |
| readable policy violation | bind policy denial / HTTP `permission_denied` |
| unknown unavailable scope | bind `UnknownScope` with safe message |
| unknown field | bind `FieldNotFilterable` / HTTP `unknown_field` |
| unsupported comparator | bind `UnsupportedComparator` / HTTP `unsupported_operator` |

Safe messages must not reveal private brain or scope names.

## Golden Test Pack

The golden pack is:

```text
crates/cortex-aql/tests/aql_v0_4_golden_tests.rs
```

It freezes:

- `RETRIEVE CONTEXT` raw AST shape;
- bound retrieve bitmap bytecode;
- `EXPLAIN RETRIEVE CONTEXT`;
- `VERIFY FACT` and `REMEMBER` parse/bind contracts;
- parse diagnostics and bind safe messages;
- unsupported comparator behavior.

Parser and binder support tests also run through the gate:

```text
crates/cortex-aql/tests/parser_tests.rs
crates/cortex-aql/tests/binder_hardening_tests.rs
crates/cortex-aql/tests/aql_stabilization_tests.rs
```

## Changelog Policy

AQL grammar or binder compatibility changes must update:

- `docs/AQL_CHANGELOG.md`;
- `docs/AQL_V0_4.md` or a new versioned grammar doc;
- golden tests;
- API/SDK docs if HTTP-visible error behavior changes.

Breaking changes require a new AQL version document instead of silently
reinterpreting v0.4.
