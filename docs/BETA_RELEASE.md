# CortexDB Beta Release Boundary

Status: beta scope frozen for the `v0.2.0-beta.1` target. Current public
status remains **Core Alpha with Beta Foundation evidence** until
`make beta-release-check` passes and the beta evidence bundle is attached to a
release.

## Beta Definition

CortexDB Beta is a local, single-node agent-native context database with stable
HTTP, CLI, SDK, AQL, ContextPack, verification, retrieval-quality, backup, and
security evidence for developer adoption.

The beta target is not a distributed database release and is not an enterprise
compliance release. It is a reproducible developer/API beta for local durable
context storage and retrieval workflows.

## Target Version

- Target tag: `v0.2.0-beta.1`
- Current state: Core Alpha with Beta Foundation evidence
- Promotion gate: `make beta-release-check`
- Evidence root: `target/beta-release/`

## Stable For Beta

- Single-node durable write path: WAL, MemTable MVCC, checkpoint, compact, and
  restart recovery.
- Typed HTTP API with OpenAPI contract checks and stable error taxonomy.
- CLI and SDK flows for health, put/get, search, AQL, ContextPack, Verify,
  Remember, ingest, tenant routing, auth errors, stats, and validate.
- ContextPack v1 evidence packing with citations, token budgets, deterministic
  order, anomaly reporting, and stable prompt/Markdown exports.
- Deterministic `VERIFY FACT` quality fixtures for supported, contradicted,
  mixed, and insufficient verdicts.
- Guarded lexical, vector, hybrid, and ANN retrieval with exact fallback.
- Backup, restore, validation, repair, audit redaction, auth, tenant, CORS,
  rate-limit, and body-limit gates.

## Experimental Or Guarded In Beta

- ANN/HNSW remains guarded by recall/latency gates and exact fallback.
- Real-domain embedding benchmarks remain local/manual until stable beta
  environments are selected.
- Dashboard remains a developer/operations console, not a full product UI.
- Replication and consensus remain experimental local hardening evidence, not
  production distributed consensus.
- Built-in LLM inference remains a disabled-by-default deterministic test
  double plus safety prerequisites, not a production model runtime.

## Explicit Non-Goals For Beta

- Production distributed consensus.
- Managed cloud service.
- Enterprise RBAC/compliance certification.
- Legal-grade verification or legal advice.
- Unrestricted production HNSW without exact fallback.
- Built-in production LLM inference.
- External identity provider production rollout.
- Performance or availability SLA claims.

## Required Beta Evidence

`make beta-release-check` must prove:

1. SDK e2e works for Rust, Python, and TypeScript.
2. HTTP API responses validate against OpenAPI.
3. ContextPack quality gate passes.
4. `VERIFY FACT` quality gate passes.
5. Retrieval quality gate passes on endpoint-backed investment-project evidence
   and the multi-domain beta fixture report.
6. Security beta gate passes.
7. Backup/restore and tenant recovery gates pass.
8. Beta operations runbook covers install, run, auth, tenant, backup, restore,
   validate, repair, upgrade, rollback, metrics, logs, and known limits.
9. Demo smoke path is reproducible.
10. Release evidence bundle is created and includes local binary package
    validation artifacts.
11. Public documentation keeps distributed/cloud/enterprise/legal/LLM claims
    bounded as future or experimental work.

## Promotion Rule

Do not change README wording from Core Alpha with Beta Foundation evidence to a
Beta release until:

```bash
make beta-release-check
```

passes from a clean checkout and the generated `target/beta-release/` report is
included in the release artifacts.
