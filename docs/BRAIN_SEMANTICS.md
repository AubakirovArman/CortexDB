# Brain Semantics

Status: single-brain contract with deprecated aliases.

CortexDB does not implement isolated multi-brain namespaces in the current
single-node data model. The only real brain namespace is:

```text
default = BrainId(1)
```

Existing AQL syntax still requires `IN BRAIN <identifier>` for compatibility.
For v0.5, every non-empty brain identifier resolves to `BrainId(1)`. Names such
as `investment_projects` are deprecated aliases, not isolated storage,
permission, statistics, or index namespaces.

## Product Decision

B20 chooses removal/simplification instead of partial multi-brain
implementation.

Reasons:

- cell descriptors and persisted indexes do not store `brain_id`;
- scopes are already the enforced security namespace;
- adding real multi-brain semantics would require a storage/catalog migration,
  per-brain index partitions, per-brain stats, and backup/restore compatibility;
- keeping arbitrary names undocumented makes AQL promise isolation that the
  engine cannot provide.

## Current Contract

- `default` is the only non-deprecated brain name.
- `AgentView.readable_brains` must include `BrainId(1)` for AQL retrieve and
  verify operations.
- Non-default AQL brain names are accepted only as deprecated aliases for
  `BrainId(1)`.
- No query, ContextPack, VERIFY, graph, memory, tool, or statistics path may
  claim cross-brain isolation.
- Use scopes and tenants for isolation.

## Migration Plan

Before v1.0, callers should rewrite AQL from:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects;
```

to:

```sql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default;
```

Agent views should keep `readable_brains = {1}` and express product boundaries
through `readable_scopes`, `writable_scopes`, and tenant routing.

For v1.0 there are two compatible exits:

- make `IN BRAIN` optional and default it to `default`; or
- keep the clause but reject non-`default` names with `UnknownBrain`.

Real multi-brain may be reopened only as a future storage-format epic with:

- `brain_id` in typed descriptors and WAL/segment metadata;
- a persisted brain catalog;
- scope uniqueness rules inside each brain;
- per-brain bitmap/vector/lexical/statistics partitions;
- backup/restore and migration fixtures for old single-brain data.
