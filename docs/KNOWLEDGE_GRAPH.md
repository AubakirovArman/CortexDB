# B18 Knowledge Graph/Provenance Index Contract

Status: `EPIC-B18` contract.

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
- entity name -> adjacent relation edges
- edge kind -> relation edges
- `source_support_edges_by_fact` for VERIFY source-support lookup
- source id -> cell ids
- tool cells for legacy graph tooling

Graph APIs do not rebuild from visible payloads at query time:

- `graph_entities`
- `graph_neighbors`
- `graph_cells_for_source`
- `graph_source_supports_fact_edges`
- `graph_fact_contradicts_fact_edges`

## Graph Retrieval

`Database::graph_retrieve_related` returns `GraphRetrievalHit` rows by walking
the maintained adjacency index. Each hit includes:

- matched cell id
- matched entity
- hop depth
- `proximity_score_q16`
- `explaining_edges`

## VERIFY Source-Support

VERIFY source-support enrichment reads `source_support_edges_by_fact` for the
current evidence cell ids. It only materializes payload for matching readable
relation edges so unrelated relation cells are not scanned during verification.

## Gate

Run:

```bash
make knowledge-graph-check
```

The gate checks this contract, incremental graph store markers, graph retrieval
fixtures, VERIFY source-support indexing, lazy reopen behavior, and static
guards against graph query-time payload scans.
