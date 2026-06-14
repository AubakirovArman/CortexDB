# B17 Tool Registry Contract

Status: `EPIC-B17` contract.

CortexDB stores agent tools as typed catalog records. A tool record is a durable
`KnowledgeCellType::Tool` cell whose descriptor is authoritative for scope,
status, type, and source. The payload body carries structured tool fields.

## Typed Catalog Record

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

Supported permissions:

- `read`
- `execute`
- `write`
- `approval_required`

`ToolDescriptor` validates this contract and serializes it through
`KnowledgeCellType::Tool`.

## Index Contract

`ToolIndex` is the maintained in-memory catalog. It is updated on put, patch,
tombstone, checkpoint reopen, replication snapshot rebuild, and lazy-payload
open-time rebuild.

Catalog queries do not scan visible cells:

- `Database::list_tools(view)` reads `ToolIndex` and filters by
  `AgentView.readable_scopes`.
- `Database::recommend_tools_for_task(view, task, limit)` reads the term index
  maintained inside `ToolIndex`.

The term index tokenizes tool name, description, `input_schema`, and
`output_schema`. Recommendations are sorted by descending matched-term count,
then stable `CellId`.

## Agent Example

```rust
let pack = db.context_pack_from_aql(aql, &agent_view, options)?;
let tools = db.recommend_tools_for_task(
    &agent_view,
    "investment budget analysis",
    5,
);
```

The agent receives both context cells and visible tool catalog entries. CortexDB
does not execute tools, store remote credentials, or grant external identity.

## Gate

Run:

```bash
make tool-registry-check
```

The gate checks the contract doc, indexed implementation markers, scoped lookup
tests, patch/tombstone term-index tests, and lazy reopen behavior.
