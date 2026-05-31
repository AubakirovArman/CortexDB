# AQL Changelog

This changelog tracks AQL grammar and binder compatibility. It is separate from
the HTTP API changelog because query syntax can remain stable even when response
fields evolve.

## AQL v0.4

Status: current Core Alpha contract.

Frozen behavior:

- `RETRIEVE CONTEXT`, `EXPLAIN RETRIEVE CONTEXT`, `VERIFY FACT`, and
  `REMEMBER` statements parse as one full-input statement ending in `;`.
- `USING MODE`, `BUDGET`, `LIMIT`, `WHERE`, and `REQUIRE` are supported for
  `RETRIEVE CONTEXT` as documented in `AQL_V0_4.md`.
- `WHERE` precedence is `NOT > AND > OR`.
- `IN [...]` is supported for bitmap filters.
- `LIMIT` can be clamped by policy.
- `REQUIRE` updates citation, confidence, source-trust, and freshness policy.
- Parse errors expose stable safe kinds.
- Bind errors expose stable codes and safe messages.

Evidence:

```bash
make aql-compat-check
```

Golden tests:

```text
crates/cortex-aql/tests/aql_v0_4_golden_tests.rs
```

## Change Policy

A breaking change is any change that removes or reinterprets v0.4 syntax,
changes stable parse error kinds, changes stable bind error codes, or widens
permissions through AQL.

Every breaking change must:

1. create a new versioned grammar document;
2. update this changelog;
3. add or update golden tests;
4. update API/SDK docs if client-visible errors change.

Non-breaking changes include additive syntax that does not reinterpret v0.4 and
additional diagnostics that preserve existing stable codes and safe messages.
