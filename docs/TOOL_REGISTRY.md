# CortexDB Tool Registry

Status: Core Alpha extension. Epic 57 closed the initial registry gate; Epic
143 closes Tool Registry v1 for the production epic plan.

The tool registry stores agent tool descriptions as durable CortexDB cells. It
does not execute tools. Execution, remote credentials, external identity, and
admin RBAC remain separate future product layers.

## Why It Exists

Agent workflows often need to choose a tool before answering a task. CortexDB
keeps tool descriptions in the same retrieval system as facts, memories, and
documents so an agent can ask for relevant tools through AQL and Context Packs.

The current path is:

```text
ToolDescriptor
-> KnowledgeCellType::Tool
-> WAL + MemTable
-> AQL WHERE type = "tool"
-> ContextPack
```

## Epic 143 Tool Registry v1 Contract

Epic 143 turns the earlier registry foundation into a small durable contract:

| Plan task | Core Alpha behavior | Evidence |
| --- | --- | --- |
| Add tool cells | `ToolDescriptor` persists as `KnowledgeCellType::Tool` through WAL and MemTable. | `Database::register_tool` |
| Add permissions | `ToolPermission` stores `read`, `execute`, `write`, and `approval_required` markers. | `ToolDescriptor::to_knowledge_cell` |
| Add input/output schema | Optional `input_schema` and `output_schema` lines are preserved in the tool cell payload. | `ToolDescriptor::from_payload` |
| Add tool retrieval by task | `Database::recommend_tools_for_task(view, task, limit)` ranks visible tool cells by task-term overlap. | `tool_retrieval_by_task_returns_relevant_tool_cell` |

## Cell Contract

Tool cells use normal CortexDB metadata headers:

```text
scope=project:investments
status=ready
type=tool
source=tool-registry

name=calculator
description=Budget calculator for investment-project analysis
permissions=read,execute,approval_required
input_schema={"type":"object","required":["query"]}
output_schema={"type":"object","required":["answer"]}
```

Required fields:

- `scope`
- `name`
- `description`
- `permissions`

Optional fields:

- `input_schema`
- `output_schema`
- `source`

## Permissions

Supported permission markers:

- `read`: the tool metadata can be shown to an agent.
- `execute`: an agent runtime may execute the tool after its own policy checks.
- `write`: the tool may mutate state.
- `approval_required`: execution requires explicit approval outside CortexDB.

CortexDB stores and retrieves these markers. It does not perform external tool
execution in Core Alpha.

## Agent Scope Enforcement

`Database::list_tools(view)` returns only tool cells whose scope appears in
`AgentView.readable_scopes`.

AQL retrieval uses the same runtime `AgentAllowed` mask:

```sql
RETRIEVE CONTEXT
FOR TASK "find a calculator tool"
IN BRAIN default
WHERE scope = project:investments AND type = "tool"
LIMIT 5 CANDIDATES;
```

An agent without `project:investments` cannot retrieve that tool through
`ContextPack`.

## Tool Retrieval By Task

Agents can ask for tools that match the current task without executing them:

```rust
let tools = db.recommend_tools_for_task(
    &agent_view,
    "investment budget analysis",
    5,
);
```

The scorer is intentionally simple in Core Alpha:

- tokenize the task;
- tokenize visible tool name, description, and schemas;
- keep tools with at least one matching term;
- sort by descending match count, then by stable cell id.

This gives deterministic local tool selection for demos, tests, and
ContextPack planning. It is not a production tool-use policy engine.

## Local Gate

Run:

```bash
make tool-registry-check
```

The gate verifies:

- tool registration writes a durable `type=tool` cell;
- tool listing respects `AgentView` scope;
- tool recommendation returns a relevant visible tool for a task;
- AQL + ContextPack can include a tool cell;
- an agent without the scope cannot retrieve the tool.

## Non-Goals

- Tool execution.
- Remote credential storage.
- External identity provider integration.
- Enterprise RBAC administration UI.
- Legal-grade tool verification.
