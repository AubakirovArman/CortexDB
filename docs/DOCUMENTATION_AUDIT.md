# Documentation Audit

Audit date: 2026-05-31.

## Scope

The audit inspected markdown files tracked by git:

```bash
git ls-files '*.md'
```

Result: 95 project markdown files.

The audit intentionally excluded dependency, generated, virtualenv,
`node_modules`, and `target` markdown files.

## Main Finding

Context Pack documentation existed before this audit, but it was split
implicitly across README, architecture notes, API schemas, and
`CONTEXT_PACK.md`.

The repository now has an explicit Context Pack documentation pair:

- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) - v1 contract, invariants, API shape, and quality gate.
- [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md) - technology overview, pipeline, security model, budget/citation/redundancy behavior, and boundaries.

## Stale Items Fixed In This Pass

- API and cell metadata docs no longer call the released Core Alpha contract a
  "candidate".
- The post-Core Alpha implementation status no longer describes the closed
  layer as "production-ready".
- The post-Core Alpha plan now separates local/tag-gated SDK package checks from
  future public registry publication.
- The real-domain embedding section now reflects the local endpoint-backed
  `investment-projects-v1` baseline and keeps GitHub automation deferred until
  beta.
- The agent-native overview no longer claims CortexDB is already a production
  memory layer.
- README and architecture docs now link to the dedicated Context Pack docs and
  this documentation index.
- Roadmap mirror docs now state that `BETA_DELTA.md` and
  `REMAINING_EXECUTION_PLAN.md` are the current status sources.

## Current Source-Of-Truth Docs

- Overall status: [`PROJECT_STATUS.md`](PROJECT_STATUS.md)
- Beta delta: [`BETA_DELTA.md`](BETA_DELTA.md)
- Public claims: [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md)
- Current cycle: [`REMAINING_EXECUTION_PLAN.md`](REMAINING_EXECUTION_PLAN.md)
- Docs map: [`DOCUMENTATION_INDEX.md`](DOCUMENTATION_INDEX.md)
- Context Pack technology: [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md)

## Known Documentation Debt

- `AQL_V0_3.md` and `aql-v0.3.md` are historical v0.3 references while
  `AQL_V0_4.md` is current.
- Several roadmap files intentionally overlap, but now carry status-source
  notices. `BETA_DELTA.md` and `REMAINING_EXECUTION_PLAN.md` should be treated
  as the current public status.
- Public SDK publishing docs describe the procedure, but real registry
  publication remains beta-stage and credential-dependent.
- Long-running ANN/HNSW production traffic history remains future work; current
  real-domain embedding evidence is local-only for Core Alpha.

## Recommended Recurring Check

Run this when changing public-facing docs:

```bash
make beta-delta-check
make public-claims-check
make openapi-contract-check
```
