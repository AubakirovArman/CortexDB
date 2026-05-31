# CortexDB Documentation Index

Last audited: 2026-05-31.

This index covers project-owned markdown tracked by git. Dependency,
generated, virtualenv, `node_modules`, and `target` markdown files are excluded.
At the time of this audit, the repository tracks 105 project markdown files.

## Start Here

- [`README.md`](../README.md) - project overview, quickstart, current status.
- [`PROJECT_STATUS.md`](PROJECT_STATUS.md) - honest Core Alpha status.
- [`BETA_DELTA.md`](BETA_DELTA.md) - what remains before beta promotion.
- [`EPIC_EXECUTION_ORDER.md`](EPIC_EXECUTION_ORDER.md) - ordered epic queue and closing rules.
- [`REMAINING_EXECUTION_PLAN.md`](REMAINING_EXECUTION_PLAN.md) - current cycle plan.
- [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) - wording boundaries.

## Architecture

- [`ARCHITECTURE.md`](ARCHITECTURE.md) - crate and subsystem map.
- [`CORE_ENGINE.md`](CORE_ENGINE.md) - engine facade and single-node loop.
- [`CORE_INVARIANTS.md`](CORE_INVARIANTS.md) - safety invariants.
- [`CORE_CONSISTENCY_AUDIT.md`](CORE_CONSISTENCY_AUDIT.md) - consistency checks.
- [`CONSENSUS_SLO.md`](CONSENSUS_SLO.md) - consensus hardening gates and beta SLO targets.
- [`WHY_CORTEXDB.md`](WHY_CORTEXDB.md) - positioning and rationale.
- [`WHY_AGENT_NATIVE_DB.md`](WHY_AGENT_NATIVE_DB.md) - agent-native database concept.

## AQL And Context

- [`AQL_V0_4.md`](AQL_V0_4.md) - current AQL grammar.
- [`AQL_V0_3.md`](AQL_V0_3.md) and [`aql-v0.3.md`](aql-v0.3.md) - historical v0.3 notes.
- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) - Context Pack v1 contract.
- [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md) - Context Pack technology overview.
- [`VERIFY_FACT.md`](VERIFY_FACT.md) - deterministic verification behavior.
- [`FEEDBACK.md`](FEEDBACK.md) - feedback scoring signal.

## Storage And Recovery

- [`ACLOG_FORMAT.md`](ACLOG_FORMAT.md) - WAL format.
- [`WAL_REPLAY.md`](WAL_REPLAY.md) - replay behavior.
- [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md) - storage file formats.
- [`SEGMENT_BUNDLES.md`](SEGMENT_BUNDLES.md) - segment bundle consistency.
- [`ATOMIC_WRITE_AUDIT.md`](ATOMIC_WRITE_AUDIT.md) - atomic write coverage.
- [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) - backup/restore operations.
- [`CRASH_SIMULATION.md`](CRASH_SIMULATION.md) - crash/fault checks.
- [`FAILURE_SCENARIOS.md`](FAILURE_SCENARIOS.md) - failure behavior.
- [`RECOVERY_INVARIANTS.md`](RECOVERY_INVARIANTS.md) - recovery safety.

## API, CLI, SDK

- [`API.md`](API.md) - HTTP API contract.
- [`API_JSON_SCHEMAS.md`](API_JSON_SCHEMAS.md) - response schema examples.
- [`openapi.yaml`](openapi.yaml) - OpenAPI contract.
- [`API_CHANGELOG.md`](API_CHANGELOG.md) - API evolution notes.
- [`API_COMPATIBILITY.md`](API_COMPATIBILITY.md) - compatibility rules.
- [`API_ERROR_TAXONOMY.md`](API_ERROR_TAXONOMY.md) - stable error classes.
- [`AUTH.md`](AUTH.md) - authentication and token policy.
- [`RBAC_POLICY_STORE_DESIGN.md`](RBAC_POLICY_STORE_DESIGN.md) - future dynamic RBAC policy-store design.
- [`CLI.md`](CLI.md) - CLI command reference.
- [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md) - SDK usage.
- [`SDK_RELEASE.md`](SDK_RELEASE.md) - package release procedure.
- [`SDK_DEPRECATION_POLICY.md`](SDK_DEPRECATION_POLICY.md) - SDK deprecation rules.

