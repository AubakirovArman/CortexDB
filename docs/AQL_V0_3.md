# AQL v0.3

`RETRIEVE CONTEXT` is the implemented end-to-end query path.

```sql
RETRIEVE CONTEXT
FOR TASK "compare project budgets"
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
LIMIT 500 CANDIDATES
WHERE space = project:investments AND status IN ["ready", "verified"]
REQUIRE citations, confidence >= 0.80, source_trust >= 0.90, freshness <= 86400 SECONDS;
```

Optional clauses:

- `USING MODE`
- `BUDGET`
- `LIMIT`
- `WHERE`
- `REQUIRE`

Defaults are supplied by the binder from `AgentView`.

## WHERE

Supported fields:

- `space` / `scope`
- `status`
- `type` / `cell_type`
- `memory_type`

Supported precedence:

```text
NOT > AND > OR
```

Supported list predicate:

```sql
status IN ["ready", "verified"]
```

## REQUIRE

Supported requirements:

- `citations`
- `citations = true`
- `confidence >= 0.80`
- `source_trust >= 0.90`
- `freshness <= 86400 SECONDS`

Decimal thresholds are parsed deterministically and values above `1.0` are bind
errors.
