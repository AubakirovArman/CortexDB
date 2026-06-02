# CortexDB Documentation Index

Last audited: 2026-06-01.

This index covers project-owned markdown. Dependency, generated, virtualenv,
`node_modules`, and `target` markdown files are excluded.

## Start Here

- [`README.md`](../README.md) - project overview, quickstart, current status.
- [`PROJECT_STATUS.md`](PROJECT_STATUS.md) - honest Core Alpha status.
- [`BETA_RELEASE.md`](BETA_RELEASE.md) - `v0.2.0-beta.1` scope, non-goals, and promotion gate.
- [`BETA_LANDING.md`](BETA_LANDING.md) - concise external beta landing path.
- [`USE_CASE_PACKS.md`](USE_CASE_PACKS.md) - runnable legal, financial, and technical beta scenarios.
- [`CONTRIBUTOR_ONBOARDING.md`](CONTRIBUTOR_ONBOARDING.md) - 15-minute first contributor path.
- [`GOOD_FIRST_ISSUES.md`](GOOD_FIRST_ISSUES.md) - bounded starter issue map.
- [`PUBLIC_BENCHMARKS.md`](PUBLIC_BENCHMARKS.md) - release-by-release public benchmark history.
- [`COMPARISONS.md`](COMPARISONS.md) - neutral comparison with SQL databases, vector databases, RAG stacks, and memory frameworks.
- [`SDK_PRODUCTIZATION.md`](SDK_PRODUCTIZATION.md) - local Rust, Python, and TypeScript SDK productization gate.
- [`TOOL_REGISTRY.md`](TOOL_REGISTRY.md) - durable tool cells, schemas, permissions, and ContextPack inclusion.
- [`KNOWLEDGE_GRAPH.md`](KNOWLEDGE_GRAPH.md) - entity, relation, and source-reference graph projection.
- [`BETA_OPERATIONS.md`](BETA_OPERATIONS.md) - beta operator runbook for install, auth, backup, validation, repair, metrics, and known limits.
- [`RELEASE_NOTES_v0.2.0-beta.1.md`](RELEASE_NOTES_v0.2.0-beta.1.md) - planned beta release notes and checklist.
- [`BETA_DELTA.md`](BETA_DELTA.md) - what remains before beta promotion.
- [`BETA_FOUNDATION_EVIDENCE.md`](BETA_FOUNDATION_EVIDENCE.md) - local Epic 2 evidence gate.
- [`BETA_RC_EVIDENCE.md`](BETA_RC_EVIDENCE.md) - local Epic 3 evidence gate.
- [`PRODUCTION_HARDENING_EVIDENCE.md`](PRODUCTION_HARDENING_EVIDENCE.md) - local Epic 4 evidence gate.
- [`PRODUCTION_CANDIDATE_EVIDENCE.md`](PRODUCTION_CANDIDATE_EVIDENCE.md) - local Epic 5 evidence gate.
- [`PRODUCTION_V1.md`](PRODUCTION_V1.md) - local single-node Production v1.0 boundary.
- [`PRODUCTION_V1_EVIDENCE.md`](PRODUCTION_V1_EVIDENCE.md) - local Epic 6 evidence gate.
- [`PUBLIC_CLAIMS_FREEZE.md`](PUBLIC_CLAIMS_FREEZE.md) - public wording freeze for local single-node claims.
- [`EPIC_EXECUTION_ORDER.md`](EPIC_EXECUTION_ORDER.md) - ordered epic queue and closing rules.
- [`NEXT_60_EPICS.md`](NEXT_60_EPICS.md) - normalized next-stage backlog extracted from the latest external `pl.md`.
- [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) - future epics extracted from current non-goals.
- [`REMAINING_EXECUTION_PLAN.md`](REMAINING_EXECUTION_PLAN.md) - current cycle plan.
- [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) - wording boundaries.

## Architecture