## Search, ANN, And Benchmarks

- [`SEARCH.md`](SEARCH.md) - lexical/vector/search foundations.
- [`ANN_PRODUCTION_TUNING.md`](ANN_PRODUCTION_TUNING.md) - ANN guardrails and tuning.
- [`ANN_CORPUS_FORMAT.md`](ANN_CORPUS_FORMAT.md) - ANN corpus format.
- [`ANN_PUBLIC_CORPUS_RUNS.md`](ANN_PUBLIC_CORPUS_RUNS.md) - public corpus run policy.
- [`BENCHMARKS.md`](BENCHMARKS.md) - benchmark gates and reports.

## Product Surfaces

- [`DASHBOARD_UI.md`](DASHBOARD_UI.md) - dashboard status and gates.
- [`BINARY_RELEASES.md`](BINARY_RELEASES.md) - binary tarball packaging and install flow.
- [`INGESTION.md`](INGESTION.md) - ingestion behavior.
- [`AGENT_MEMORY.md`](AGENT_MEMORY.md) - local agent memory.
- [`METRICS.md`](METRICS.md) - metrics endpoint and fields.
- [`OPERATIONS.md`](OPERATIONS.md) - operational guidance.
- [`SECURITY_MODEL.md`](SECURITY_MODEL.md) - security model.
- [`SECURITY_THREAT_MODEL.md`](SECURITY_THREAT_MODEL.md) - threat model.

## Planning And Release

Current status should be read from `BETA_DELTA.md` and
`REMAINING_EXECUTION_PLAN.md` first. The other roadmap files are useful
backlogs, snapshots, or deeper planning notes.

- [`CORE_ALPHA.md`](CORE_ALPHA.md) - Core Alpha scope.
- [`CORE_ALPHA_RELEASE_CHECKLIST.md`](CORE_ALPHA_RELEASE_CHECKLIST.md) - release checklist.
- [`RELEASE_NOTES_v0.1.0-core-alpha.md`](RELEASE_NOTES_v0.1.0-core-alpha.md) - release notes.
- [`FUTURE_PRODUCT_LAYERS_PLAN.md`](FUTURE_PRODUCT_LAYERS_PLAN.md) - future product backlog.
- [`EPIC_EXECUTION_ORDER.md`](EPIC_EXECUTION_ORDER.md) - active ordered epic queue.
- [`PL_IMPLEMENTATION_TASKS.md`](PL_IMPLEMENTATION_TASKS.md) - normalized action list from the external `pl.md` audit.
- [`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md) - full epic/task extraction from the external `pl.md` audit.
- [`PRODUCTION_LAYER_NEXT_ACTIONS.md`](PRODUCTION_LAYER_NEXT_ACTIONS.md) - dated production-layer snapshot.
- [`POST_CORE_ALPHA_EXECUTION_PLAN.md`](POST_CORE_ALPHA_EXECUTION_PLAN.md) - milestone execution backlog.
- [`POST_CORE_ALPHA_IMPLEMENTATION_PLAN.md`](POST_CORE_ALPHA_IMPLEMENTATION_PLAN.md) - implementation progress snapshot.
- [`POST_CORE_ALPHA_PRODUCT_PLAN.md`](POST_CORE_ALPHA_PRODUCT_PLAN.md) - product backlog.

## Audit Notes

- Current Context Pack docs are split intentionally:
  - `CONTEXT_PACK.md` is the v1 contract and quality gate.
  - `CONTEXT_PACK_TECHNOLOGY.md` explains the technology and invariants.
- AQL v0.4 is current. v0.3 docs remain historical references.
- Public SDK publication and real-embedding GitHub automation remain beta-stage
  work. Local checks and package dry-runs are the Core Alpha boundary.
