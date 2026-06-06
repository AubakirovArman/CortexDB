# AQL v0.4 Freeze

AQL v0.4 is the Core Alpha query contract. This document freezes the syntax
and binder behavior that SDKs, CLI, HTTP handlers, and documentation may rely
on. Future syntax changes require a new versioned document and compatibility
tests.

## Statements

One AQL input contains exactly one statement and must end with `;`.

Supported statements:

```sql
RETRIEVE CONTEXT ...;
EXPLAIN RETRIEVE CONTEXT ...;
VERIFY FACT ...;
REMEMBER ...;
```

The parser consumes the full input. Partial consumption is not valid.

## RETRIEVE CONTEXT

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

- `USING MODE <fast|balanced|semantic|audit>`
- `BUDGET <integer> TOKENS`
- `LIMIT <integer> CANDIDATES`
- `WHERE <condition>`
- `REQUIRE <requirements>`

Binder defaults:

- missing `USING MODE` becomes `balanced`;
- missing `BUDGET` becomes `AgentView.default_context_budget_tokens`;
- missing `LIMIT` becomes `AgentView.default_candidate_limit`.

Policy remains fail-closed:

- AQL `WHERE` filters can only narrow `AgentView` visibility;
- unreadable scopes are bind errors;
- invalid retrieval modes are parse errors;
- `budget` and `candidate_limit` may be clamped by policy;
- `audit` requires explicit `AgentView.allow_audit_mode`.

## EXPLAIN

`EXPLAIN` wraps a supported statement. For Core Alpha, the public execution
surface is `EXPLAIN RETRIEVE CONTEXT`.

```sql
EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects;
```

The binder preserves the inner statement semantics. Engine/server explain
output includes:

- selected retrieval mode;
- bitmap bytecode and debug plan;
- policy, liveness, and `WHERE` filters;
- candidate counts for universe, agent-allowed, live, after-bitmap,
  after-quality, and returned-limit stages;
- effective candidate limit, token budget, and citation requirement.

## WHERE

Supported fields:

- `space` and `scope`;
- `status`;
- `type` and `cell_type`;
- `memory_type`.

Supported comparators:

- `=`
- `IN [...]`

The parser recognizes `!=`, `>`, `>=`, `<`, and `<=`, but the v0.4 binder only
executes `=` and `IN` for bitmap filters.

Supported literal forms:

- quoted strings, with escapes `\"`, `\\`, `\n`, `\t`, `\r`;
- identifiers containing ASCII letters, digits after the first character, `_`,
  `-`, and `:`;
- list literals for `IN`, for example `["ready", verified]`.

Operator precedence:

```text
NOT > AND > OR
```

Parentheses are supported in `WHERE`. The maximum condition depth is 32.

## REQUIRE

Supported requirements:

```sql
REQUIRE citations
REQUIRE citations = true
REQUIRE confidence >= 0.80
REQUIRE source_trust >= 0.90
REQUIRE freshness <= 86400 SECONDS
```

Decimal thresholds are deterministic decimal literals. They are converted to
Q16 during binding. Values greater than `1.0` are bind errors, not silent clamps.

Runtime semantics are frozen in
[`AQL_REQUIRE_SEMANTICS.md`](AQL_REQUIRE_SEMANTICS.md): citation requirements
flow into `ContextPack`, while confidence, source-trust, and freshness are hard
candidate filters.

## VERIFY FACT

```sql
VERIFY FACT "Solar Plant budget is approved" IN BRAIN investment_projects;
```

Binding requires:

- `AgentView.allow_verify_fact = true`;
- the target brain must be readable.

Policy denial uses `VerifyFactNotAllowed`, not remember-related errors.

## REMEMBER

```sql
REMEMBER "Use conservative budget assumptions" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;
```

`TTL <integer> SECONDS` is optional.

Binding requires:

- `AgentView.allow_remember = true`;
- target scope must be writable;
- memory type must be allowed;
- TTL must not exceed `AgentView.max_ttl_seconds` when a max is configured.

Supported memory types:

- `decision`
- `preference`
- `workflow_result`
- `error_log`
- `observation`

## Diagnostics

Parse diagnostics expose:

- safe kind;
- `SourceSpan { offset, line, column, len }`;
- safe message.

Stable parse error kinds:

- `Unexpected`
- `ExpectedKeyword`
- `Incomplete`
- `InvalidInteger`
- `InvalidMode`
- `InvalidStringEscape`
- `WhereDepthExceeded`

Bind errors expose stable codes through `BindError::code()` and safe external
messages through `BindError::safe_message()`.

## Compatibility Rules

- Do not remove or reinterpret v0.4 syntax without a new versioned spec.
- Do not widen permissions through AQL syntax or runtime masks.
- Do not introduce float-backed AST thresholds.
- Do not silently clamp semantic thresholds such as confidence or source trust.
- Add golden tests for any grammar, binder, or diagnostic contract change.