- [`ARCHITECTURE.md`](ARCHITECTURE.md) - crate and subsystem map.
- [`CORE_ENGINE.md`](CORE_ENGINE.md) - engine facade and single-node loop.
- [`ENGINE_API.md`](ENGINE_API.md) - stable embedded engine API boundary.
- [`MODULE_OWNERSHIP.md`](MODULE_OWNERSHIP.md) - crate/module ownership map.
- [`GOOD_FIRST_ISSUES.md`](GOOD_FIRST_ISSUES.md) - suggested starter task classes and required gates.
- [`CORE_INVARIANTS.md`](CORE_INVARIANTS.md) - safety invariants.
- [`CORE_CONSISTENCY_AUDIT.md`](CORE_CONSISTENCY_AUDIT.md) - consistency checks.
- [`CONSENSUS_SLO.md`](CONSENSUS_SLO.md) - consensus hardening gates and beta SLO targets.
- [`DISTRIBUTED_CONSENSUS_DESIGN.md`](DISTRIBUTED_CONSENSUS_DESIGN.md) - future production distributed consensus design.
- [`DISTRIBUTED_CONSENSUS_RESEARCH.md`](DISTRIBUTED_CONSENSUS_RESEARCH.md) - local research evidence gates without production HA claims.
- [`MANAGED_CLOUD_FEASIBILITY.md`](MANAGED_CLOUD_FEASIBILITY.md) - local prerequisites and non-claims for a future hosted service.
- [`WHY_CORTEXDB.md`](WHY_CORTEXDB.md) - positioning and rationale.
- [`WHY_AGENT_NATIVE_DB.md`](WHY_AGENT_NATIVE_DB.md) - agent-native database concept.
- [`COMPARISONS.md`](COMPARISONS.md) - adjacent-stack comparison without replacement claims.
- [`RAG_VS_CORTEXDB.md`](RAG_VS_CORTEXDB.md) - practical comparison with classic RAG.
- [`RAG_VS_CORTEXDB_DEMO.md`](RAG_VS_CORTEXDB_DEMO.md) - beta product demo scenario and expected output contract.

## AQL And Context

- [`AQL_V0_4.md`](AQL_V0_4.md) - current AQL grammar.
- [`AQL_COMPATIBILITY.md`](AQL_COMPATIBILITY.md) - AQL v0.4 compatibility boundary.
- [`AQL_CHANGELOG.md`](AQL_CHANGELOG.md) - AQL grammar and binder changelog.
- [`AQL_V0_3.md`](AQL_V0_3.md) and [`aql-v0.3.md`](aql-v0.3.md) - historical v0.3 notes.
- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) - Context Pack v1 contract.
- [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md) - Context Pack technology overview.
- [`CONTEXT_PACK_QUALITY_EVIDENCE.md`](CONTEXT_PACK_QUALITY_EVIDENCE.md) - local Epic 11 evidence gate.
- [`VERIFY_FACT.md`](VERIFY_FACT.md) - deterministic verification behavior.
- [`VERIFICATION_QUALITY_EVIDENCE.md`](VERIFICATION_QUALITY_EVIDENCE.md) - local Epic 12 evidence gate.
- [`FEEDBACK.md`](FEEDBACK.md) - feedback scoring signal.
- [`TOOL_REGISTRY.md`](TOOL_REGISTRY.md) - typed tool registry cells and agent scope enforcement.
- [`KNOWLEDGE_GRAPH.md`](KNOWLEDGE_GRAPH.md) - typed entity/relation/source traversal.

## Storage And Recovery

