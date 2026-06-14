# Knowledge Graph/Provenance Index Contract

Status: `EPIC-B18` + `EPIC-C15` contract.

CortexDB keeps graph/provenance records as typed cells and maintains an
incremental in-memory graph index. The graph is a lookup structure for
retrieval and verification; it is not a graph database query language.

## Typed Records

Entity records use `type=entity`:

```text
scope=project:investments
status=ready
type=entity
source_id=ifc:solar-001

name=Solar Plant
kind=project
```

Relation records use `type=relation`:

```text
scope=project:investments
status=ready
type=relation

subject=Solar Plant
predicate=located_in
object=Kazakhstan
```

`GraphEdgeKind` normalizes stable predicates:

- `source_supports_fact`
- `fact_contradicts_fact`

source references are indexed from descriptor-backed metadata. Descriptor
scope/type/source fields are authoritative over legacy payload headers.

## Index Contract

`GraphIndexStore` is the maintained catalog. It updates incrementally on put,
patch, tombstone, checkpoint reopen, replication snapshot rebuild, and lazy
open-time rebuild.

The maintained `KnowledgeGraphIndex` contains:

- entity name -> entity cells
- interned entity ids plus edge ids for compact adjacency traversal
- relation cell id -> relation edge
- edge kind -> relation edges
- `source_support_edges_by_fact` for VERIFY source-support lookup
- source id -> cell ids
- tool cells for legacy graph tooling

Bulk graph-index builds use an add-only path and canonicalize the index once at
the end. Incremental put/patch/tombstone updates still remove the previous cell
projection before inserting the new one.

Graph APIs do not rebuild from visible payloads at query time:

- `graph_entities`
- `graph_neighbors`
- `graph_cells_for_source`
- `graph_source_supports_fact_edges`
- `graph_fact_contradicts_fact_edges`

## Graph Retrieval

`Database::graph_retrieve_related` returns `GraphRetrievalHit` rows by walking
the maintained adjacency index with the default visit budget. Each hit includes:

- matched cell id
- matched entity
- hop depth
- `proximity_score_q16`
- `explaining_edges`

`Database::graph_retrieve_related_with_budget` returns `GraphRetrievalReport`
with the same hits plus:

- `visited_entities`
- `visited_edges`
- `visit_budget`
- `budget_exceeded`

The C15 performance gate is `make graph-index-performance-check`; it records
100K-node graph traversal p95 in `target/graph-index-performance/report.json`.

## VERIFY Source-Support

VERIFY source-support enrichment reads `source_support_edges_by_fact` for the
current evidence cell ids. It only materializes payload for matching readable
relation edges so unrelated relation cells are not scanned during verification.

## Gate

Run:

```bash
make knowledge-graph-check
make graph-index-performance-check
```

The gate checks this contract, incremental graph store markers, graph retrieval
fixtures, VERIFY source-support indexing, lazy reopen behavior, and static
guards against graph query-time payload scans.
