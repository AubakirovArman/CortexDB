# AQL v0.3

This document is retained for historical reference. The current Core Alpha AQL
contract is frozen in [`AQL_V0_4.md`](AQL_V0_4.md).

This document describes the currently implemented AQL surface.

## RETRIEVE CONTEXT

```sql
RETRIEVE CONTEXT
FOR TASK "compare project budgets"
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
LIMIT 500 CANDIDATES
WHERE space = project:investments AND status = "ready"
REQUIRE citations, confidence >= 0.80, source_trust >= 0.90, freshness <= 86400 SECONDS;
```

`USING MODE`, `BUDGET`, `LIMIT`, `WHERE`, and `REQUIRE` are optional.

Default binder values:

- mode: `balanced`
- budget: `AgentView.default_context_budget_tokens`
- candidate limit: `AgentView.default_candidate_limit`

## EXPLAIN

`EXPLAIN RETRIEVE CONTEXT` is supported by the AQL parser and has a working
executor that returns a JSON breakdown of the retrieval plan:

```sql
EXPLAIN RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects;
```

The server exposes this via `POST /v1/aql?scope=<scope>` when the request body
starts with `EXPLAIN RETRIEVE CONTEXT`.

Supported retrieval modes:

- `fast`
- `balanced`
- `semantic`
- `audit`

## WHERE

Supported predicates currently use `=` with string or identifier literals:

```sql
WHERE space = project:investments AND status = "ready"
```

`IN` is supported for list literals:

```sql
WHERE status IN ["ready", "verified"]
```

Supported filter fields:

- `space` / `scope`
- `status`
- `type` / `cell_type`
- `memory_type`

Operator precedence:

```text
NOT > AND > OR
```

Identifiers may contain ASCII letters, numbers, `_`, `-`, and `:` after the first character.

## REQUIRE

Supported requirements:

```sql
REQUIRE citations
REQUIRE citations = true
REQUIRE confidence >= 0.80
REQUIRE source_trust >= 0.90
REQUIRE freshness <= 86400 SECONDS
```

Decimal thresholds are deterministic decimal literals. Values above `1.0` are bind errors.

## VERIFY FACT

```sql
VERIFY FACT "budget is approved" IN BRAIN investment_projects;
```

## REMEMBER

```sql
REMEMBER "use conservative budget" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;
```

`TTL` is optional.