- [`ACLOG_FORMAT.md`](ACLOG_FORMAT.md) - WAL format.
- [`WAL_REPLAY.md`](WAL_REPLAY.md) - replay behavior.
- [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md) - storage file formats.
- [`SEGMENT_BUNDLES.md`](SEGMENT_BUNDLES.md) - segment bundle consistency.
- [`ATOMIC_WRITE_AUDIT.md`](ATOMIC_WRITE_AUDIT.md) - atomic write coverage.
- [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) - backup/restore operations.
- [`STORAGE_COMPATIBILITY.md`](STORAGE_COMPATIBILITY.md) - storage compatibility boundary.
- [`STORAGE_COMPATIBILITY_EVIDENCE.md`](STORAGE_COMPATIBILITY_EVIDENCE.md) - local Epic 7 evidence gate.
- [`STORAGE_SOAK.md`](STORAGE_SOAK.md) - repeated storage durability soak gate.
- [`ENCRYPTED_BACKUPS_DESIGN.md`](ENCRYPTED_BACKUPS_DESIGN.md) - encrypted backup MVP and future KMS boundary.
- [`RPO_RTO.md`](RPO_RTO.md) - single-node RPO/RTO boundaries.
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
- [`HTTP_SERVER_CONTRACT_EVIDENCE.md`](HTTP_SERVER_CONTRACT_EVIDENCE.md) - local Epic 13 evidence gate.
- [`AUTH.md`](AUTH.md) - authentication and token policy.
- [`RBAC_POLICY_STORE_DESIGN.md`](RBAC_POLICY_STORE_DESIGN.md) - future dynamic RBAC policy-store design.
- [`ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md`](ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md) - future enterprise RBAC and compliance design.
- [`EXTERNAL_IDENTITY_DESIGN.md`](EXTERNAL_IDENTITY_DESIGN.md) - future external identity provider design.
- [`EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md`](EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md) - future external identity operator checklist.
- [`CLI.md`](CLI.md) - CLI command reference.
- [`CLI_PRODUCT_EVIDENCE.md`](CLI_PRODUCT_EVIDENCE.md) - local Epic 14 evidence gate.
- [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md) - SDK usage.
- [`SDK_RELEASE.md`](SDK_RELEASE.md) - package release procedure.
- [`SDK_PUBLICATION_STATUS.md`](SDK_PUBLICATION_STATUS.md) - registry publication boundary and beta SDK status.
- [`SDK_DEPRECATION_POLICY.md`](SDK_DEPRECATION_POLICY.md) - SDK deprecation rules.
- [`SDK_E2E_RELEASE_EVIDENCE.md`](SDK_E2E_RELEASE_EVIDENCE.md) - local Epic 15 evidence gate.

## Search, ANN, And Benchmarks

- [`SEARCH.md`](SEARCH.md) - lexical/vector/search foundations.
- [`ANN_PRODUCTION_TUNING.md`](ANN_PRODUCTION_TUNING.md) - ANN guardrails and tuning.
- [`HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md`](HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md) - future fallback-free HNSW production design.
- [`ANN_CORPUS_FORMAT.md`](ANN_CORPUS_FORMAT.md) - ANN corpus format.
- [`ANN_PUBLIC_CORPUS_RUNS.md`](ANN_PUBLIC_CORPUS_RUNS.md) - public corpus run policy.
- [`BENCHMARKS.md`](BENCHMARKS.md) - benchmark gates and reports.
- [`PUBLIC_BENCHMARKS.md`](PUBLIC_BENCHMARKS.md) - release-by-release public benchmark summary.
- [`PERFORMANCE_TREND_HISTORY.md`](PERFORMANCE_TREND_HISTORY.md) - local p95/p99 load and single-node trend gate.
- [`RETRIEVAL_QUALITY_EVIDENCE.md`](RETRIEVAL_QUALITY_EVIDENCE.md) - local Epic 10 evidence gate.

## Product Surfaces

