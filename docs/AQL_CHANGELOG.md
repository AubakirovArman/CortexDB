# AQL Changelog

Unified CortexDB public-surface versioning rules are defined in
[`VERSIONING_POLICY.md`](VERSIONING_POLICY.md). This changelog covers AQL
grammar and diagnostic compatibility.

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

## AQL v0.4 Grammar Change Registry

The machine-readable registry is:

```text
fixtures/aql/grammar_change_registry_v1.json
```

Every grammar or binder compatibility change must appear in this changelog
with a stable `change_id`, at least one SQL example, and a test reference in
the registry. The local policy gate is:

```bash
python3 scripts/check_aql_changelog_policy.py
```

### aql-v0.4-retrieve-context

Classification: statement.

`RETRIEVE CONTEXT` is the primary query statement. It requires a task and
brain target and can be narrowed by optional retrieval clauses.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "compare budgets" IN BRAIN investment_projects;
```

Test reference: `crates/cortex-aql/tests/aql_v0_4_golden_tests.rs`.

### aql-v0.4-explain-retrieve

Classification: statement.

`EXPLAIN RETRIEVE CONTEXT` is the public explain surface for AQL v0.4. It
preserves the inner retrieve semantics while exposing plan diagnostics.

Example:

```sql
EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects;
```

Test reference: `crates/cortex-aql/tests/aql_v0_4_golden_tests.rs`.

### aql-v0.4-verify-fact

Classification: statement.

`VERIFY FACT` parses a claim against a brain and binds through verify-specific
policy checks.

Example:

```sql
VERIFY FACT "Mirny wind farm has a 600 MWh battery" IN BRAIN investment_projects;
```

Test reference: `crates/cortex-aql/tests/aql_v0_4_golden_tests.rs`.

### aql-v0.4-remember

Classification: statement.

`REMEMBER` writes policy-checked memory into a target scope with a memory type
and optional TTL.

Example:

```sql
REMEMBER "Use conservative budget assumptions" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;
```

Test reference: `crates/cortex-aql/tests/aql_v0_4_golden_tests.rs`.

### aql-v0.4-retrieve-options

Classification: clause.

`USING MODE`, `BUDGET`, and `LIMIT CANDIDATES` are optional retrieve clauses.
Binder defaults and policy clamps are part of the compatibility contract.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects USING MODE balanced BUDGET 12000 TOKENS LIMIT 500 CANDIDATES;
```

Test reference: `crates/cortex-aql/tests/aql_stabilization_tests.rs`.

### aql-v0.4-where-precedence

Classification: where.

`WHERE` boolean precedence is frozen as `NOT > AND > OR`.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE NOT status = "ready" AND space = project:investments OR status = "verified";
```

Test reference: `crates/cortex-aql/tests/parser_tests.rs`.

### aql-v0.4-where-in-list

Classification: where.

`IN [...]` list literals are supported for bitmap filters.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE status IN ["ready", "verified"];
```

Test reference: `crates/cortex-aql/tests/binder_hardening_tests.rs`.

### aql-v0.4-require-thresholds

Classification: clause.

`REQUIRE` supports citation, confidence, source-trust, and freshness
constraints.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects REQUIRE citations, confidence >= 0.80, source_trust >= 0.90, freshness <= 86400 SECONDS;
```

Test reference: `crates/cortex-aql/tests/aql_stabilization_tests.rs`.

### aql-v0.4-stable-bind-errors

Classification: diagnostic.

Client-visible bind error classes distinguish invalid AQL, permission denial,
unknown fields, and unsupported comparator/operator cases.

Example:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE status != "ready";
```

Test reference: `crates/cortex-server/src/tests/snapshot_api_tests.rs`.

## Change Policy

A breaking change is any change that removes or reinterprets v0.4 syntax,
changes stable parse error kinds, changes stable bind error codes, or widens
permissions through AQL.

Every grammar or binder compatibility change must:

1. update `fixtures/aql/grammar_change_registry_v1.json`;
2. update this changelog;
3. include a SQL example in this changelog;
4. create a new versioned grammar document for breaking grammar changes;
5. add or update golden tests;
6. update API/SDK docs if client-visible errors change.

Non-breaking changes include additive syntax that does not reinterpret v0.4 and
additional diagnostics that preserve existing stable codes and safe messages.
