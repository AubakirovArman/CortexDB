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
| closed | 57 |
| partial | 1 |
| not started | 0 |
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
| 7 | Python SDK Productization | closed | Python SDK package metadata, typed client, tests, example, wheel/build evidence, registry-gate wiring, and live SDK contract coverage are checked by `make sdk-productization-check`. | Public PyPI publication remains manual tag-gated and is not claimed by the local gate. |
| 8 | TypeScript SDK Productization | closed | TypeScript SDK package metadata, ESM client, `.d.ts` types, tests, example, npm dry-run evidence, registry-gate wiring, and live SDK contract coverage are checked by `make sdk-productization-check`. | Public npm publication remains manual tag-gated and is not claimed by the local gate. |
| 9 | Rust SDK Productization | closed | `crates/cortex-sdk` has typed API structs, examples, `cargo package` evidence, docs coverage boundary, registry-gate wiring, and live SDK contract coverage checked by `make sdk-productization-check`. | Public crates.io publication remains manual tag-gated and is not claimed by the local gate. |
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
| 23 | HNSW Production SLO History | closed | `make ann-production-slo-history-check` now builds a fresh 10-run local domain ANN/HNSW history, requires one corpus group, zero recall/graph regressions, bounded latency drift, and production-safe multi-layer graph evidence; `make production-evidence-sweep` includes the gate. | Keep adding larger external and real traffic histories without weakening the 10-run local SLO gate. |
| 24 | Search Explain API | closed | `/v1/search/explain` now exposes rank, matched terms, term contribution details, lexical/vector q16 shares, hybrid fusion rank score, typed SDK decoding, CLI hybrid explain support, and OpenAPI/docs coverage. | Keep explain fields additive and deterministic as ranking internals evolve. |
| 25 | Query Routing: Lexical vs Vector vs Hybrid | closed | Engine-level `route_search_query` now selects keyword/vector_ann/vector_exact/hybrid, `/v1/search` and `cortexdb search` support `mode=auto`, and HTTP/CLI/SDK responses expose `routing.selected_strategy` plus `routing.reason` with tests/docs/OpenAPI coverage. | Keep routing deterministic until a measured planner is introduced. |
| 26 | Ingestion Jobs v2 | closed | Ingestion endpoints/CLI now expose durable local job records; job state is atomically written, progress tracks completed cells and last cell id, retry/cancel/delete are available, and restart requeues stale `running` jobs as `queued`. | Keep this as a local synchronous job lifecycle until a real background ingestion executor is introduced. |
| 27 | SourceRef Model v1 | closed | `SourceRef` now carries source id, optional source URL, document/page/range/json-path fields, parser aliases for `doc_id` and `chunk_id`, confidence q16, ContextPack citation enforcement, HTTP/CLI/SDK JSON propagation, OpenAPI/docs coverage, and AQL `REQUIRE confidence` runtime filtering. | Keep future provenance fields additive and preserve SourceRef as the structured citation source. |
| 28 | Document Chunking Policies | closed | Engine-level `TextChunkPolicy`, deterministic `split_text_chunks`, stable `chunk_id` generation, chunk SourceRef payload metadata, and regression tests now exist. | Keep future chunking changes policy-driven and backwards-compatible with stable chunk ids. |
| 29 | PDF/Text Extraction Adapter Boundary | closed | `DigitalPdfTextExtractor`, `NativeDigitalPdfTextExtractor`, `ExternalOcrAdapter`, request/output structs, fail-closed disabled OCR adapter, tests, and `PDF_TEXT_EXTRACTION.md` now define the digital-PDF vs external-OCR boundary. | Keep OCR as an explicit external integration until a production OCR adapter is separately built and gated. |
| 30 | Ingestion Validation Report | closed | Engine-level `IngestionValidationReport` now reports warnings, skipped items, and per-cell SourceRef summaries; HTTP ingest responses include it with OpenAPI/docs/snapshot coverage. | Extend warning codes only additively as new ingestion adapters are added. |
| 31 | Dynamic RBAC Policy Store | closed | JSON policy-store principals now sync through `DatabaseActor` into redacted durable `_system:auth_policy` cells; tests cover policy-cell sync, disabled mapping, and no raw-token exposure. | Keep full enterprise RBAC, external identity, and session management out of beta claims until their own gates exist. |
| 32 | Per-token Quotas | closed | Policy-store principals now support request/minute, body-byte/minute, and actor queue quotas; enforcement uses per-token/principal quota keys, returns typed `rate_limited`, and `/v1/metrics` exposes aggregate quota counters without token/principal leakage. | Keep distributed, route-class, and tenant-aware quota accounting for later production tuning. |
| 33 | Tamper-evident Audit Log | closed | File-backed route audit records include `chain_id`, monotonic `sequence`, `prev_hash`, and `event_hash`; `cortexdb audit --verify-chain`, `cortexdb audit verify <file>`, and `cortexdb audit-export-siem --verify-chain` fail closed on broken local continuity. | Compliance-grade immutable ledger, external timestamping, and vendor-managed SIEM delivery remain future work. |
| 34 | Encrypted Backup Design to MVP | closed | Local passphrase archive MVP now has `Database::encrypted_backup_path`, `Database::restore_from_encrypted_backup`, `cortexdb backup-encrypted`, `cortexdb restore-encrypted`, wrong-passphrase/corrupt-ciphertext rejection, and restore validation tests. | KMS-backed envelope encryption, remote object-store restore, and compliance-grade custody workflows remain future work. |
| 35 | Security Check Gate | closed | `make security-check` exists and passed in beta release gate. | Keep it in release gating as new auth surfaces are added. |
| 36 | CortexDB Doctor | closed | `cortexdb doctor` exists with tests/docs. | Extend diagnostics as operations features grow. |
| 37 | Backup/Restore Production Pack | closed | `make backup-restore-production-pack-check` now runs local restore drills, offsite staging, encrypted-backup restore tests, retention evidence, and writes `target/backup-restore-production-pack/report.json` with RPO/RTO boundary evidence. | KMS-backed backup custody, provider object-store restore, and managed DR remain future work. |
| 38 | Storage Soak History | partial | `make storage-soak-history-check` runs a fresh soak and writes explicit 24h evidence status; `make storage-soak-24h-campaign` now provides the resumable campaign runner for accumulating real 24-hour evidence. | Run and retain the full campaign until `target/storage-soak-history/report.json` has `twenty_four_hour_evidence.met=true`. |
| 39 | Migration Compatibility Matrix v2 | closed | `make migration-compatibility-check` now validates the matrix, restores historical backup fixtures, and runs `migration_upgrade_matrix_v2_check.py` to open the previous-release direct database fixture, write with the current binary, flush/compact, back up, restore, and verify old plus new cells. | Keep adding release-to-release fixtures for each public release pair. |
| 40 | Operations Runbook | closed | `OPERATIONS.md` now has release-binary and source paths, health/auth/data/backup/restore/repair/audit/upgrade commands, known limits, and an evidence bundle; `make operations-runbook-check` writes `target/operations-runbook/report.json` and is included in production evidence sweep. | Keep the runbook aligned as operator surfaces change. |
| 41 | Dashboard Operational Status View | closed | Dashboard operational status now combines health, stats, validation, metrics reachability, backup posture, last request error state, and visible incidents; `make dashboard-product-check` verifies the source markers and release package. | Keep the status view aligned as backup and observability surfaces change. |
| 42 | Dashboard ContextPack Explorer | closed | Context route now renders selected cells, token budget usage, citation/source-ref rows, score-component explain cards, anomalies, and `why_excluded` rows; `make dashboard-product-check` guards the explorer markers. | Keep the explorer additive as ContextPack scoring fields evolve. |
| 43 | Dashboard Verification Explorer | closed | Verify route now renders verdict, supporting evidence, contradicting evidence, numeric conflict, mixed-evidence, and guard explorer sections; `make dashboard-product-check` guards the UI markers. | Keep future verification fields additive and visible in the explorer. |
| 44 | Dashboard Permissions View | closed | Permissions route now renders token state, role/access, selected scope probes, local read-only guard state, and AgentView server-enforcement posture; `make dashboard-product-check` guards the markers. | Keep the page read-only unless a dedicated admin auth-management UI is introduced. |
| 45 | Dashboard Incident View | closed | Dashboard operational status now includes an incident timeline for audit/rate-limit/storage/backup events with severity, source, message, and action guidance; `make dashboard-product-check` guards the markers. | Keep this as a read-only triage view until a real incident-management product is introduced. |
| 46 | Linux/macOS Binary Release Pipeline | closed | `.github/workflows/release.yml` now has an explicit four-platform matrix for `linux-x86_64`, `linux-aarch64`, `macos-arm64`, and `macos-x86_64`; `make binary-platform-matrix-check` validates docs/workflow markers and clean-install smoke for the local archive. | Keep release tag runs attached with all matrix artifacts before each public release. |
| 47 | Install Script | closed | `scripts/install.sh` verifies external `.sha256`, internal `SHA256SUMS`, executable bits, and installs CLI/server binaries; `make install-script-check` validates dry-run, install, and corrupt-checksum rejection. | Keep the script compatible with Linux `sha256sum` and macOS `shasum`. |
| 48 | Systemd and launchd Support | closed | `SYSTEMD.md`, `LAUNCHD.md`, checked-in service/plist examples, and `make service-manager-smoke-check` now validate Linux/macOS service-manager artifacts. | Keep examples aligned with release binary paths and auth environment changes. |
| 49 | Release Artifact Manifest | closed | `make release-artifact-manifest-check` now writes and validates `target/release-artifact-manifest/manifest.json` with binary, sidecar checksum, SDK, OpenAPI, evidence report, install-script, binary-platform, and git metadata. | Keep adding required evidence reports as release gates expand. |
| 50 | Version and Compatibility Dashboard | closed | `/v1/compatibility` now exposes API, SDK contract, storage format, and migration matrix versions; dashboard operational status renders the compatibility section and OpenAPI/dashboard checks guard it. | Keep compatibility fields additive and update docs/OpenAPI when release contracts change. |
| 51 | Official Beta Landing Page | closed | `BETA_LANDING.md` now provides a concise external beta path with demo, beta scope, non-goals, evidence links, and contribution path; `make beta-landing-check` is included in `make beta-release-check`. | Keep landing claims aligned with public claims policy and release notes. |
| 52 | Use-case Packs | closed | Legal, financial, and technical use-case packs now live under `examples/use_cases`; `make use-case-pack-check` validates manifests, fixtures, scenario docs, and CLI search/context/verify smoke flows. | Keep packs aligned with public claims boundaries and add more domains only behind the same gate. |
| 53 | Contributor Onboarding | closed | `CONTRIBUTOR_ONBOARDING.md`, `GOOD_FIRST_ISSUES.md`, a good-first-issue template, and `make contributor-onboarding-check` now provide a checked 15-minute path and starter task map. | Keep starter tasks bounded and update the checker as new contributor surfaces appear. |
| 54 | Public Benchmarks Page | closed | `PUBLIC_BENCHMARKS.md` now summarizes release-by-release benchmark evidence and links retrieval, ContextPack, verification, performance, beta, and claims gates; `make public-benchmarks-check` validates the page. | Refresh snapshots before each public release and keep non-claims visible. |
| 55 | Comparison Docs | closed | `COMPARISONS.md` now compares CortexDB with SQL databases, vector databases, classic RAG stacks, agent memory frameworks, and search engines without replacement claims; `make comparison-docs-check` validates the public-claims boundary. | Keep comparisons factual and update when product boundaries change. |
| 56 | Agent Memory v2 | closed | TTL expiry, fixed-point decay scoring, durable feedback cells, feedback-aware ContextPack ordering, and `examples/demo/agent_memory` are now covered by `make agent-memory-demo-check`. | Keep natural-language contradiction extraction and production memory ranking as future work. |
| 57 | Tool Registry | closed | Tool descriptions now persist as `KnowledgeCellType::Tool` cells with schema fields, permission markers, scope-filtered `Database::list_tools`, AQL retrieval, ContextPack inclusion, docs, and `make tool-registry-check`. | Execution, external credentials, and enterprise RBAC remain future product layers. |
| 58 | Knowledge Graph Layer | closed | `KnowledgeGraphIndex` now builds entity, relation-adjacency, and source-reference indexes from visible typed cells, survives checkpoint/reopen, and is guarded by `make knowledge-graph-check`. | Persisted graph index files, graph query language, and multi-hop ranking remain future work. |
| 59 | Distributed Consensus Research Track | research | `make distributed-consensus-research-check` now aggregates replicated-log, partition, failover, and rejoin local reports while preserving `production_ready=false`. | Keep as research until sustained multi-process split-brain/failover/snapshot evidence can support a production claim. |
| 60 | Managed Cloud Feasibility Track | research | `make managed-cloud-feasibility-check` now aggregates local tenant lifecycle, backup/restore, and upgrade prerequisite reports while preserving `managed_cloud_ready=false`. | Decide build/postpone/reject only after real hosted control-plane, tenant deletion, billing/quota, support-access, and cloud backup evidence exists. |

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

1. Advance Epic 38: run and retain a real 24-hour storage soak campaign.
2. Keep Epics 59-60 as research/feasibility until real operational evidence changes their claim boundary.