- [`DASHBOARD_UI.md`](DASHBOARD_UI.md) - dashboard status and gates.
- [`DASHBOARD_PRODUCT_UI_EVIDENCE.md`](DASHBOARD_PRODUCT_UI_EVIDENCE.md) - local Epic 16 dashboard product UI evidence gate.
- [`BINARY_RELEASES.md`](BINARY_RELEASES.md) - binary tarball packaging and install flow.
- [`BINARY_PLATFORM_MATRIX.md`](BINARY_PLATFORM_MATRIX.md) - supported binary platforms and clean-install smoke.
- [`RELEASE_ARTIFACT_MANIFEST.md`](RELEASE_ARTIFACT_MANIFEST.md) - machine-readable release evidence manifest.
- [`INSTALL.md`](INSTALL.md) - local binary/source install and first database checks.
- [`SYSTEMD.md`](SYSTEMD.md) - single-node systemd service example.
- [`LAUNCHD.md`](LAUNCHD.md) - single-node macOS launchd service example.
- [`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md) - offline upgrade and restore-based rollback workflow.
- [`INGESTION.md`](INGESTION.md) - ingestion behavior.
- [`PDF_TEXT_EXTRACTION.md`](PDF_TEXT_EXTRACTION.md) - digital PDF and external OCR adapter boundary.
- [`AGENT_MEMORY.md`](AGENT_MEMORY.md) - local agent memory, TTL, decay, feedback, and demo gate.
- [`METRICS.md`](METRICS.md) - metrics endpoint and fields.
- [`OBSERVABILITY_EVIDENCE.md`](OBSERVABILITY_EVIDENCE.md) - local Epic 18 observability evidence gate.
- [`OBSERVABILITY_ALERTS.md`](OBSERVABILITY_ALERTS.md) - alert rules and operator actions.
- [`SINGLE_NODE_SLO.md`](SINGLE_NODE_SLO.md) - local single-node SLO boundaries.
- [`OPERATIONS.md`](OPERATIONS.md) - operational guidance.
- [`OPERATIONS_RUNBOOK_EVIDENCE.md`](OPERATIONS_RUNBOOK_EVIDENCE.md) - local single-node operations runbook evidence gate.
- [`BETA_OPERATIONS.md`](BETA_OPERATIONS.md) - beta operations checklist and runbook.
- [`SECURITY_MODEL.md`](SECURITY_MODEL.md) - security model.
- [`SECURITY_BETA_BASELINE.md`](SECURITY_BETA_BASELINE.md) - beta security baseline and backlog boundaries.
- [`SECURITY_THREAT_MODEL.md`](SECURITY_THREAT_MODEL.md) - threat model.
- [`SECURITY_HARDENING_EVIDENCE.md`](SECURITY_HARDENING_EVIDENCE.md) - local Epic 17 security hardening evidence gate.
- [`COMPLIANCE_BOUNDARY_MAPPING.md`](COMPLIANCE_BOUNDARY_MAPPING.md) - local compliance evidence boundary and non-claim map.
- [`SECURITY_RELEASE_CHECKLIST.md`](SECURITY_RELEASE_CHECKLIST.md) - security release checklist and non-goals.
- [`SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md`](SECURITY_PRODUCTION_CANDIDATE_DECISIONS.md) - release-blocking security claim decisions.
- [`DEPLOYMENT_UPGRADE_EVIDENCE.md`](DEPLOYMENT_UPGRADE_EVIDENCE.md) - local Epic 19 deployment and upgrade evidence gate.
- [`MANAGED_CLOUD_DESIGN.md`](MANAGED_CLOUD_DESIGN.md) - future managed cloud design.
- [`LLM_INFERENCE_DESIGN.md`](LLM_INFERENCE_DESIGN.md) - future built-in LLM inference design.
- [`LEGAL_VERIFICATION_BOUNDARY.md`](LEGAL_VERIFICATION_BOUNDARY.md) - future legal-grade verification boundary.

## Planning And Release

Current status should be read from `BETA_DELTA.md` and
`REMAINING_EXECUTION_PLAN.md` first. The other roadmap files are useful
backlogs, snapshots, or deeper planning notes.

- [`CORE_ALPHA.md`](CORE_ALPHA.md) - Core Alpha scope.
- [`CORE_ALPHA_RELEASE_CHECKLIST.md`](CORE_ALPHA_RELEASE_CHECKLIST.md) - release checklist.
- [`RELEASE_EVIDENCE.md`](RELEASE_EVIDENCE.md) - latest local release evidence summary and artifact map.
- [`BETA_FOUNDATION_EVIDENCE.md`](BETA_FOUNDATION_EVIDENCE.md) - SDK/API/ContextPack/VERIFY/Search beta foundation evidence.
- [`BETA_RC_EVIDENCE.md`](BETA_RC_EVIDENCE.md) - backup/security/ingestion/dashboard beta RC evidence.
- [`PRODUCTION_HARDENING_EVIDENCE.md`](PRODUCTION_HARDENING_EVIDENCE.md) - load/crash/migration/audit/rate-limit hardening evidence.
- [`PRODUCTION_CANDIDATE_EVIDENCE.md`](PRODUCTION_CANDIDATE_EVIDENCE.md) - RPO/RTO/SLO/compatibility production-candidate evidence.
- [`PRODUCTION_V1.md`](PRODUCTION_V1.md) - single-node Production v1.0 boundary.
- [`PRODUCTION_V1_EVIDENCE.md`](PRODUCTION_V1_EVIDENCE.md) - Production v1.0 gate evidence.
- [`STORAGE_COMPATIBILITY.md`](STORAGE_COMPATIBILITY.md) - storage durability and compatibility boundary.
- [`STORAGE_COMPATIBILITY_EVIDENCE.md`](STORAGE_COMPATIBILITY_EVIDENCE.md) - storage compatibility gate evidence.
- [`ENGINE_API.md`](ENGINE_API.md) - stable embedded engine API boundary.
- [`ENGINE_API_EVIDENCE.md`](ENGINE_API_EVIDENCE.md) - engine API gate evidence.
- [`MODULE_OWNERSHIP.md`](MODULE_OWNERSHIP.md) - module ownership and stable/internal split.
- [`AQL_COMPATIBILITY.md`](AQL_COMPATIBILITY.md) - AQL v0.4 client compatibility boundary.
- [`AQL_COMPATIBILITY_EVIDENCE.md`](AQL_COMPATIBILITY_EVIDENCE.md) - AQL compatibility gate evidence.
- [`AQL_CHANGELOG.md`](AQL_CHANGELOG.md) - AQL grammar/binder compatibility changelog.
- [`RETRIEVAL_QUALITY_EVIDENCE.md`](RETRIEVAL_QUALITY_EVIDENCE.md) - retrieval quality and ANN history gate evidence.
- [`CONTEXT_PACK_QUALITY_EVIDENCE.md`](CONTEXT_PACK_QUALITY_EVIDENCE.md) - ContextPack quality gate evidence.
- [`VERIFICATION_QUALITY_EVIDENCE.md`](VERIFICATION_QUALITY_EVIDENCE.md) - Verification quality gate evidence.
- [`RELEASE_NOTES_v0.1.0-core-alpha.md`](RELEASE_NOTES_v0.1.0-core-alpha.md) - base Core Alpha release notes.
- [`RELEASE_NOTES_v0.1.0-core-alpha.5.md`](RELEASE_NOTES_v0.1.0-core-alpha.5.md) - audited public Core Alpha prerelease notes.
- [`FUTURE_PRODUCT_LAYERS_PLAN.md`](FUTURE_PRODUCT_LAYERS_PLAN.md) - future product backlog.
- [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) - future non-goal epics and promotion gates.
- [`EPIC_EXECUTION_ORDER.md`](EPIC_EXECUTION_ORDER.md) - active ordered epic queue.
- [`NEXT_60_EPICS.md`](NEXT_60_EPICS.md) - normalized next-stage backlog extracted from the latest external `pl.md`.
- [`PL_IMPLEMENTATION_TASKS.md`](PL_IMPLEMENTATION_TASKS.md) - normalized action list from the external `pl.md` audit.
- [`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md) - full epic/task extraction from the external `pl.md` audit.
- [`PL_NEXT_DEVELOPMENT_PLAN.md`](PL_NEXT_DEVELOPMENT_PLAN.md) - normalized next-stage plan from the latest external `pl.md` audit.
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
