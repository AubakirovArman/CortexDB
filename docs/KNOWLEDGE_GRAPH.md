# CortexDB Knowledge Graph Layer

Status: Core Alpha extension. Epic 58 introduced the layer; Epic 145 closes the
v1 edge contract by separating provenance and contradiction relation classes.

CortexDB builds a lightweight knowledge-graph projection from visible typed
cells. The graph is not a separate storage backend. It is derived from the same
WAL, MemTable, checkpoint, and segment path used by normal cells.

## Indexed Cells

The snapshot graph index currently covers:

- `type=entity`: grouped by exact `name`.
- `type=relation`: adjacency by `subject` and `object`.
- source references: cells grouped by `source`, `source_id`, or `citation`.
- `source_supports_fact` relation edges: source-to-fact provenance links.
- `fact_contradicts_fact` relation edges: fact-to-fact contradiction links.

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

Example source-supports-fact edge:

```text
scope=project:investments
status=ready
type=relation

subject=source:ifc:solar-001
predicate=source_supports_fact
object=cell:42
```

Example fact-contradicts-fact edge:

```text
scope=project:investments
status=ready
type=relation

subject=cell:42
predicate=fact_contradicts_fact
object=cell:43
```

## Epic 145 Knowledge Graph Layer v1 Contract

Epic 145 is closed when the local graph projection exposes all four v1 graph
building blocks:

- Entity cells: `type=entity` plus body fields such as `name=` and `kind=`.
- Relation cells: `type=relation` plus `subject=`, `predicate=`, and `object=`.
- Source-supports-fact edges: `GraphEdgeKind::SourceSupportsFact` from
  predicates such as `source_supports_fact`.
- Fact-contradicts-fact edges: `GraphEdgeKind::FactContradictsFact` from
  predicates such as `fact_contradicts_fact` or the legacy `contradicts` alias.

The edge kind is intentionally stored on each `GraphEdge`. That keeps higher
layers from treating evidence provenance, contradiction, and ordinary relation
edges as the same thing.

## API Surface

Embedded engine helpers:

```rust
let index = db.knowledge_graph_index();
let entities = index.entities_named("Solar Plant");
let neighbors = index.neighbors("Solar Plant");
let source_cells = index.cells_for_source("wb:solar-001");
let support_edges = index.source_supports_fact_edges();
let contradiction_edges = index.fact_contradicts_fact_edges();
```

Database shortcuts:

```rust
db.graph_entities("Solar Plant");
db.graph_neighbors("Solar Plant");
db.graph_cells_for_source("wb:solar-001");
db.graph_source_supports_fact_edges();
db.graph_fact_contradicts_fact_edges();
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
- `GraphEdgeKind` classification for provenance and contradiction relations;
- `graph_source_supports_fact_edges`;
- `graph_fact_contradicts_fact_edges`;
- checkpoint and reopen behavior.

## Non-Goals

- Graph query language.
- Persisted graph index files.
- Entity extraction from natural language.
- Multi-hop ranking.
- Production GraphRAG planning.
- Distributed graph traversal.
