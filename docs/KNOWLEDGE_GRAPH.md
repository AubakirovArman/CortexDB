# CortexDB Knowledge Graph Layer

Status: Core Alpha extension, Epic 58 closed by local gate.

CortexDB builds a lightweight knowledge-graph projection from visible typed
cells. The graph is not a separate storage backend. It is derived from the same
WAL, MemTable, checkpoint, and segment path used by normal cells.

## Indexed Cells

The snapshot graph index currently covers:

- `type=entity`: grouped by exact `name`.
- `type=relation`: adjacency by `subject` and `object`.
- source references: cells grouped by `source`, `source_id`, or `citation`.

Example entity cell:

```text
scope=project:investments
status=ready
type=entity
source=wb:solar-001

name=Solar Plant
kind=project
```

Example relation cell:

```text
scope=project:investments
status=ready
type=relation

subject=Solar Plant
predicate=located_in
object=Kazakhstan
```

## API Surface

Embedded engine helpers:

```rust
let index = db.knowledge_graph_index();
let entities = index.entities_named("Solar Plant");
let neighbors = index.neighbors("Solar Plant");
let source_cells = index.cells_for_source("wb:solar-001");
```

Database shortcuts:

```rust
db.graph_entities("Solar Plant");
db.graph_neighbors("Solar Plant");
db.graph_cells_for_source("wb:solar-001");
```

## Persistence Boundary

Because the graph index is built from visible cells, it works before and after:

- WAL replay;
- checkpoint;
- reopen;
- compact, as long as the underlying typed cells remain visible.

The current graph is rebuilt on demand from the local snapshot. Persisted graph
index files and multi-hop query planning remain future work.

## Local Gate

Run:

```bash
make knowledge-graph-check
```

The gate verifies:

- relation traversal by subject and object;
- entity lookup by exact name;
- source-ref grouping;
- checkpoint and reopen behavior.

## Non-Goals

- Graph query language.
- Persisted graph index files.
- Entity extraction from natural language.
- Multi-hop ranking.
- Distributed graph traversal.
