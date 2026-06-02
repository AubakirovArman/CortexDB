# CortexDB Tool Registry

Status: Core Alpha extension, Epic 57 closed by local gate.

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

## Local Gate

Run:

```bash
make tool-registry-check
```

The gate verifies:

- tool registration writes a durable `type=tool` cell;
- tool listing respects `AgentView` scope;
- AQL + ContextPack can include a tool cell;
- an agent without the scope cannot retrieve the tool.

## Non-Goals

- Tool execution.
- Remote credential storage.
- External identity provider integration.
- Enterprise RBAC administration UI.
- Legal-grade tool verification.
