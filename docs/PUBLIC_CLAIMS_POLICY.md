# Public Claims Policy

This policy keeps CortexDB public-facing documentation aligned with the actual
Core Alpha with Beta Foundation evidence state.

## Allowed Claims

- CortexDB is an experimental Core Alpha agent-native context database with
  Beta Foundation evidence.
- CortexDB may describe `v0.2.0-beta.2` as a future local single-node
  developer/API beta target when that statement references `BETA_RELEASE.md`
  or `make beta-release-check`.
- The single-node durable core has repeatable test and release evidence.
- HTTP, CLI, SDK, AQL, ContextPack, backup/restore, and ANN gates exist for the
  documented alpha contract.
- ANN/HNSW, consensus, dashboard, and SDK publication can be described as
  guarded, experimental, blocked, or future product layers when that status is
  explicit.

## Disallowed Claims

Do not describe CortexDB as:

- production-ready;
- enterprise-ready;
- fully production-grade;
- a production distributed consensus database;
- an unrestricted production ANN/HNSW search engine;
- an SLA-backed or benchmark-certified high-performance database.

## Required Qualifiers

Public docs that describe product status must include the relevant qualifier:

- `Core Alpha` for the current release status;
- `Beta Foundation evidence` for beta-preparation status before the beta gate
  passes;
- `not recommended for production workloads` or equivalent for README-level
  positioning;
- `not a production SLA` for API performance or server behavior;
- `experimental`, `guarded`, `future`, or `blocked` for ANN/HNSW, consensus,
  product UI, and SDK publication lifecycle claims.

The release gate is `make public-claims-check`. It writes
`target/public-claims/report.json` and is paired with the release-facing freeze
checklist in [`PUBLIC_CLAIMS_FREEZE.md`](archive/PUBLIC_CLAIMS_FREEZE.md).
