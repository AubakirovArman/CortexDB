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

## Core Database

- [`ARCHITECTURE.md`](ARCHITECTURE.md) - workspace and subsystem map.
- [`CORE_INVARIANTS.md`](CORE_INVARIANTS.md) - safety invariants.
- [`DATA_MODEL.md`](DATA_MODEL.md) - cell and metadata model overview.
- [`CELL_METADATA_MODEL.md`](CELL_METADATA_MODEL.md) - metadata source of truth.
- [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md) - WAL/segment/index/manifest formats.
- [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) - backup and restore operations.
- [`ENGINE_API.md`](ENGINE_API.md) - embedded Rust engine API.

## Agent-Native Surface

- [`AQL_V0_4.md`](AQL_V0_4.md) - current AQL grammar.
- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) - ContextPack contract.
- [`VERIFY_FACT.md`](VERIFY_FACT.md) - deterministic verification.
- [`AGENT_MEMORY.md`](AGENT_MEMORY.md) - agent memory behavior.
- [`SEARCH.md`](SEARCH.md) - lexical/vector/hybrid search.
- [`INGESTION.md`](INGESTION.md) - ingestion behavior.

## API, CLI, SDK

- [`API.md`](API.md) - HTTP API contract.
- [`API_JSON_SCHEMAS.md`](API_JSON_SCHEMAS.md) - JSON response examples.
- [`AUTH.md`](AUTH.md) - auth and token policy.
- [`CLI.md`](CLI.md) - CLI reference.
- [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md) - SDK usage.

## Operations

- [`OPERATIONS.md`](OPERATIONS.md) - operator guidance.
- [`METRICS.md`](METRICS.md) - metrics endpoint and fields.
- [`SECURITY_MODEL.md`](SECURITY_MODEL.md) - security model.
- [`BENCHMARKS.md`](BENCHMARKS.md) - benchmark gates and reports.
- [`SCALE_BENCHMARKS.md`](SCALE_BENCHMARKS.md) - 100K/1M scale baselines.

## Product And Roadmap

- [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) - public wording boundary.
- [`COMPARISONS.md`](COMPARISONS.md) - adjacent-stack comparison.
- [`COMMUNITY_ROADMAP.md`](COMMUNITY_ROADMAP.md) - roadmap board.
- [`FUTURE_PRODUCT_LAYERS_PLAN.md`](FUTURE_PRODUCT_LAYERS_PLAN.md) - future product layers.
- [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) - future/non-goal epics.
- [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md) - active ordered epic plan.
- [`RELEASE_NOTES_v0.2.0-beta.1.md`](RELEASE_NOTES_v0.2.0-beta.1.md) - beta release notes.

## Archive

- [`archive/INDEX.md`](archive/INDEX.md) - archived evidence, experiments, old
  roadmaps, compatibility notes, and historical release artifacts.
