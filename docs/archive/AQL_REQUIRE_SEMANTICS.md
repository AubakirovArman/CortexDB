# AQL REQUIRE Semantics v1

`REQUIRE` clauses are deterministic retrieval constraints. They are parsed by
AQL, bound into `BoundRetrievePlan`, and enforced by the engine before a cell is
returned or packed into a `ContextPack`.

## Supported Clauses

```sql
REQUIRE citations
REQUIRE citations = true
REQUIRE confidence >= 0.80
REQUIRE source_trust >= 0.90
REQUIRE freshness <= 86400 SECONDS
```

Decimal thresholds are string-backed deterministic literals in the AST and are
converted to Q16 at bind time. Values greater than `1.0` are bind errors.

## Citations

`REQUIRE citations` makes citations mandatory for the retrieval plan:

- `BoundRetrievePlan.context_policy.require_citations = true`;
- `ContextPack.citations_required = true` when created from AQL;
- cells without `source=`, `citation=`, or `source_id=` are selected only with
  a `missing_citation` anomaly.

The clause preserves stricter policy. If the `AgentView` or retrieval mode
already requires citations, AQL cannot disable it.

## Confidence

`REQUIRE confidence >= X` filters candidates by candidate confidence:

- first use SourceRef `confidence_q16` when a SourceRef exists;
- otherwise use `source_trust_q16` as a conservative compatibility fallback;
- otherwise treat confidence as `0`.

The effective minimum is the maximum of `AgentView.min_required_confidence_q16`
and the AQL threshold.

## Source Trust

`REQUIRE source_trust >= X` filters candidates by `source_trust_q16` metadata.
Missing `source_trust_q16` is treated as `0` for this hard filter. Source trust
also remains available as a scoring/explain signal in ContextPack, but the
`REQUIRE` clause is a hard candidate gate.

## Freshness

`REQUIRE freshness <= N SECONDS` filters candidates by age:

```text
age = now_unix_seconds - created_unix_seconds
```

If `created_unix_seconds` is absent, the cell fails the freshness requirement.
Freshness is evaluated at query time.

## Runtime Surfaces

The same bound retrieve plan drives:

- `Database::retrieve_aql`;
- `Database::context_pack_from_aql`;
- `Database::explain_retrieve_aql`.

This keeps direct retrieval, ContextPack generation, and explain output aligned
on the same `REQUIRE` semantics.
