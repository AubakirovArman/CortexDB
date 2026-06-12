# Epic 26 Status Matrix

Total tracked epics: 26 (19 roadmap + 7 active queue).

## Source
- `docs/EPIC_EXECUTION_ORDER.md` has 7 active epics.
- `docs/PL_EXTRACTED_EPICS.md` has 19 roadmap epics.

## Epic table

| № | Source | Epic | Status | Evidence | Note |
|---|---|---|---|---|---|
| 01 | Active Queue | Epic 1 - Production Evidence Sweep And Reality Check | done | passed | target/production-evidence/report.json |
| 02 | Active Queue | Epic 2 - API And SDK Contract Proof | done | passed | target/sdk-e2e-release/report.json |
| 03 | Active Queue | Epic 3 - Context Pack Quality Lock | done | passed | target/context-pack-quality/report.json |
| 04 | Active Queue | Epic 4 - ANN/HNSW Guarded Production Evidence | done | passed | target/retrieval-quality/report.json |
| 05 | Active Queue | Epic 5 - Security And Operations Baseline | done | passed | target/security-hardening/report.json |
| 06 | Active Queue | Epic 6 - Product UI And Release Surface Hardening | done | passed | target/dashboard/product-ui-report.json |
| 07 | Active Queue | Epic 7 - Consensus Hardening | done | passed / ok | target/replication-partition/report.json, target/replication-lifecycle/report.json |
| 08 | Roadmap | Epic 1 - Evidence-Backed Core Alpha | done | passed | target/production-evidence/report.json |
| 09 | Roadmap | Epic 2 - Beta Foundation | done | passed | target/beta-foundation/report.json |
| 10 | Roadmap | Epic 3 - Beta Release Candidate | done | passed | target/beta-rc/report.json |
| 11 | Roadmap | Epic 4 - Production Hardening | done | passed | target/production-hardening/report.json |
| 12 | Roadmap | Epic 5 - Production Candidate | done | passed | target/production-candidate/report.json |
| 13 | Roadmap | Epic 6 - Production v1.0 | done | passed | target/production-v1/report.json |
| 14 | Roadmap | Epic 7 - Storage Durability And Compatibility | done | passed | target/storage-compat/report.json |
| 15 | Roadmap | Epic 8 - Core Engine API Stability | done | passed | target/engine-api/report.json |
| 16 | Roadmap | Epic 9 - AQL Query Compatibility | done | passed | target/aql-compat/report.json |
| 17 | Roadmap | Epic 10 - Retrieval Quality And ANN History | done | passed | target/retrieval-quality/report.json |
| 18 | Roadmap | Epic 11 - ContextPack Quality | done | passed | target/context-pack-quality/report.json |
| 19 | Roadmap | Epic 12 - Verification Evaluation | done | passed | target/verification-quality/report.json |
| 20 | Roadmap | Epic 13 - HTTP Server Contract And Operations | done | passed | target/http-contract-ops/report.json |
| 21 | Roadmap | Epic 14 - CLI Productization | done | passed | target/cli-product/report.json |
| 22 | Roadmap | Epic 15 - SDK E2E And Release Train | done | passed | target/sdk-e2e-release/report.json |
| 23 | Roadmap | Epic 16 - Dashboard Product UI | done | passed | target/dashboard/product-ui-report.json |
| 24 | Roadmap | Epic 17 - Security Hardening | done | passed | target/security-hardening/report.json |
| 25 | Roadmap | Epic 18 - Observability | done | passed | target/observability/report.json |
| 26 | Roadmap | Epic 19 - Deployment And Upgrade | done | passed | target/deployment-upgrade/report.json |

## Completion status

- done: 26 / 26
- in_progress: 0 / 26
- blocked: 0 / 26

## Notes

- In active queue and roadmap some themes overlap, especially in evidence gates.
- All 26 entries above are currently mapped to passing local evidence reports.
- Remaining risk: evidence freshness (fresh re-run after large refactors).

## Next action

- Before next release cycle: run `make release-check` and `make production-evidence-sweep` on a clean environment.
