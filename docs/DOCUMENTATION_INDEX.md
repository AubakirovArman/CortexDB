# CortexDB Documentation Index

Last consolidated: 2026-06-12.

The root `docs/` directory is the core documentation set. Historical plans,
evidence snapshots, benchmark experiments, and release working notes live in
[`archive/`](archive/INDEX.md).

## Start Here

- [`README.md`](../README.md) - project overview and quickstart.
- [`GETTING_STARTED.md`](GETTING_STARTED.md) - five-minute local ContextPack path.
- [`PROJECT_STATUS.md`](PROJECT_STATUS.md) - current product status and limits.
- [`CORE_ALPHA.md`](CORE_ALPHA.md) - Core Alpha scope.
- [`INSTALL.md`](INSTALL.md) - local install and first checks.
- [`CONTRIBUTOR_ONBOARDING.md`](CONTRIBUTOR_ONBOARDING.md) - contributor path.

## Docs Site

- [`SUMMARY.md`](SUMMARY.md) - mdBook navigation over the current core docs.
- [`book.toml`](../book.toml) - mdBook configuration; output goes to
  `target/mdbook`.
- Local gate: `make docs-site-check`.
- Local preview when mdBook is installed: `mdbook serve --open`.
- GitHub Pages deploy: [`.github/workflows/docs-pages.yml`](../.github/workflows/docs-pages.yml).

## Core Database

- [`ARCHITECTURE.md`](ARCHITECTURE.md) - workspace and subsystem map.
- [`CORE_INVARIANTS.md`](CORE_INVARIANTS.md) - safety invariants.
- [`DATA_MODEL.md`](DATA_MODEL.md) - cell and metadata model overview.
- [`CELL_METADATA_MODEL.md`](CELL_METADATA_MODEL.md) - metadata source of truth.
- [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md) - WAL/segment/index/manifest formats.
- [`TIERED_STORAGE_V2.md`](TIERED_STORAGE_V2.md) - hot/cold lazy payload cache
  policy and F01 prototype gate.
- [`LEXICAL_INDEX.md`](LEXICAL_INDEX.md) - compact lexical term dictionary and
  postings contract.
- [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) - backup and restore operations.
- [`CORRUPTION_HANDLING.md`](CORRUPTION_HANDLING.md) - validation issues and recovery actions.
- [`ENGINE_API.md`](ENGINE_API.md) - embedded Rust engine API.

## Agent-Native Surface

- [`AGENT_TRANSACTION_SEMANTICS.md`](AGENT_TRANSACTION_SEMANTICS.md) -
  multi-agent write conflict and isolation semantics.
- [`AQL_V0_5.md`](AQL_V0_5.md) - current AQL REMEMBER write contract.
- [`AQL_V0_4.md`](AQL_V0_4.md) - frozen AQL grammar.
- [`BRAIN_SEMANTICS.md`](BRAIN_SEMANTICS.md) - single-brain contract and
  deprecated alias migration plan.
- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) - ContextPack contract.
- [`VERIFY_FACT.md`](VERIFY_FACT.md) - deterministic verification.
- [`AGENT_MEMORY.md`](AGENT_MEMORY.md) - agent memory behavior.
- [`SEARCH.md`](SEARCH.md) - lexical/vector/hybrid search.
- [`SCORING.md`](SCORING.md) - BM25, field weights, and query scoring.
- [`LEARNED_RANKING_CALIBRATION.md`](LEARNED_RANKING_CALIBRATION.md) -
  offline train/heldout ranking calibration and opt-in runtime flag.
- [`SEMANTIC_MEMORY_COMPRESSION.md`](SEMANTIC_MEMORY_COMPRESSION.md) -
  opt-in external-worker semantic summary commit contract and audit metadata.
- [`INGESTION.md`](INGESTION.md) - ingestion behavior.

## API, CLI, SDK

- [`API.md`](API.md) - HTTP API contract.
- [`API_JSON_SCHEMAS.md`](API_JSON_SCHEMAS.md) - JSON response examples.
- [`AUTH.md`](AUTH.md) - auth and token policy.
- [`CLI.md`](CLI.md) - CLI reference.
- [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md) - SDK usage.
- [`MCP.md`](MCP.md) - MCP stdio adapter for agent tools.
- [`examples/integrations`](../examples/integrations/README.md) - live
  integration examples for tool-calling, LangChain-style retrieval, and memory
  chat agents.

## Operations

- [`OPERATIONS.md`](OPERATIONS.md) - operator guidance.
- [`AUDIT_LOG_FORMAT.md`](AUDIT_LOG_FORMAT.md) - local audit JSONL schema,
  rotation, fsync, and SIEM export boundary.
- [`DOCKER.md`](DOCKER.md) - Docker image, compose quickstart, and GHCR release path.
  Deployment artifacts: [`docker-compose.production.yml`](../docker-compose.production.yml),
  [`auth.tokens.example`](deployment/auth.tokens.example), and
  [`cortexdb.conf`](deployment/nginx/cortexdb.conf).
- Archived install and upgrade runbooks: [`SYSTEMD.md`](archive/SYSTEMD.md),
  [`LAUNCHD.md`](archive/LAUNCHD.md),
  [`UPGRADE_ROLLBACK.md`](archive/UPGRADE_ROLLBACK.md), and
  [`BINARY_PLATFORM_MATRIX.md`](archive/BINARY_PLATFORM_MATRIX.md).
- [`METRICS.md`](METRICS.md) - metrics endpoint and fields.
- [`SECURITY_MODEL.md`](SECURITY_MODEL.md) - security model.
- [`BENCHMARKS.md`](BENCHMARKS.md) - benchmark gates and reports.
- [`SCALE_BENCHMARKS.md`](SCALE_BENCHMARKS.md) - 100K/1M/10M scale baselines.
- [`SDK_DOCKER_OBSERVABILITY.md`](SDK_DOCKER_OBSERVABILITY.md) - integration smoke paths.

## Product And Roadmap

- [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) - public wording boundary.
- [`BETA_LANDING.md`](BETA_LANDING.md) - concise external beta landing page.
- [`COMPARISONS.md`](COMPARISONS.md) - adjacent-stack comparison.
- [`COMMUNITY_ROADMAP.md`](COMMUNITY_ROADMAP.md) - roadmap board.
- [`FUTURE_PRODUCT_LAYERS_PLAN.md`](FUTURE_PRODUCT_LAYERS_PLAN.md) - future product layers.
- [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) - future/non-goal epics.
- [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md) - active ordered epic plan.
- [`RELEASE_NOTES_v0.2.0-beta.2.md`](RELEASE_NOTES_v0.2.0-beta.2.md) - beta release notes.

## Archive

- [`archive/INDEX.md`](archive/INDEX.md) - archived evidence, experiments, old
  roadmaps, compatibility notes, and historical release artifacts.
