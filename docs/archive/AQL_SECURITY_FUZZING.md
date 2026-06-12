# AQL Security Fuzzing

This gate protects the AQL permission invariant:

```text
AQL WHERE clauses may narrow AgentView visibility, but must never expand it.
```

## Scope

The current fuzz-like suite generates deterministic `WHERE` expressions using:

- predicates over `space`, `scope`, `status`, and `type`;
- `NOT`;
- `AND`;
- `OR`;
- nested parentheses.

It runs the generated corpus against a database containing two scopes:

- `project:alpha`, readable by the test `AgentView`;
- `project:beta`, not readable by the test `AgentView`.

## Expected Outcomes

Each generated query must satisfy one of these outcomes:

1. It succeeds and every returned cell belongs to `project:alpha`.
2. It fails closed with `permission_denied` because the expression explicitly
   references `project:beta`.

Any successful result containing `project:beta` is a scope-bypass bug.
Any unexpected non-permission error is also a regression.

## Persistence Coverage

The same generated corpus is checked twice:

1. against the live MemTable path;
2. after `checkpoint` and reopen, against persisted segment/index state.

This verifies both runtime `AgentAllowed` masks and persisted bitmap index
execution.

## Determinism

The corpus uses a fixed-seed local generator instead of external randomness.
That makes failures reproducible while still covering a broader shape space than
hand-written examples.
