# CortexDB Next 60 Epics

Source: `/mnt/hf_model_weights/arman/3bit/sites/pl.md`, audited on 2026-06-01.

This file is the normalized execution backlog for the next product layer after
`v0.2.0-beta.1`. It should be treated as the active epic map unless a newer
plan explicitly replaces it.

## Status Legend

- `closed` - implemented and backed by a local or release gate.
- `partial` - foundation exists, but the epic acceptance is not fully met.
- `not started` - no meaningful implementation was found beyond notes/design.
- `research` - intentionally exploratory and not a product claim.

## Current Summary

| Status | Count |
| --- | ---: |
| closed | 27 |
| partial | 26 |
| not started | 5 |
| research | 2 |
| total | 60 |

## Priority Order

The plan recommends this first execution batch:

1. Public Beta Release Definition
2. Beta Release Evidence Bundle
3. SDK Public Release Train
4. ContextPack Quality v2
5. Verification Dataset v2
6. Real-domain Corpus Expansion
7. Retrieval Quality Dashboard
8. Security Check Gate
9. Operations Runbook
10. Linux/macOS Binary Release Pipeline

## Epic Table

| # | Epic | Status | What Exists Now | Remaining Closure Work |
| ---: | --- | --- | --- | --- |
| 1 | Public Beta Release Definition | closed | `BETA_RELEASE.md`, README/public claims boundary, release tag `v0.2.0-beta.1`. | Keep wording aligned as features change. |
| 2 | Beta Release Evidence Bundle | closed | `make beta-release-check`, `target/beta-release/evidence.tar.gz`, GitHub release asset. | Keep bundle regenerated for each release. |
| 3 | Public Beta Release Notes | closed | `RELEASE_NOTES_v0.2.0-beta.1.md` and GitHub release notes. | Update for patch/beta follow-ups. |
| 4 | Beta Claims Guard | closed | `make public-claims-check`, public claims policy/freeze docs. | Add new forbidden claims as surfaces expand. |
| 5 | Beta Compatibility Matrix | closed | `compatibility_matrix_v1.json` now fixes `v0.1.0-core-alpha.5 -> v0.2.0-beta.1`; `make migration-compatibility-check` restores the historical backup fixture and validates API/SDK/storage gate wiring. | Keep adding previous-release fixtures for future releases. |
| 6 | SDK Public Release Train | closed | SDK release manifest now includes `sdk-registry-gate-check`; `make sdk-e2e-release-check` runs release contract, artifacts, registry gate, and live SDK contract checks. | Public registry publication still requires manual tag-gated workflow execution and credentials. |
| 7 | Python SDK Productization | partial | Python SDK, examples, tests, wheel artifacts exist. | Harden packaging, structured errors, docs, and published package workflow. |
| 8 | TypeScript SDK Productization | partial | TypeScript SDK, generated types, examples, package metadata exist. | Harden ESM/CJS packaging, typed errors, and public npm workflow. |
| 9 | Rust SDK Productization | partial | `crates/cortex-sdk` exists with typed client/examples. | Finish crate publication readiness and docs.rs-quality docs. |
| 10 | SDK Compatibility Contract | closed | `make sdk-contract-check` and OpenAPI/SDK checks exist. | Keep blocking drift on every release. |
| 11 | ContextPack Quality v2 | closed | `make context-pack-quality-check` now validates 25 cases across 5 domains and writes aggregate plus per-domain metrics. | Keep adding private/customer-domain evidence later without weakening deterministic gates. |
| 12 | ContextPack Explain v2 | closed | ContextPack explain now exposes `why_selected`, structured `score_components`, source refs, and `why_excluded` for excluded candidates/token overload across engine, CLI, SDK, server JSON, snapshots, and OpenAPI. | Keep explanations stable as new scoring components are added. |
| 13 | ContextPack vs Classic RAG Benchmark | closed | RAG demo smoke and expected output contract exist. | Keep demo evidence updated for each release. |
| 14 | ContextPack Agent Prompt Export | closed | ContextPack now has engine-level stable prompt and Markdown exporters, CLI `context --format prompt|markdown`, HTTP `/v1/context?format=prompt|markdown`, Rust SDK helpers, tests, OpenAPI, and docs. | Keep export wording stable across beta releases unless schema/version policy changes. |
| 15 | ContextPack Budget Optimizer | closed | ContextPack now accounts for required-citation overhead, applies redundancy checks before budget overload, skips oversized middle candidates while keeping later smaller cells, and documents/tests the optimizer behavior. | Keep estimator changes deterministic and covered by quality fixtures. |
| 16 | Verification Dataset v2 | closed | `make verification-quality-check` now executes and reports 50 deterministic cases across 5 domains with all verdict classes and guard coverage. | Add real customer/legal review fixtures later without weakening deterministic gates. |
| 17 | NumericValue Engine Integration | closed | `VerificationReport.numeric_conflicts` now carries engine-level structured numeric conflicts with `cell_id`, metric, display values, and typed `NumericValue` pairs; CLI/server only serialize the report. | Keep future unit/currency expansion inside engine structs before exposing new API fields. |
| 18 | Source Trust Model v1 | closed | Engine now has first-class `SourceTrust`/`SourceTrustCategory`; ContextPack explain and VERIFY evidence expose deterministic q16/category contribution across CLI/server/SDK surfaces. | Add richer policy inputs later without changing q16 category semantics silently. |
| 19 | Contradiction Index v1 | closed | `Database::persist_contradiction_relation` writes durable `type=relation` cells with `predicate=contradicts`; `conflict_index` and `conflicts_for_fact` now read both inline markers and persisted relation cells under the caller's AgentView scope mask, with restart regression coverage. | Expose the contradiction index through CLI/server later only if product consumers need a public route. |
| 20 | Verification Report Export | closed | `VerificationReport` now has engine-level Markdown and deterministic audit-text exporters, wired through CLI `verify --format markdown|audit` and HTTP `/v1/verify?format=markdown|audit`. | Keep export wording stable unless a new report export version is introduced. |
| 21 | Real-domain Corpus Expansion | closed | `make retrieval-quality-check` now validates investment projects, support tickets, legal policies, and technical docs corpora with ground truth. | Add larger public/private corpora later as separate quality expansions. |
| 22 | Retrieval Quality Dashboard | closed | `make retrieval-quality-check` now writes `target/retrieval-quality/dashboard.html` with guarded ANN and per-domain recall/MRR/nDCG/p95/exact-parity tables. | Keep the dashboard in the beta evidence bundle and extend it as more domains are added. |
| 23 | HNSW Production SLO History | partial | ANN guardrails and real-domain report gates exist. | Build sustained 10+ run history and SLO regression tracking. |
| 24 | Search Explain API | closed | `/v1/search/explain` now exposes rank, matched terms, term contribution details, lexical/vector q16 shares, hybrid fusion rank score, typed SDK decoding, CLI hybrid explain support, and OpenAPI/docs coverage. | Keep explain fields additive and deterministic as ranking internals evolve. |
| 25 | Query Routing: Lexical vs Vector vs Hybrid | closed | Engine-level `route_search_query` now selects keyword/vector_ann/vector_exact/hybrid, `/v1/search` and `cortexdb search` support `mode=auto`, and HTTP/CLI/SDK responses expose `routing.selected_strategy` plus `routing.reason` with tests/docs/OpenAPI coverage. | Keep routing deterministic until a measured planner is introduced. |
| 26 | Ingestion Jobs v2 | partial | Ingestion endpoints/CLI and responses exist. | Add durable jobs, retry/cancel/progress, and restart resume. |
| 27 | SourceRef Model v1 | partial | `SourceRef` structs include document/page/row/json path/source URL fields. | Finish extraction confidence and end-to-end SourceRef enforcement. |
| 28 | Document Chunking Policies | partial | Chunking exists in example/real-domain scripts and ingestion docs. | Make engine-level chunk id stability and policy tests first-class. |
| 29 | PDF/Text Extraction Adapter Boundary | partial | Ingestion docs and adapter-boundary direction exist. | Add explicit digital PDF/external OCR adapter contracts. |
| 30 | Ingestion Validation Report | partial | Typed ingest responses exist. | Add richer warnings, skipped chunks, invalid metadata, and source-ref reports. |
| 31 | Dynamic RBAC Policy Store | partial | RBAC design/security docs exist. | Implement persisted user/role/policy cells and runtime policy mapping. |
| 32 | Per-token Quotas | partial | Security/rate-limit gates exist. | Add complete per-token body, queue, and metrics budgets. |
| 33 | Tamper-evident Audit Log | partial | Audit paths and security checks exist. | Add hash-chain audit log and `cortexdb audit verify`. |
| 34 | Encrypted Backup Design to MVP | partial | Encrypted backup design doc exists. | Implement passphrase encrypted archive and restore validation. |
| 35 | Security Check Gate | closed | `make security-check` exists and passed in beta release gate. | Keep it in release gating as new auth surfaces are added. |
| 36 | CortexDB Doctor | closed | `cortexdb doctor` exists with tests/docs. | Extend diagnostics as operations features grow. |
| 37 | Backup/Restore Production Pack | partial | Backup drill/offsite checks and docs exist. | Promote backup/restore to supported operational workflow with prune/RPO/RTO evidence. |
| 38 | Storage Soak History | partial | Storage soak target/docs exist. | Accumulate long-running history and 24h soak evidence. |
| 39 | Migration Compatibility Matrix v2 | partial | Migration compatibility docs/targets exist. | Add previous-release DB/backup fixtures and upgrade evidence. |
| 40 | Operations Runbook | partial | `OPERATIONS.md`, beta operations docs, install/runbook docs exist. | Make it complete enough for an operator to run without repo knowledge. |
| 41 | Dashboard Operational Status View | partial | Dashboard assets and product UI evidence exist. | Add complete health/stats/backup/validation/error status view. |
| 42 | Dashboard ContextPack Explorer | partial | Dashboard can inspect retrieval/context foundations. | Add full ContextPack cells/citations/explain/anomalies/token UI. |
| 43 | Dashboard Verification Explorer | partial | Dashboard has verification/reporting foundations. | Add full mixed-evidence/numeric-conflict explorer. |
| 44 | Dashboard Permissions View | partial | Security/RBAC docs and dashboard foundations exist. | Add read-only token/role/scope/AgentView UI. |
| 45 | Dashboard Incident View | partial | Audit/metrics foundations exist. | Add operational incident timeline for audit/rate/storage/backup events. |
| 46 | Linux/macOS Binary Release Pipeline | closed | `.github/workflows/release.yml` now has an explicit four-platform matrix for `linux-x86_64`, `linux-aarch64`, `macos-arm64`, and `macos-x86_64`; `make binary-platform-matrix-check` validates docs/workflow markers and clean-install smoke for the local archive. | Keep release tag runs attached with all matrix artifacts before each public release. |
| 47 | Install Script | closed | `scripts/install.sh` verifies external `.sha256`, internal `SHA256SUMS`, executable bits, and installs CLI/server binaries; `make install-script-check` validates dry-run, install, and corrupt-checksum rejection. | Keep the script compatible with Linux `sha256sum` and macOS `shasum`. |
| 48 | Systemd and launchd Support | partial | Systemd docs exist. | Add launchd docs/examples and smoke validation. |
| 49 | Release Artifact Manifest | closed | `make release-artifact-manifest-check` now writes and validates `target/release-artifact-manifest/manifest.json` with binary, sidecar checksum, SDK, OpenAPI, evidence report, install-script, binary-platform, and git metadata. | Keep adding required evidence reports as release gates expand. |
| 50 | Version and Compatibility Dashboard | not started | Compatibility docs exist. | Expose API/SDK/storage/migration versions in dashboard/API. |
| 51 | Official Beta Landing Page | partial | README and positioning docs exist. | Turn them into a concise external beta landing path. |
| 52 | Use-case Packs | partial | Investment/support demos and RAG demo exist. | Add legal, financial, and technical use-case packs. |
| 53 | Contributor Onboarding | partial | Contributing/module/test docs exist. | Add good-first-issue map and 15-minute onboarding path. |
| 54 | Public Benchmarks Page | partial | Benchmark docs and evidence reports exist. | Publish release-by-release benchmark history in one public page. |
| 55 | Comparison Docs | partial | RAG/vector positioning docs exist. | Add clearer Postgres/vector DB/memory framework comparison without aggressive claims. |
| 56 | Agent Memory v2 | partial | Agent memory docs/module foundations exist. | Add TTL/decay/feedback and end-to-end memory demo. |
| 57 | Tool Registry | not started | Tool registry is roadmap-level only. | Add tool cells, schemas, permissions, and ContextPack inclusion. |
| 58 | Knowledge Graph Layer | not started | Typed cell foundations exist. | Add entity/relation/source graph indexes and traversal. |
| 59 | Distributed Consensus Research Track | research | Consensus/replication design docs and experimental code exist. | Keep as research until split-brain/failover/snapshot evidence is sustained. |
| 60 | Managed Cloud Feasibility Track | research | Managed cloud design doc exists. | Decide build/postpone/reject using tenant/security/ops cost model. |

## Closure Rules

1. Do not mark an epic `closed` from documentation alone.
2. A closed epic needs a command, artifact, test, release asset, or runnable demo.
3. Public registry publication, managed cloud, production distributed consensus,
   enterprise compliance, legal-grade verification, and fallback-free HNSW remain
   non-claims until their dedicated gates pass.
4. Work epics in order unless a blocking dependency forces a local reorder.
5. Every completed epic should update this file, the relevant evidence doc, and
   the release checklist.

## Next Batch

The next practical implementation batch is:

1. Advance Epic 26: durable ingestion jobs with retry/cancel/progress and restart resume.
2. Advance Epic 27: SourceRef extraction confidence and end-to-end enforcement.
