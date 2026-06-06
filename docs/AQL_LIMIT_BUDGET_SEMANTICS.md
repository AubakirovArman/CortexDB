# AQL LIMIT And BUDGET Semantics v1

`LIMIT ... CANDIDATES` and `BUDGET ... TOKENS` are retrieval policy controls.
They are parsed by AQL, clamped by `AgentView`, and then used by the engine
runtime.

## Candidate Limit

```sql
LIMIT 10 CANDIDATES
```

The effective candidate limit is:

```text
min(requested_limit_or_agent_default, AgentView.max_candidate_limit)
```

This limit is a hard upper bound on cells returned by `Database::retrieve_aql`.
`Database::context_pack_from_aql` receives the same bounded candidate stream, so
the number of packed cells can never exceed the effective candidate limit.

If `LIMIT` is absent, AQL uses `AgentView.default_candidate_limit`.

## ContextPack Cell Limit

AQL v1 has no separate final `LIMIT ... CELLS` clause. The ContextPack cell
count is bounded by:

- the effective candidate limit;
- token budget truncation;
- optional redundancy reduction in `ContextPackOptions`.

This keeps candidate retrieval and packing deterministic without introducing a
second result-count contract.

## Token Budget

```sql
BUDGET 12000 TOKENS
```

The effective AQL budget is:

```text
min(requested_budget_or_agent_default, AgentView.max_context_budget_tokens)
```

When `ContextPackOptions.token_budget_tokens == 0`, `context_pack_from_aql`
uses the effective AQL budget.

When `ContextPackOptions.token_budget_tokens > 0`, the explicit runtime option
is used instead and is still clamped by `AgentView.max_context_budget_tokens`.
This gives CLI/server/internal callers a deliberate override while keeping the
default AQL path faithful to the query.

## Explain Output

`EXPLAIN RETRIEVE CONTEXT` reports:

- `candidate_limit`;
- `budget_tokens`;
- `candidate_counts.returned_limit`.

These values describe the same effective plan used by direct retrieval and
ContextPack generation.
