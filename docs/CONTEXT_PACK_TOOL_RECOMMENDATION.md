# CortexDB ContextPack Tool Recommendations

Status: Core Alpha extension, Epic 144 closed by local gate.

ContextPack normally returns selected knowledge cells for an agent task. Tool
recommendations add a second, typed list of relevant tool descriptions beside
that context so an agent can decide which external capability may help.

## Why It Exists

Many agent tasks need both evidence and an action surface. For example, an
investment analyst may need facts about budget variance and a calculator tool
that can check financial exposure. CortexDB keeps the two pieces connected:

```text
AQL task
-> ContextPack evidence cells
-> visible ToolDescriptor cells
-> tool recommendations with matched terms and why_selected
```

## API Contract

Use:

```rust
let result = db.context_pack_with_tool_recommendations_from_aql(
    aql,
    &agent_view,
    ContextPackOptions::default(),
    5,
)?;
```

The result shape is:

```text
ContextPackWithTools {
  pack: ContextPack,
  tool_recommendations: Vec<ToolRecommendation>,
}
```

Each `ToolRecommendation` includes:

- the durable `RegisteredTool`;
- `matched_terms`;
- deterministic `score`;
- `why_selected`.

## Selection Rules

Tool recommendation is deliberately simple in Core Alpha:

1. Build the normal ContextPack from the AQL query.
2. Read the bound AQL task text.
3. Call `Database::recommend_tools_for_task(view, task, tool_limit)`.
4. Return only tools visible through the same `AgentView`.
5. Explain each recommendation by listing matched task terms.

This means an agent gets facts and relevant tool metadata together without
granting tool execution rights.

## Local Gate

Run:

```bash
make context-pack-tool-recommendation-check
```

The gate verifies:

- ContextPack evidence still returns normal cells;
- relevant visible tools are returned beside the pack;
- tool recommendation explanations expose matched task terms;
- recommendation limit is respected;
- tools outside the agent scope are excluded.

## Non-Goals

- Executing tools.
- Storing API keys or remote credentials.
- Enterprise RBAC administration.
- Legal-grade tool suitability verification.
- LLM-driven tool planning.
