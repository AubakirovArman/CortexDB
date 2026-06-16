# CortexDB Status

Current version: `0.2.0-beta.2`.

This page is the living one-screen status table. It describes what works now,
what is prototype-only, and what remains frozen. Historical release narrative
lives in [`PROJECT_STATUS.md`](PROJECT_STATUS.md).

## Labels

| Label | Meaning |
| --- | --- |
| production | Not claimed for any subsystem yet. |
| beta | Works locally with tests, contracts, and release gates; not production HA. |
| prototype | Useful research or opt-in feature; not a stable product claim. |
| frozen | Explicitly deferred; no new feature work without unfreeze criteria. |

## Workspace Crates

| Crate | Status | What works now | Evidence |
| --- | --- | --- | --- |
| `cortex-core` | beta | Knowledge cell contracts, descriptors, MVCC-facing types. | [`DATA_MODEL.md`](DATA_MODEL.md), [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md) |
| `cortex-storage` | beta | WAL, segments, checksums, manifest, compatibility tests. | [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md), [`UPGRADE_MIGRATION.md`](archive/UPGRADE_MIGRATION.md) |
| `cortex-aql` | beta | AQL parser/binder, bitmap plans, policy-safe retrieval shape. | [`AQL_V0_5.md`](AQL_V0_5.md), [`ENGINE_API.md`](ENGINE_API.md) |
| `cortex-engine` | beta | Single-node database engine, retrieval, ContextPack, verification, indexes. | [`PROJECT_STATUS.md`](PROJECT_STATUS.md), [`SEARCH.md`](SEARCH.md), [`CONTEXT_PACK.md`](CONTEXT_PACK.md) |
| `cortex-server` | beta | HTTP API, auth policy store, tenant realms, quotas, OpenAPI snapshots. | [`API.md`](API.md), [`SECURITY_MODEL.md`](SECURITY_MODEL.md) |
| `cortex-cli` | beta | Local admin/dev CLI, backup/restore, validate/repair, doctor flows. | [`GETTING_STARTED.md`](GETTING_STARTED.md), [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) |
| `cortex-api-types` | beta | Shared request/response types for SDK and server contracts. | [`API_JSON_SCHEMAS.md`](API_JSON_SCHEMAS.md), [`API.md`](API.md) |
| `cortex-sdk` | beta | Rust SDK contracts and generated ContextPack aliases. | [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md), [`RELEASE_NOTES_v0.2.0-beta.2.md`](RELEASE_NOTES_v0.2.0-beta.2.md) |
| `cortex-mcp` | beta | Local MCP adapter for agent clients. | [`MCP.md`](MCP.md), [`GETTING_STARTED.md`](GETTING_STARTED.md) |

## Product Subsystems

| Subsystem | Status | Boundary | Evidence |
| --- | --- | --- | --- |
| A/B/C core database line | beta | Durable single-node DB, retrieval, ContextPack, verification, security gates. | [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md), [`PROJECT_STATUS.md`](PROJECT_STATUS.md) |
| Durable storage | beta | Local WAL/MVCC/checkpoint/compact/repair; not distributed storage. | [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md), [`BACKUP_RESTORE.md`](BACKUP_RESTORE.md) |
| AQL + retrieval | beta | Permission-rewritten AQL, lexical/vector/hybrid retrieval, guarded ANN fallback. | [`AQL_V0_5.md`](AQL_V0_5.md), [`SEARCH.md`](SEARCH.md) |
| ContextPack + verify | beta | Budgeted ContextPack v1 and deterministic `VERIFY FACT`; no legal-grade guarantee. | [`CONTEXT_PACK.md`](CONTEXT_PACK.md), [`VERIFY_FACT.md`](VERIFY_FACT.md) |
| HTTP/CLI/SDK/API | beta | Local API and developer tooling with contracts; public registry publishing still separate. | [`API.md`](API.md), [`SDK_QUICKSTART.md`](SDK_QUICKSTART.md) |
| Security controls | beta | Local token auth, AgentView policy, tenant realms, quotas, audit; not enterprise IAM. | [`SECURITY_MODEL.md`](SECURITY_MODEL.md) |
| Operations | beta | Backup/restore, metrics, docs gates, dashboard checks, release gates. | [`BETA_RELEASE.md`](archive/BETA_RELEASE.md), [`OBSERVABILITY_EVIDENCE.md`](archive/OBSERVABILITY_EVIDENCE.md) |
| F-block research | prototype | Tiered storage v2, agent transactions, learned ranking, semantic compression, value-per-token planning, multi-agent consistency, formal invariants. | [`PROJECT_STATUS.md`](PROJECT_STATUS.md), [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md) |
| Distributed replication | frozen | `EPIC-F02`; no HA product claim until unfreeze criteria are met. | [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md), [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) |
| Consensus / multi-node transactions | frozen | `EPIC-F03`; no production consensus claim. | [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md), [`FUTURE_NON_GOAL_EPICS.md`](FUTURE_NON_GOAL_EPICS.md) |
| Managed cloud | frozen | `EPIC-F09`; local prerequisite gates only, no hosted service. | [`DATABASE_GRADE_EXECUTION_PLAN.md`](DATABASE_GRADE_EXECUTION_PLAN.md), [`COMMUNITY_ROADMAP.md`](COMMUNITY_ROADMAP.md) |

## Benchmark Status

| Benchmark | Status | Current number | Boundary | Evidence |
| --- | --- | ---: | --- | --- |
| EnterpriseRAG-Bench 500 | interim | `47.74` | Gemma answerer + Gemini judge; not leaderboard-comparable until `gpt-5.4` re-judge. | [`erb-submission`](../erb-submission/README.md) |
| LongMemEval v1 | beta evidence | `0.7660` | Official local run; not a production or SOTA claim. | [`LONGMEMEVAL_OFFICIAL.md`](archive/LONGMEMEVAL_OFFICIAL.md) |

## Not Production

CortexDB does not currently claim production IAM, TLS/mTLS lifecycle,
encrypted-at-rest storage/backups, distributed HA, managed cloud, compliance
certification, or production SLA support. See
[`SECURITY_MODEL.md`](SECURITY_MODEL.md) for the security boundary.
