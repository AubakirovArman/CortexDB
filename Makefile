.PHONY: release-artifact-manifest-check
.PHONY: encrypted-backup-check
.PHONY: backup-restore-production-pack-check
.PHONY: migration-compatibility-v2-check
.PHONY: longmemeval-v1-official-repo longmemeval-v1-official-lite-env longmemeval-v1-official-data longmemeval-v1-cortexdb-retrieval longmemeval-v1-official-retrieval-metrics longmemeval-v1-official-generate longmemeval-v1-official-qa-score longmemeval-v1-official-score longmemeval-v1-package-submission longmemeval-v1-error-analysis longmemeval-v1-deepseek-flash-falsecase-check longmemeval-v1-deepseek-flash-diff longmemeval-v1-deepseek-flash-compact-50-check longmemeval-v1-deepseek-flash-compact-500-check longmemeval-v1-deepseek-flash-preference-check longmemeval-v1-deepseek-flash-single-session-user-check longmemeval-v1-deepseek-flash-multi-session-check longmemeval-v1-deepseek-flash-temporal-check
.PHONY: multihop-rag-official-repo multihop-rag-official-data multihop-rag-preflight multihop-rag-balanced-50 multihop-rag-local-50-check multihop-rag-cortexdb-retrieval-50 multihop-rag-official-retrieval-metrics-50 multihop-rag-cortexdb-retrieval-full multihop-rag-official-retrieval-metrics-full multihop-rag-retrieval-full-existing-check multihop-rag-qa-full-existing-check multihop-rag-qa-hybrid-full-retry-existing-check multihop-rag-qa-hybrid-full-retry-v4-existing-check multihop-rag-deepseek-qa-50 multihop-rag-deepseek-qa-50-cache-metrics multihop-rag-official-qa-metrics-50 multihop-rag-official-qa-metrics-existing-50 multihop-rag-qa-error-analysis-50 multihop-rag-deepseek-qa-full multihop-rag-deepseek-qa-temporal-50-v3 multihop-rag-official-qa-metrics-temporal-50-v3 multihop-rag-qa-error-analysis-temporal-50-v3 multihop-rag-deepseek-qa-temporal-50-v3-retry multihop-rag-official-qa-metrics-temporal-50-v3-retry multihop-rag-qa-error-analysis-temporal-50-v3-retry multihop-rag-deepseek-qa-temporal-50-v4-decompose-retry multihop-rag-official-qa-metrics-temporal-50-v4-decompose-retry multihop-rag-qa-error-analysis-temporal-50-v4-decompose-retry multihop-rag-deepseek-qa-temporal-chronology-50-v1 multihop-rag-official-qa-metrics-temporal-chronology-50-v1 multihop-rag-qa-error-analysis-temporal-chronology-50-v1 multihop-rag-deepseek-qa-temporal-v3 multihop-rag-deepseek-qa-temporal-v3-retry multihop-rag-deepseek-qa-comparison-50-retry multihop-rag-official-qa-metrics-comparison-50-retry multihop-rag-qa-error-analysis-comparison-50-retry multihop-rag-deepseek-qa-comparison-50-decompose-retry multihop-rag-official-qa-metrics-comparison-50-decompose-retry multihop-rag-qa-error-analysis-comparison-50-decompose-retry multihop-rag-deepseek-qa-comparison-v2-retry multihop-rag-deepseek-qa-comparison-v3-decompose-retry multihop-rag-combine-qa-full-hybrid multihop-rag-combine-qa-full-hybrid-retry multihop-rag-combine-qa-full-hybrid-retry-v4 multihop-rag-postprocess-hybrid-full-retry-v5 multihop-rag-combine-qa-full-hybrid-retry-v6 multihop-rag-official-qa-metrics-hybrid-full multihop-rag-official-qa-metrics-hybrid-full-retry multihop-rag-official-qa-metrics-hybrid-full-retry-v4 multihop-rag-official-qa-metrics-hybrid-full-retry-v5 multihop-rag-official-qa-metrics-hybrid-full-retry-v6 multihop-rag-official-qa-metrics-full multihop-rag-official-qa-metrics-existing-full multihop-rag-qa-error-analysis-full multihop-rag-qa-error-analysis-hybrid-full-retry multihop-rag-qa-error-analysis-hybrid-full-retry-v4 multihop-rag-qa-error-analysis-hybrid-full-retry-v5 multihop-rag-qa-error-analysis-hybrid-full-retry-v6
.PHONY: multihop-rag-temporal-subtype-analysis-v6
.PHONY: operations-runbook-check
.PHONY: service-manager-smoke-check
.PHONY: beta-landing-check
.PHONY: use-case-pack-check
.PHONY: contributor-onboarding-check
.PHONY: public-benchmarks-check
.PHONY: comparison-docs-check
.PHONY: agent-memory-demo-check
.PHONY: tool-registry-check
.PHONY: knowledge-graph-check
.PHONY: distributed-consensus-research-check
.PHONY: managed-cloud-feasibility-check
.PHONY: next-60-epics-audit next-60-epics-completion-check
.PHONY: check test sdk-check sdk-release-contract-check sdk-deprecation-check sdk-release-artifacts-check sdk-registry-gate-check sdk-productization-check openapi-check openapi-contract-check sdk-contract-check sdk-e2e-release-check migration-policy-check migration-compatibility-check storage-compat-check engine-api-check aql-compat-check retrieval-quality-check context-pack-quality-check verification-quality-check security-check rbac-policy-store-check quota-policy-check audit-chain-check security-hardening-check compliance-boundary-check observability-check deployment-upgrade-check http-contract-ops-check cli-product-check future-epic-design-check distributed-consensus-design-check managed-cloud-design-check enterprise-rbac-design-check hnsw-no-fallback-design-check llm-inference-design-check external-identity-design-check legal-verification-design-check distributed-consensus-check consensus-partition-soak-check consensus-failover-slo-check consensus-rejoin-check cloud-tenant-lifecycle-check cloud-backup-restore-check cloud-upgrade-check ann-production-no-fallback-check ann-production-slo-history-check ann-real-domain-history-check ann-public-corpus-history-check ann-graph-freshness-check llm-inference-contract-check llm-inference-safety-check llm-inference-smoke-check secrets-check oidc-auth-contract-check identity-policy-mapping-check auth-rotation-check legal-verification-dataset-check legal-verification-quality-check legal-citation-policy-check binary-release-package binary-release-validate binary-platform-matrix-check install-script-check binary-release-check beta-delta-check beta-foundation-check beta-rc-check beta-release-check production-hardening-check production-candidate-check production-v1-check public-claims-check load-smoke-check single-node-performance-check performance-trend-check tenant-recovery-check context-verify-quality-check dashboard-build dashboard-standalone-build dashboard-check dashboard-standalone-check dashboard-standalone-smoke dashboard-package dashboard-validate-package dashboard-release-check dashboard-product-check dashboard-smoke dashboard-screenshots ann-fixture-check ann-fixture-report ann-drift-check ann-drift-report ann-external-check ann-external-report ann-metric-matrix-check ann-metric-matrix-report ann-corpus-smoke-check ann-corpus-smoke-report ann-domain-corpus-check ann-domain-corpus-report ann-recall-probe-check ann-recall-probe-report ann-demo-domain-corpus-build ann-demo-domain-corpus-run ann-demo-domain-publish-baseline ann-demo-domain-package-baseline ann-demo-domain-validate-baseline-package ann-embedded-domain-corpus-build ann-embedded-domain-corpus-run ann-embedding-domain-export ann-embedding-domain-corpus-run ann-real-embedding-readiness ann-real-embedding-preflight ann-real-embedding-benchmark ann-real-embedding-compare ann-real-embedding-benchmark-and-compare ann-real-embedding-history-report ann-real-embedding-history-regression-check ann-real-embedding-publish-baseline ann-real-embedding-package-baseline ann-real-embedding-validate-baseline-package ann-real-embedding-release-check ann-slo-profile ann-scripts-check ann-convert-public-smoke ann-public-corpus-smoke ann-public-corpus-run ann-corpus-compare ann-corpus-run-smoke ann-history-report ann-history-regression-check ann-history-fixture-check ann-publish-baseline ann-package-baseline ann-validate-baseline-package ann-compare-baseline-bundle ann-release-evidence-check backup-drill-check backup-offsite-check crash-fault-check chaos-restart-check storage-soak-check storage-soak-history-check storage-soak-24h-campaign storage-soak-campaign-status storage-soak-campaign-watchdog storage-soak-epic-finalize replication-partition-check replication-lifecycle-check production-evidence-sweep smoke-test sdk-smoke-test rag-demo-smoke alpha-check release-check demo

ANN_FIXTURE_BASELINE ?= crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json
ANN_FIXTURE_REPORT ?= target/ann/ann_fixture_report.json
ANN_DRIFT_BASELINE ?= crates/cortex-engine/fixtures/ann_drift_baseline_v1.json
ANN_DRIFT_REPORT ?= target/ann/ann_drift_report.json
ANN_EXTERNAL_BASELINE ?= crates/cortex-engine/fixtures/ann_external_baseline_v1.json
ANN_EXTERNAL_REPORT ?= target/ann/ann_external_fixture_report.json
ANN_METRIC_MATRIX_BASELINE ?= crates/cortex-engine/fixtures/ann_metric_matrix_baseline_v1.json
ANN_METRIC_MATRIX_REPORT ?= target/ann/ann_metric_matrix_report.json
ANN_CORPUS_VECTORS ?= crates/cortex-engine/fixtures/ann_corpus_vectors_v1.jsonl
ANN_CORPUS_QUERIES ?= crates/cortex-engine/fixtures/ann_corpus_queries_v1.jsonl
ANN_CORPUS_GROUND_TRUTH ?= crates/cortex-engine/fixtures/ann_corpus_ground_truth_v1.jsonl
ANN_CORPUS_REPORT ?= target/ann/ann_corpus_report.json
ANN_CORPUS_GENERATED_GROUND_TRUTH ?= target/ann/generated_ground_truth.jsonl
ANN_DOMAIN_VECTORS ?= crates/cortex-engine/fixtures/ann_domain_vectors_v1.jsonl
ANN_DOMAIN_QUERIES ?= crates/cortex-engine/fixtures/ann_domain_queries_v1.jsonl
ANN_DOMAIN_GROUND_TRUTH ?= crates/cortex-engine/fixtures/ann_domain_ground_truth_v1.jsonl
ANN_DOMAIN_REPORT ?= target/ann/ann_domain_corpus_report.json
ANN_RECALL_PROBE_REPORT ?= target/ann/ann_recall_probe_report.json
ANN_RECALL_PROBE_ITERATIONS ?= 3
ANN_PRODUCTION_SLO_HISTORY_ROOT ?= target/ann/production-slo-history/runs
ANN_PRODUCTION_SLO_HISTORY_REPORT ?= $(ANN_PRODUCTION_SLO_HISTORY_ROOT)/history.json
ANN_PRODUCTION_SLO_HISTORY_RUNS ?= 10
ANN_PRODUCTION_SLO_HISTORY_P95_TOLERANCE_NANOS ?= 100000000
ANN_PRODUCTION_SLO_HISTORY_P99_TOLERANCE_NANOS ?= 150000000
ANN_PRODUCTION_SLO_HISTORY_MAX_TOLERANCE_NANOS ?= 200000000
ANN_DEMO_DOMAIN_SOURCE_ROOT ?= examples/datasets
ANN_DEMO_DOMAIN_EXTRA_SOURCE_ROOT ?= examples/rag_demo/data
ANN_DEMO_DOMAIN_OUTPUT_DIR ?= target/ann/demo-domain-corpus/converted
ANN_DEMO_DOMAIN_RUN_ROOT ?= target/ann/demo-domain-corpus/runs
ANN_DEMO_DOMAIN_RUN_ID ?= demo-domain
ANN_DEMO_DOMAIN_DIMENSION ?= 64
ANN_DEMO_DOMAIN_LIMIT ?= 5
ANN_DEMO_DOMAIN_SCALE ?= 1200
ANN_DEMO_DOMAIN_MAX_NEIGHBORS ?= 16
ANN_DEMO_DOMAIN_EF_SEARCH ?= 128
ANN_DEMO_DOMAIN_EF_CONSTRUCTION ?= 128
ANN_DEMO_DOMAIN_LAYER_COUNT ?= 4
ANN_DEMO_DOMAIN_BASELINE_ID ?= $(ANN_DEMO_DOMAIN_RUN_ID)
ANN_DEMO_DOMAIN_BASELINE_ROOT ?= target/ann/demo-domain-corpus/release-baselines
ANN_DEMO_DOMAIN_BASELINE_BUNDLE ?= $(ANN_DEMO_DOMAIN_BASELINE_ROOT)/$(ANN_DEMO_DOMAIN_BASELINE_ID)
ANN_DEMO_DOMAIN_BASELINE_ARCHIVE ?= $(ANN_DEMO_DOMAIN_BASELINE_ROOT)/$(ANN_DEMO_DOMAIN_BASELINE_ID).tar.gz
ANN_EMBEDDED_DOMAIN_SOURCE_ROOT ?=
ANN_EMBEDDED_DOMAIN_QUERIES ?=
ANN_EMBEDDED_DOMAIN_OUTPUT_DIR ?= target/ann/embedded-domain-corpus/converted
ANN_EMBEDDED_DOMAIN_RUN_ROOT ?= target/ann/embedded-domain-corpus/runs
ANN_EMBEDDED_DOMAIN_RUN_ID ?= embedded-domain
ANN_EMBEDDED_DOMAIN_METRIC ?= dot_product
ANN_EMBEDDED_DOMAIN_LIMIT ?= 10
ANN_EMBEDDED_DOMAIN_SLO_PROFILE ?= balanced
ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS ?=
ANN_EMBEDDED_DOMAIN_EF_SEARCH ?=
ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION ?=
ANN_EMBEDDED_DOMAIN_LAYER_COUNT ?=
ANN_EMBEDDING_SOURCE_ROOT ?=
ANN_EMBEDDING_QUERIES ?=
ANN_EMBEDDING_OUTPUT_DIR ?= target/ann/embedding-domain-corpus/export
ANN_EMBEDDING_RUN_ROOT ?= target/ann/embedding-domain-corpus/runs
ANN_EMBEDDING_RUN_ID ?= embedding-domain
ANN_EMBEDDING_PROVIDER ?= command
ANN_EMBEDDING_COMMAND ?=
ANN_EMBEDDING_NORMALIZATION ?= unit
ANN_EMBEDDING_SCALE ?= 32767
ANN_EMBEDDING_METRIC ?= cosine
ANN_EMBEDDING_LIMIT ?= 10
ANN_EMBEDDING_SLO_PROFILE ?= balanced
ANN_EMBEDDING_MAX_NEIGHBORS ?=
ANN_EMBEDDING_EF_SEARCH ?=
ANN_EMBEDDING_EF_CONSTRUCTION ?=
ANN_EMBEDDING_LAYER_COUNT ?=
ANN_REAL_EMBEDDING_SOURCE_ROOT ?=
ANN_REAL_EMBEDDING_QUERIES ?=
ANN_REAL_EMBEDDING_COMMAND ?= python3 scripts/ann/embed_text_command.py --require-model
ANN_REAL_EMBEDDING_OUTPUT_DIR ?= target/ann/real-embedding/export
ANN_REAL_EMBEDDING_RUN_ROOT ?= target/ann/real-embedding/runs
ANN_REAL_EMBEDDING_RUN_ID ?= real-embedding
ANN_REAL_EMBEDDING_READINESS_REPORT ?= target/ann/real-embedding/readiness.json
ANN_REAL_EMBEDDING_PREFLIGHT_REPORT ?= target/ann/real-embedding/preflight.json
ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST ?=
ANN_REAL_EMBEDDING_REQUIRE_API_KEY ?= false
ANN_REAL_EMBEDDING_NORMALIZATION ?= unit
ANN_REAL_EMBEDDING_SCALE ?= 32767
ANN_REAL_EMBEDDING_METRIC ?= cosine
ANN_REAL_EMBEDDING_LIMIT ?= 10
ANN_REAL_EMBEDDING_SLO_PROFILE ?= balanced
ANN_REAL_EMBEDDING_MAX_NEIGHBORS ?=
ANN_REAL_EMBEDDING_EF_SEARCH ?=
ANN_REAL_EMBEDDING_EF_CONSTRUCTION ?=
ANN_REAL_EMBEDDING_LAYER_COUNT ?=
ANN_REAL_EMBEDDING_BASELINE_REPORT ?=
ANN_REAL_EMBEDDING_CANDIDATE_REPORT ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)/report.json
ANN_REAL_EMBEDDING_COMPARISON ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)/baseline_comparison.json
ANN_REAL_EMBEDDING_HISTORY_REPORT ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/history.json
ANN_REAL_EMBEDDING_BASELINE_ID ?= $(ANN_REAL_EMBEDDING_RUN_ID)
ANN_REAL_EMBEDDING_BASELINE_ROOT ?= target/ann/real-embedding/release-baselines
ANN_REAL_EMBEDDING_BASELINE_BUNDLE ?= $(ANN_REAL_EMBEDDING_BASELINE_ROOT)/$(ANN_REAL_EMBEDDING_BASELINE_ID)
ANN_REAL_EMBEDDING_BASELINE_ARCHIVE ?= $(ANN_REAL_EMBEDDING_BASELINE_ROOT)/$(ANN_REAL_EMBEDDING_BASELINE_ID).tar.gz
ANN_REAL_EMBEDDING_REQUIRE_SOURCE_ARCHIVE ?= false
ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS ?= 1000000
ANN_REAL_EMBEDDING_MAX_P99_REGRESSION_NANOS ?= 2500000
ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS ?= 5000000
ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS ?= 3
RETRIEVAL_QUALITY_SOURCE_ROOT ?= examples/real_domains/investment_projects/corpus
RETRIEVAL_QUALITY_QUERIES ?= examples/real_domains/investment_projects/queries/queries.jsonl
RETRIEVAL_QUALITY_GROUND_TRUTH ?= examples/real_domains/investment_projects/queries/ground_truth.jsonl
RETRIEVAL_QUALITY_REPORT ?= target/retrieval-quality/report.json
RETRIEVAL_BETA_REPORT ?= target/retrieval-quality/beta-report.json
RETRIEVAL_QUALITY_DASHBOARD ?= target/retrieval-quality/dashboard.html
RETRIEVAL_QUALITY_MIN_DOCS ?= 50
RETRIEVAL_QUALITY_MIN_CHUNKS ?= 150
RETRIEVAL_QUALITY_MIN_QUERIES ?= 40
CONTEXT_PACK_QUALITY_FIXTURE ?= examples/eval/context_pack_quality.jsonl
CONTEXT_PACK_QUALITY_REPORT ?= target/context-pack-quality/report.json
VERIFICATION_QUALITY_FIXTURE ?= examples/eval/verification_cases.jsonl
VERIFICATION_QUALITY_REPORT ?= target/verification-quality/report.json
HTTP_CONTRACT_OPS_REPORT ?= target/http-contract-ops/report.json
CLI_PRODUCT_REPORT ?= target/cli-product/report.json
OPERATIONS_RUNBOOK_REPORT ?= target/operations-runbook/report.json
SDK_E2E_RELEASE_REPORT ?= target/sdk-e2e-release/report.json
SDK_RELEASE_ARTIFACT_ROOT ?= target/sdk-release-artifacts
SDK_RELEASE_ARTIFACT_REPORT ?= $(SDK_RELEASE_ARTIFACT_ROOT)/report.json
SDK_REGISTRY_GATE_REPORT ?= target/sdk-registry-gate/report.json
SDK_PRODUCTIZATION_REPORT ?= target/sdk-productization/report.json
SECURITY_REPORT ?= target/security/report.json
SECURITY_HARDENING_REPORT ?= target/security-hardening/report.json
COMPLIANCE_BOUNDARY_REPORT ?= target/compliance-boundary/report.json
RBAC_POLICY_STORE_REPORT ?= target/enterprise-rbac/rbac-policy-store.json
QUOTA_POLICY_REPORT ?= target/enterprise-rbac/quota-policy.json
AUDIT_CHAIN_REPORT ?= target/enterprise-rbac/audit-chain.json
CONSENSUS_GATE_ROOT ?= target/consensus
CONSENSUS_CORE_REPORT ?= $(CONSENSUS_GATE_ROOT)/distributed-consensus.json
CONSENSUS_PARTITION_SOAK_REPORT ?= $(CONSENSUS_GATE_ROOT)/partition-soak.json
CONSENSUS_FAILOVER_SLO_REPORT ?= $(CONSENSUS_GATE_ROOT)/failover-slo.json
CONSENSUS_REJOIN_REPORT ?= $(CONSENSUS_GATE_ROOT)/rejoin.json
MANAGED_CLOUD_GATE_ROOT ?= target/managed-cloud
MANAGED_CLOUD_TENANT_REPORT ?= $(MANAGED_CLOUD_GATE_ROOT)/tenant-lifecycle.json
MANAGED_CLOUD_BACKUP_REPORT ?= $(MANAGED_CLOUD_GATE_ROOT)/backup-restore.json
MANAGED_CLOUD_UPGRADE_REPORT ?= $(MANAGED_CLOUD_GATE_ROOT)/upgrade.json
MANAGED_CLOUD_FEASIBILITY_REPORT ?= $(MANAGED_CLOUD_GATE_ROOT)/feasibility-summary.json
HNSW_NO_FALLBACK_GATE_ROOT ?= target/hnsw-no-fallback
HNSW_PRODUCTION_NO_FALLBACK_REPORT ?= $(HNSW_NO_FALLBACK_GATE_ROOT)/production-no-fallback.json
HNSW_REAL_DOMAIN_HISTORY_REPORT ?= $(HNSW_NO_FALLBACK_GATE_ROOT)/real-domain-history.json
HNSW_PUBLIC_CORPUS_HISTORY_REPORT ?= $(HNSW_NO_FALLBACK_GATE_ROOT)/public-corpus-history.json
HNSW_GRAPH_FRESHNESS_REPORT ?= $(HNSW_NO_FALLBACK_GATE_ROOT)/graph-freshness.json
LLM_INFERENCE_GATE_ROOT ?= target/llm-inference
LLM_INFERENCE_CONTRACT_REPORT ?= $(LLM_INFERENCE_GATE_ROOT)/contract.json
LLM_INFERENCE_SAFETY_REPORT ?= $(LLM_INFERENCE_GATE_ROOT)/safety.json
LLM_INFERENCE_SMOKE_REPORT ?= $(LLM_INFERENCE_GATE_ROOT)/smoke.json
LLM_INFERENCE_SECRETS_REPORT ?= $(LLM_INFERENCE_GATE_ROOT)/secrets.json
EXTERNAL_IDENTITY_GATE_ROOT ?= target/external-identity
OIDC_AUTH_CONTRACT_REPORT ?= $(EXTERNAL_IDENTITY_GATE_ROOT)/oidc-contract.json
IDENTITY_POLICY_MAPPING_REPORT ?= $(EXTERNAL_IDENTITY_GATE_ROOT)/policy-mapping.json
AUTH_ROTATION_REPORT ?= $(EXTERNAL_IDENTITY_GATE_ROOT)/rotation.json
LEGAL_VERIFICATION_GATE_ROOT ?= target/legal-verification
LEGAL_VERIFICATION_DATASET_REPORT ?= $(LEGAL_VERIFICATION_GATE_ROOT)/dataset.json
LEGAL_VERIFICATION_QUALITY_REPORT ?= $(LEGAL_VERIFICATION_GATE_ROOT)/quality.json
LEGAL_CITATION_POLICY_REPORT ?= $(LEGAL_VERIFICATION_GATE_ROOT)/citation-policy.json
OBSERVABILITY_REPORT ?= target/observability/report.json
DEPLOYMENT_UPGRADE_REPORT ?= target/deployment-upgrade/report.json
SERVICE_MANAGER_REPORT ?= target/service-manager-smoke/report.json
ANN_BASELINE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_CANDIDATE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_REPORT_COMPARISON ?= target/ann/ann_report_comparison.json
ANN_CORPUS_RUN_ID ?= smoke
ANN_CORPUS_RUN_ROOT ?= target/ann/corpus-runs
ANN_HISTORY_ROOT ?= $(ANN_CORPUS_RUN_ROOT)
ANN_HISTORY_REPORT ?= $(ANN_HISTORY_ROOT)/history.json
ANN_HISTORY_CLEAN_FIXTURE ?= crates/cortex-engine/fixtures/ann_history_clean_v1.json
ANN_HISTORY_RECALL_REGRESSION_FIXTURE ?= crates/cortex-engine/fixtures/ann_history_recall_regression_v1.json
ANN_HISTORY_LATENCY_REGRESSION_FIXTURE ?= crates/cortex-engine/fixtures/ann_history_latency_regression_v1.json
ANN_BASELINE_RUN_ID ?= $(ANN_CORPUS_RUN_ID)
ANN_BASELINE_ID ?= $(ANN_BASELINE_RUN_ID)
ANN_BASELINE_ROOT ?= target/ann/release-baselines
ANN_CANDIDATE_RUN_ID ?= $(ANN_CORPUS_RUN_ID)
ANN_BASELINE_BUNDLE ?= $(ANN_BASELINE_ROOT)/$(ANN_BASELINE_ID)
ANN_BASELINE_BUNDLE_REPORT ?= $(ANN_BASELINE_BUNDLE)/report.json
ANN_BASELINE_BUNDLE_COMPARISON ?= $(ANN_HISTORY_ROOT)/$(ANN_CANDIDATE_RUN_ID)/baseline_comparison.json
ANN_BASELINE_ARCHIVE ?= $(ANN_BASELINE_ROOT)/$(ANN_BASELINE_ID).tar.gz
ANN_RELEASE_EVIDENCE_ROOT ?= target/ann/release-evidence
ANN_RELEASE_EVIDENCE_RUN_ROOT ?= $(ANN_RELEASE_EVIDENCE_ROOT)/corpus-runs
ANN_RELEASE_EVIDENCE_RUN_ID ?= smoke
ANN_RELEASE_EVIDENCE_BASELINE_ID ?= $(ANN_RELEASE_EVIDENCE_RUN_ID)
ANN_RELEASE_EVIDENCE_BASELINE_ROOT ?= $(ANN_RELEASE_EVIDENCE_ROOT)/release-baselines
ANN_RELEASE_EVIDENCE_BASELINE_BUNDLE ?= $(ANN_RELEASE_EVIDENCE_BASELINE_ROOT)/$(ANN_RELEASE_EVIDENCE_BASELINE_ID)
ANN_RELEASE_EVIDENCE_BASELINE_ARCHIVE ?= $(ANN_RELEASE_EVIDENCE_BASELINE_ROOT)/$(ANN_RELEASE_EVIDENCE_BASELINE_ID).tar.gz
ANN_MAX_P95_REGRESSION_NANOS ?= 0
ANN_MAX_P99_REGRESSION_NANOS ?= 0
ANN_MAX_MAX_REGRESSION_NANOS ?= 0
DASHBOARD_PACKAGE_ID ?= dashboard-v1
DASHBOARD_PACKAGE_ARCHIVE ?= target/dashboard/$(DASHBOARD_PACKAGE_ID).tar.gz
DASHBOARD_PRODUCT_REPORT ?= target/dashboard/product-ui-report.json
BINARY_RELEASE_PLATFORM ?= $(shell uname -s | tr '[:upper:]' '[:lower:]')-$(shell uname -m)
BINARY_RELEASE_VERSION ?= dev
BINARY_RELEASE_ID ?= cortexdb-$(BINARY_RELEASE_VERSION)-$(BINARY_RELEASE_PLATFORM)
BINARY_RELEASE_ARCHIVE ?= target/release-artifacts/$(BINARY_RELEASE_ID).tar.gz
BINARY_PLATFORM_MATRIX_REPORT ?= target/binary-platform-matrix/report.json
RELEASE_ARTIFACT_MANIFEST ?= target/release-artifact-manifest/manifest.json
RELEASE_ARTIFACT_MANIFEST_REPORT ?= target/release-artifact-manifest/report.json
ANN_PUBLIC_SOURCE ?=
ANN_PUBLIC_DATASET_ID ?= public-ann
ANN_PUBLIC_FORMAT ?= fvecs
ANN_PUBLIC_METRIC ?= cosine
ANN_PUBLIC_RUN_ID ?= $(ANN_PUBLIC_DATASET_ID)-$(ANN_CORPUS_RUN_ID)
ANN_PUBLIC_OUTPUT_ROOT ?= target/ann/public-corpora
ANN_PUBLIC_NORMALIZATION ?= unit
ANN_PUBLIC_SCALE ?= 32767
ANN_PUBLIC_LIMIT ?= 10
ANN_PUBLIC_MAX_NEIGHBORS ?= 8
ANN_PUBLIC_EF_SEARCH ?= 64
ANN_PUBLIC_LAYER_COUNT ?= 4
ANN_PUBLIC_MAX_P99_LATENCY_NANOS ?= 200000000
BACKUP_DRILL_ROOT ?= target/backup-drill
BACKUP_DRILL_REPORT ?= $(BACKUP_DRILL_ROOT)/report.json
BACKUP_DRILL_KEEP_LATEST ?= 2
BACKUP_DRILL_PREFIX ?= cortexdb-
BACKUP_OFFSITE_ROOT ?= target/backup-offsite
BACKUP_OFFSITE_REPORT ?= $(BACKUP_OFFSITE_ROOT)/report.json
BACKUP_OFFSITE_ID ?= cortexdb-20260530T000000Z
BACKUP_RESTORE_PACK_ROOT ?= target/backup-restore-production-pack
BACKUP_RESTORE_PACK_REPORT ?= $(BACKUP_RESTORE_PACK_ROOT)/report.json
LOAD_SMOKE_ROOT ?= target/load-smoke
LOAD_SMOKE_REPORT ?= $(LOAD_SMOKE_ROOT)/report.json
LOAD_SMOKE_CELLS ?= 100
LOAD_SMOKE_READS ?= 100
LOAD_SMOKE_SEARCHES ?= 20
LOAD_SMOKE_CONTEXTS ?= 5
LOAD_SMOKE_VERIFIES ?= 5
LOAD_SMOKE_WORKERS ?= 8
LONGMEMEVAL_V1_OFFICIAL_REPO ?= target/external-benchmarks/longmemeval
LONGMEMEVAL_V1_VENV ?= target/longmemeval-v1/.venv
LONGMEMEVAL_V1_PYTHON ?= $(LONGMEMEVAL_V1_VENV)/bin/python
LONGMEMEVAL_V1_DATA_ROOT ?= target/longmemeval-v1/data
LONGMEMEVAL_V1_DATA_FILE ?= $(LONGMEMEVAL_V1_DATA_ROOT)/longmemeval_s_cleaned.json
LONGMEMEVAL_V1_DATA_MANIFEST ?= $(LONGMEMEVAL_V1_DATA_ROOT)/manifest.json
LONGMEMEVAL_V1_DATA_SPLIT ?= s
LONGMEMEVAL_V1_OUTPUT_ROOT ?= target/longmemeval-v1/cortexdb
LONGMEMEVAL_V1_RETRIEVAL_LOG ?= $(LONGMEMEVAL_V1_OUTPUT_ROOT)/$(basename $(notdir $(LONGMEMEVAL_V1_DATA_FILE)))_cortexdb_$(LONGMEMEVAL_V1_GRANULARITY)_retrieval.jsonl
LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT ?= $(LONGMEMEVAL_V1_OUTPUT_ROOT)/official_retrieval_metrics.txt
LONGMEMEVAL_V1_GRANULARITY ?= session
LONGMEMEVAL_V1_INDEX_MODE ?= user
LONGMEMEVAL_V1_CONTEXT_MODE ?= user
LONGMEMEVAL_V1_MAX_TURN_CHARS ?= 900
LONGMEMEVAL_V1_MAX_SESSION_CHARS ?= 4000
LONGMEMEVAL_V1_TOPK ?= 10
LONGMEMEVAL_V1_LIMIT ?=
LONGMEMEVAL_V1_READER_MODEL_NAME ?= gpt-4o-2024-08-06
LONGMEMEVAL_V1_READER_MODEL_ALIAS ?= gpt-4o
LONGMEMEVAL_V1_READER_BASE_URL ?=
LONGMEMEVAL_V1_READER_OPENAI_KEY ?= $(OPENAI_API_KEY)
LONGMEMEVAL_V1_GENERATION_ROOT ?= target/longmemeval-v1/generation
LONGMEMEVAL_V1_GENERATION_TOPK ?= 10
LONGMEMEVAL_V1_HYPOTHESIS_FILE ?=
LONGMEMEVAL_V1_EVAL_MODEL ?= gpt-4o
LONGMEMEVAL_V1_PACKAGE_NAME ?= cortexdb-longmemeval-v1-official-gpt4o
LONGMEMEVAL_V1_SUBMISSION_ROOT ?= target/longmemeval-v1/submission
LONGMEMEVAL_V1_ANALYSIS_ROOT ?= target/longmemeval-v1/analysis
LONGMEMEVAL_V1_FALSECASE_ROOT ?= target/longmemeval-v1/targeted-compact-falsecases
LONGMEMEVAL_V1_DEEPSEEK_ROOT ?= target/longmemeval-v1/targeted-deepseek-flash-thinking-disabled
LONGMEMEVAL_V1_DEEPSEEK_MODEL ?= deepseek-v4-flash
LONGMEMEVAL_V1_DEEPSEEK_GENERATION_THINKING ?= disabled
LONGMEMEVAL_V1_DEEPSEEK_JUDGE_THINKING ?= disabled
LONGMEMEVAL_V1_DEEPSEEK_FLASH_IMPLICIT_ROOT ?= target/longmemeval-v1/targeted-deepseek-flash
LONGMEMEVAL_V1_DEEPSEEK_FLASH_DIFF_ROOT ?= target/longmemeval-v1/deepseek-flash-diff
LONGMEMEVAL_V1_DEEPSEEK_COMPACT_500_ROOT ?= target/longmemeval-v1/deepseek-flash-compact-500-pref-ms-temporal-user-aware-thinking-disabled
LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG ?= target/longmemeval-v1/cortexdb-compact-context/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl
LONGMEMEVAL_V1_COMPACT_50_LIMIT ?= 50
LONGMEMEVAL_V1_COMPACT_50_ROOT ?= target/longmemeval-v1/deepseek-flash-compact-50-pref-ms-temporal-user-aware-thinking-disabled
LONGMEMEVAL_V1_COMPACT_50_INPUT_ROOT ?= target/longmemeval-v1/compact-50-balanced-input
LONGMEMEVAL_V1_COMPACT_50_RETRIEVAL ?= $(LONGMEMEVAL_V1_COMPACT_50_INPUT_ROOT)/compact_50_retrieval.jsonl
LONGMEMEVAL_V1_COMPACT_50_REFERENCE ?= $(LONGMEMEVAL_V1_COMPACT_50_INPUT_ROOT)/compact_50_reference.json
LONGMEMEVAL_V1_PREFERENCE_ROOT ?= target/longmemeval-v1/preference-format-check-flash
LONGMEMEVAL_V1_PREFERENCE_INPUT_ROOT ?= target/longmemeval-v1/preference-format-check-input
LONGMEMEVAL_V1_PREFERENCE_RETRIEVAL ?= $(LONGMEMEVAL_V1_PREFERENCE_INPUT_ROOT)/preference_retrieval.jsonl
LONGMEMEVAL_V1_PREFERENCE_REFERENCE ?= $(LONGMEMEVAL_V1_PREFERENCE_INPUT_ROOT)/preference_reference.json
LONGMEMEVAL_V1_SINGLE_SESSION_USER_ROOT ?= target/longmemeval-v1/single-session-user-format-check-flash-v2
LONGMEMEVAL_V1_SINGLE_SESSION_USER_INPUT_ROOT ?= target/longmemeval-v1/single-session-user-format-check-input
LONGMEMEVAL_V1_SINGLE_SESSION_USER_RETRIEVAL ?= $(LONGMEMEVAL_V1_SINGLE_SESSION_USER_INPUT_ROOT)/single_session_user_retrieval.jsonl
LONGMEMEVAL_V1_SINGLE_SESSION_USER_REFERENCE ?= $(LONGMEMEVAL_V1_SINGLE_SESSION_USER_INPUT_ROOT)/single_session_user_reference.json
LONGMEMEVAL_V1_MULTI_SESSION_ROOT ?= target/longmemeval-v1/multi-session-format-check-flash
LONGMEMEVAL_V1_MULTI_SESSION_INPUT_ROOT ?= target/longmemeval-v1/multi-session-format-check-input
LONGMEMEVAL_V1_MULTI_SESSION_RETRIEVAL ?= $(LONGMEMEVAL_V1_MULTI_SESSION_INPUT_ROOT)/multi_session_retrieval.jsonl
LONGMEMEVAL_V1_MULTI_SESSION_REFERENCE ?= $(LONGMEMEVAL_V1_MULTI_SESSION_INPUT_ROOT)/multi_session_reference.json
LONGMEMEVAL_V1_TEMPORAL_ROOT ?= target/longmemeval-v1/temporal-reasoning-format-check-flash
LONGMEMEVAL_V1_TEMPORAL_INPUT_ROOT ?= target/longmemeval-v1/temporal-reasoning-format-check-input
LONGMEMEVAL_V1_TEMPORAL_RETRIEVAL ?= $(LONGMEMEVAL_V1_TEMPORAL_INPUT_ROOT)/temporal_retrieval.jsonl
LONGMEMEVAL_V1_TEMPORAL_REFERENCE ?= $(LONGMEMEVAL_V1_TEMPORAL_INPUT_ROOT)/temporal_reference.json
MULTIHOP_RAG_ROOT ?= target/multihop-rag
MULTIHOP_RAG_DATA_ROOT ?= $(MULTIHOP_RAG_ROOT)/data
MULTIHOP_RAG_QUERY_FILE ?= $(MULTIHOP_RAG_DATA_ROOT)/MultiHopRAG.json
MULTIHOP_RAG_CORPUS_FILE ?= $(MULTIHOP_RAG_DATA_ROOT)/corpus.json
MULTIHOP_RAG_DATA_MANIFEST ?= $(MULTIHOP_RAG_DATA_ROOT)/manifest.json
MULTIHOP_RAG_PREFLIGHT_REPORT ?= $(MULTIHOP_RAG_ROOT)/preflight_report.json
MULTIHOP_RAG_SUBSET_ROOT ?= $(MULTIHOP_RAG_ROOT)/subsets
MULTIHOP_RAG_SUBSET_LIMIT ?= 50
MULTIHOP_RAG_SUBSET_PREFIX ?= balanced_50
MULTIHOP_RAG_OFFICIAL_REPO ?= target/external-benchmarks/multihop-rag
MULTIHOP_RAG_DB_50 ?= $(MULTIHOP_RAG_ROOT)/cortexdb-50
MULTIHOP_RAG_DB_FULL ?= $(MULTIHOP_RAG_ROOT)/cortexdb-full
MULTIHOP_RAG_RETRIEVAL_50 ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_balanced_50_retrieval.json
MULTIHOP_RAG_RETRIEVAL_FULL ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_full_retrieval.json
MULTIHOP_RAG_RETRIEVAL_50_REPORT ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_balanced_50_report.json
MULTIHOP_RAG_RETRIEVAL_FULL_REPORT ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_full_report.json
MULTIHOP_RAG_RETRIEVAL_50_METRICS ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_balanced_50_metrics.txt
MULTIHOP_RAG_RETRIEVAL_FULL_METRICS ?= $(MULTIHOP_RAG_ROOT)/retrieval/cortexdb_full_metrics.txt
MULTIHOP_RAG_TOPK ?= 10
MULTIHOP_RAG_QA_MODEL ?= deepseek-v4-flash
MULTIHOP_RAG_QA_BASE_URL ?= https://api.deepseek.com
MULTIHOP_RAG_QA_PROMPT_STYLE ?= multihop-v2
MULTIHOP_RAG_QA_50_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-balanced-50-v2
MULTIHOP_RAG_QA_50_CACHE_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-balanced-50-cache-metrics
MULTIHOP_RAG_QA_FULL_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v2
MULTIHOP_RAG_QA_TEMPORAL_50_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-50-v3
MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-50-v3-retry
MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-50-v4-decompose-retry
MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-chronology-50-v1
MULTIHOP_RAG_QA_TEMPORAL_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-v3
MULTIHOP_RAG_QA_TEMPORAL_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-temporal-v3-retry
MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-comparison-50-retry
MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-comparison-50-decompose-retry
MULTIHOP_RAG_QA_COMPARISON_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-comparison-v2-retry
MULTIHOP_RAG_QA_COMPARISON_DECOMPOSE_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-comparison-v3-decompose-retry
MULTIHOP_RAG_QA_HYBRID_FULL_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v3-hybrid
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v3-hybrid-retry
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v4-hybrid-retry
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v5-hybrid-retry-normalized
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT ?= $(MULTIHOP_RAG_ROOT)/qa/deepseek-full-v6-hybrid-decompose-normalized
MULTIHOP_RAG_TEMPORAL_GATE_LIMIT ?= 50
MULTIHOP_RAG_TEMPORAL_CHRONOLOGY_GATE_LIMIT ?= 50
MULTIHOP_RAG_COMPARISON_GATE_LIMIT ?= 50
MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT ?= 10
MULTIHOP_RAG_QA_TOPK_CONTEXT ?= 6
MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC ?= 1200
MULTIHOP_RAG_QA_WORKERS ?= 4
MULTIHOP_RAG_QA_50_METRICS ?= $(MULTIHOP_RAG_QA_50_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_50_CACHE_METRICS ?= $(MULTIHOP_RAG_QA_50_CACHE_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_TEMPORAL_50_METRICS ?= $(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_METRICS ?= $(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_FULL_METRICS ?= $(MULTIHOP_RAG_QA_FULL_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_HYBRID_FULL_METRICS ?= $(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_METRICS ?= $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_METRICS ?= $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_METRICS ?= $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_METRICS ?= $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/official_qa_metrics.txt
MULTIHOP_RAG_QA_50_ANALYSIS ?= $(MULTIHOP_RAG_QA_50_ROOT)/qa_error_analysis.json
MULTIHOP_RAG_QA_FULL_ANALYSIS ?= $(MULTIHOP_RAG_QA_FULL_ROOT)/qa_error_analysis.json
DEEPSEEK_KEY_FILE ?= /mnt/hf_model_weights/arman/3bit/.deepseek
SINGLE_NODE_PERF_ROOT ?= target/single-node-performance
SINGLE_NODE_PERF_REPORT ?= $(SINGLE_NODE_PERF_ROOT)/report.json
SINGLE_NODE_PERF_CELLS ?= 500
SINGLE_NODE_PERF_MAX_TOTAL_MS ?= 30000
PERFORMANCE_TREND_ROOT ?= target/performance-trends
PERFORMANCE_TREND_REPORT ?= $(PERFORMANCE_TREND_ROOT)/report.json
PERFORMANCE_HISTORY_ROOT ?= fixtures/performance/history
TENANT_RECOVERY_ROOT ?= target/tenant-recovery
TENANT_RECOVERY_REPORT ?= $(TENANT_RECOVERY_ROOT)/report.json
CRASH_FAULT_ROOT ?= target/crash-fault
CRASH_FAULT_REPORT ?= $(CRASH_FAULT_ROOT)/report.json
CHAOS_RESTART_ROOT ?= target/chaos-restart
CHAOS_RESTART_REPORT ?= $(CHAOS_RESTART_ROOT)/report.json
CHAOS_RESTART_SEED ?= 20260530
CHAOS_RESTART_STEPS ?= 24
STORAGE_SOAK_ROOT ?= target/storage-soak
STORAGE_SOAK_REPORT ?= $(STORAGE_SOAK_ROOT)/report.json
STORAGE_SOAK_CYCLES ?= 3
STORAGE_SOAK_CELLS_PER_CYCLE ?= 5
STORAGE_SOAK_KILL_DELAY_MS ?= 15
STORAGE_SOAK_HISTORY_ROOT ?= target/storage-soak-history
STORAGE_SOAK_HISTORY_REPORT ?= $(STORAGE_SOAK_HISTORY_ROOT)/report.json
STORAGE_SOAK_HISTORY_FILE ?= $(STORAGE_SOAK_HISTORY_ROOT)/history.jsonl
STORAGE_SOAK_HISTORY_MIN_RUNS ?= 1
STORAGE_SOAK_HISTORY_MIN_HOURS ?= 0
STORAGE_SOAK_CAMPAIGN_REPORT ?= $(STORAGE_SOAK_HISTORY_ROOT)/campaign.json
STORAGE_SOAK_CAMPAIGN_TARGET_HOURS ?= 24
STORAGE_SOAK_CAMPAIGN_MAX_RUNS ?= 100000
STORAGE_SOAK_CAMPAIGN_CYCLES ?= 20
STORAGE_SOAK_CAMPAIGN_CELLS_PER_CYCLE ?= 50
STORAGE_SOAK_CAMPAIGN_STATUS_FORMAT ?= text
REPLICATION_PARTITION_ROOT ?= target/replication-partition
REPLICATION_PARTITION_REPORT ?= $(REPLICATION_PARTITION_ROOT)/report.json
REPLICATION_LIFECYCLE_ROOT ?= target/replication-lifecycle
REPLICATION_LIFECYCLE_REPORT ?= $(REPLICATION_LIFECYCLE_ROOT)/report.json
PRODUCTION_EVIDENCE_ROOT ?= target/production-evidence
PRODUCTION_EVIDENCE_REPORT ?= $(PRODUCTION_EVIDENCE_ROOT)/report.json
BETA_FOUNDATION_ROOT ?= target/beta-foundation
BETA_FOUNDATION_REPORT ?= $(BETA_FOUNDATION_ROOT)/report.json
BETA_RC_ROOT ?= target/beta-rc
BETA_RC_REPORT ?= $(BETA_RC_ROOT)/report.json
BETA_RELEASE_ROOT ?= target/beta-release
BETA_RELEASE_REPORT ?= $(BETA_RELEASE_ROOT)/report.json
BETA_RELEASE_ARCHIVE ?= $(BETA_RELEASE_ROOT)/evidence.tar.gz
BETA_LANDING_REPORT ?= target/beta-landing/report.json
USE_CASE_PACK_REPORT ?= target/use-case-packs/report.json
CONTRIBUTOR_ONBOARDING_REPORT ?= target/contributor-onboarding/report.json
PUBLIC_BENCHMARKS_REPORT ?= target/public-benchmarks/report.json
COMPARISON_DOCS_REPORT ?= target/comparison-docs/report.json
AGENT_MEMORY_DEMO_REPORT ?= target/agent-memory-demo/report.json
TOOL_REGISTRY_REPORT ?= target/tool-registry/report.json
KNOWLEDGE_GRAPH_REPORT ?= target/knowledge-graph/report.json
CONSENSUS_RESEARCH_REPORT ?= target/consensus/research-summary.json
PRODUCTION_HARDENING_ROOT ?= target/production-hardening
PRODUCTION_HARDENING_REPORT ?= $(PRODUCTION_HARDENING_ROOT)/report.json
PRODUCTION_CANDIDATE_ROOT ?= target/production-candidate
PRODUCTION_CANDIDATE_REPORT ?= $(PRODUCTION_CANDIDATE_ROOT)/report.json
PRODUCTION_V1_ROOT ?= target/production-v1
PRODUCTION_V1_REPORT ?= $(PRODUCTION_V1_ROOT)/report.json
PUBLIC_CLAIMS_REPORT ?= target/public-claims/report.json
STORAGE_COMPAT_ROOT ?= target/storage-compat
STORAGE_COMPAT_REPORT ?= $(STORAGE_COMPAT_ROOT)/report.json
MIGRATION_HISTORICAL_RESTORE_ROOT ?= target/migration-historical-restore
MIGRATION_HISTORICAL_RESTORE_REPORT ?= $(MIGRATION_HISTORICAL_RESTORE_ROOT)/report.json
MIGRATION_UPGRADE_MATRIX_V2_ROOT ?= target/migration-upgrade-matrix-v2
MIGRATION_UPGRADE_MATRIX_V2_REPORT ?= $(MIGRATION_UPGRADE_MATRIX_V2_ROOT)/report.json
ENGINE_API_ROOT ?= target/engine-api
ENGINE_API_REPORT ?= $(ENGINE_API_ROOT)/report.json
AQL_COMPAT_ROOT ?= target/aql-compat
AQL_COMPAT_REPORT ?= $(AQL_COMPAT_ROOT)/report.json
FUTURE_EPIC_REPORT ?= target/future-epics/report.json
FUTURE_EPIC_ROOT ?= target/future-epics

check:
	cargo check --workspace

test:
	cargo test --workspace

sdk-check:
	./sdk/publish/check.sh

sdk-release-contract-check:
	python3 scripts/check_sdk_release_contract.py

sdk-deprecation-check:
	python3 scripts/check_sdk_deprecation_policy.py

sdk-release-artifacts-check:
	python3 scripts/sdk_release_artifacts_check.py --root "$(SDK_RELEASE_ARTIFACT_ROOT)" --report "$(SDK_RELEASE_ARTIFACT_REPORT)"

sdk-registry-gate-check:
	python3 scripts/sdk_registry_gate_check.py --report "$(SDK_REGISTRY_GATE_REPORT)"

openapi-check:
	python3 scripts/check_openapi_coverage.py

openapi-contract-check:
	python3 scripts/check_openapi_contract.py
	python3 scripts/check_error_taxonomy_contract.py

sdk-contract-check:
	python3 scripts/check_sdk_contract.py

sdk-e2e-release-check:
	$(MAKE) sdk-release-contract-check
	$(MAKE) sdk-deprecation-check
	$(MAKE) sdk-release-artifacts-check
	$(MAKE) sdk-registry-gate-check
	$(MAKE) sdk-contract-check
	python3 scripts/sdk_e2e_release_check.py --report "$(SDK_E2E_RELEASE_REPORT)"

sdk-productization-check: sdk-e2e-release-check
	python3 scripts/sdk_productization_check.py --report "$(SDK_PRODUCTIZATION_REPORT)"

migration-policy-check:
	python3 scripts/check_migration_policy.py

migration-compatibility-check:
	python3 scripts/check_migration_compatibility.py
	python3 scripts/migration_historical_restore_check.py --root "$(MIGRATION_HISTORICAL_RESTORE_ROOT)" --report "$(MIGRATION_HISTORICAL_RESTORE_REPORT)"
	python3 scripts/migration_upgrade_matrix_v2_check.py --root "$(MIGRATION_UPGRADE_MATRIX_V2_ROOT)" --report "$(MIGRATION_UPGRADE_MATRIX_V2_REPORT)"

migration-compatibility-v2-check:
	python3 scripts/migration_upgrade_matrix_v2_check.py --root "$(MIGRATION_UPGRADE_MATRIX_V2_ROOT)" --report "$(MIGRATION_UPGRADE_MATRIX_V2_REPORT)"

storage-compat-check:
	python3 scripts/storage_compat_check.py --root "$(STORAGE_COMPAT_ROOT)" --report "$(STORAGE_COMPAT_REPORT)"

engine-api-check:
	python3 scripts/engine_api_check.py --root "$(ENGINE_API_ROOT)" --report "$(ENGINE_API_REPORT)"

aql-compat-check:
	python3 scripts/aql_compat_check.py --root "$(AQL_COMPAT_ROOT)" --report "$(AQL_COMPAT_REPORT)"

retrieval-quality-check:
	cd examples/real_domains/investment_projects && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/support_tickets && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/legal_policies && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/technical_docs && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	$(MAKE) ann-real-embedding-history-regression-check
	python3 scripts/retrieval_quality_check.py --source-root "$(RETRIEVAL_QUALITY_SOURCE_ROOT)" --queries "$(RETRIEVAL_QUALITY_QUERIES)" --ground-truth "$(RETRIEVAL_QUALITY_GROUND_TRUTH)" --history "$(ANN_REAL_EMBEDDING_HISTORY_REPORT)" --benchmarks docs/BENCHMARKS.md --output "$(RETRIEVAL_QUALITY_REPORT)" --min-docs $(RETRIEVAL_QUALITY_MIN_DOCS) --min-chunks $(RETRIEVAL_QUALITY_MIN_CHUNKS) --min-queries $(RETRIEVAL_QUALITY_MIN_QUERIES) --min-history-runs $(ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS)
	python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output "$(RETRIEVAL_BETA_REPORT)" --min-domains 4 --repeat-runs 5
	python3 scripts/retrieval_quality_dashboard.py --report "$(RETRIEVAL_QUALITY_REPORT)" --beta-report "$(RETRIEVAL_BETA_REPORT)" --output "$(RETRIEVAL_QUALITY_DASHBOARD)"

context-pack-quality-check:
	cargo test -p cortex-engine --test context_pack
	cargo test -p cortex-engine --test context_verify_quality
	python3 scripts/context_pack_quality_check.py --fixture "$(CONTEXT_PACK_QUALITY_FIXTURE)" --report "$(CONTEXT_PACK_QUALITY_REPORT)"

verification-quality-check:
	cargo test -p cortex-engine --test verification_tests
	cargo test -p cortex-engine --test verification_guards
	cargo test -p cortex-engine --test verification_natural_language
	cargo test -p cortex-engine --test verification_evaluation
	python3 scripts/verification_quality_check.py --fixture "$(VERIFICATION_QUALITY_FIXTURE)" --report "$(VERIFICATION_QUALITY_REPORT)"

security-check:
	cargo test -p cortex-server security_tests
	cargo test -p cortex-server auth_policy_tests
	cargo test -p cortex-server error_taxonomy_tests
	$(MAKE) openapi-contract-check
	python3 scripts/security_beta_check.py --report "$(SECURITY_REPORT)"

rbac-policy-store-check:
	cargo test -p cortex-server auth_policy_tests
	cargo test -p cortex-cli auth_review
	python3 scripts/enterprise_rbac_gate_check.py --gate rbac-policy-store --report "$(RBAC_POLICY_STORE_REPORT)"

quota-policy-check:
	cargo test -p cortex-server security_quota_tests
	cargo test -p cortex-server rate_limit_returns_typed_429_when_enabled
	cargo test -p cortex-cli auth_review_rejects_zero_quota
	python3 scripts/enterprise_rbac_gate_check.py --gate quota-policy --report "$(QUOTA_POLICY_REPORT)"

audit-chain-check:
	cargo test -p cortex-server audit_tests
	cargo test -p cortex-cli audit_command_can_verify_chain
	cargo test -p cortex-cli audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering
	python3 scripts/enterprise_rbac_gate_check.py --gate audit-chain --report "$(AUDIT_CHAIN_REPORT)"

security-hardening-check: security-check rbac-policy-store-check quota-policy-check audit-chain-check
	python3 scripts/security_hardening_check.py --report "$(SECURITY_HARDENING_REPORT)"

compliance-boundary-check:
	python3 scripts/compliance_boundary_check.py --report "$(COMPLIANCE_BOUNDARY_REPORT)"

observability-check:
	python3 scripts/observability_check.py --report "$(OBSERVABILITY_REPORT)"

service-manager-smoke-check:
	python3 scripts/service_manager_smoke_check.py --report "$(SERVICE_MANAGER_REPORT)"

deployment-upgrade-check: service-manager-smoke-check
	python3 scripts/deployment_upgrade_check.py --report "$(DEPLOYMENT_UPGRADE_REPORT)"

operations-runbook-check:
	python3 scripts/operations_runbook_check.py --report "$(OPERATIONS_RUNBOOK_REPORT)"

http-contract-ops-check: security-check
	python3 scripts/http_contract_ops_check.py --report "$(HTTP_CONTRACT_OPS_REPORT)"

cli-product-check:
	cargo test -p cortex-cli help_and_version_commands_work
	cargo test -p cortex-cli doctor_and_completions_commands_work
	cargo test -p cortex-cli cli_golden_outputs_are_stable
	python3 scripts/cli_product_check.py --report "$(CLI_PRODUCT_REPORT)"

future-epic-design-check:
	python3 scripts/future_epic_design_check.py --epic all --report "$(FUTURE_EPIC_REPORT)"

distributed-consensus-design-check:
	python3 scripts/future_epic_design_check.py --epic distributed-consensus --report "$(FUTURE_EPIC_ROOT)/distributed-consensus.json"

managed-cloud-design-check:
	python3 scripts/future_epic_design_check.py --epic managed-cloud --report "$(FUTURE_EPIC_ROOT)/managed-cloud.json"

enterprise-rbac-design-check:
	python3 scripts/future_epic_design_check.py --epic enterprise-rbac --report "$(FUTURE_EPIC_ROOT)/enterprise-rbac.json"

hnsw-no-fallback-design-check:
	python3 scripts/future_epic_design_check.py --epic hnsw-no-fallback --report "$(FUTURE_EPIC_ROOT)/hnsw-no-fallback.json"

llm-inference-design-check:
	python3 scripts/future_epic_design_check.py --epic llm-inference --report "$(FUTURE_EPIC_ROOT)/llm-inference.json"

external-identity-design-check:
	python3 scripts/future_epic_design_check.py --epic external-identity --report "$(FUTURE_EPIC_ROOT)/external-identity.json"

legal-verification-design-check:
	python3 scripts/future_epic_design_check.py --epic legal-verification --report "$(FUTURE_EPIC_ROOT)/legal-verification.json"

distributed-consensus-check:
	cargo test -p cortex-engine --test replication_log
	cargo test -p cortex-engine --test replication_log_matching
	cargo test -p cortex-engine --test replication_commit
	cargo test -p cortex-engine --test replication_election
	cargo test -p cortex-engine --test replication_membership
	cargo test -p cortex-engine --test replication_replay_apply
	python3 scripts/consensus_gate_check.py --gate distributed-consensus --report "$(CONSENSUS_CORE_REPORT)"

consensus-partition-soak-check: replication-partition-check
	python3 scripts/consensus_gate_check.py --gate partition-soak --evidence "$(REPLICATION_PARTITION_REPORT)" --report "$(CONSENSUS_PARTITION_SOAK_REPORT)"

consensus-failover-slo-check: replication-partition-check
	python3 scripts/consensus_gate_check.py --gate failover-slo --evidence "$(REPLICATION_PARTITION_REPORT)" --report "$(CONSENSUS_FAILOVER_SLO_REPORT)"

consensus-rejoin-check: replication-partition-check replication-lifecycle-check
	python3 scripts/consensus_gate_check.py --gate rejoin --evidence "$(REPLICATION_PARTITION_REPORT)" --evidence "$(REPLICATION_LIFECYCLE_REPORT)" --report "$(CONSENSUS_REJOIN_REPORT)"

distributed-consensus-research-check: distributed-consensus-check consensus-partition-soak-check consensus-failover-slo-check consensus-rejoin-check
	python3 scripts/distributed_consensus_research_check.py --report "$(CONSENSUS_RESEARCH_REPORT)"

cloud-tenant-lifecycle-check: tenant-recovery-check observability-check http-contract-ops-check
	python3 scripts/managed_cloud_gate_check.py --gate tenant-lifecycle --evidence tenant_recovery="$(TENANT_RECOVERY_REPORT)" --evidence observability="$(OBSERVABILITY_REPORT)" --evidence http_contract_ops="$(HTTP_CONTRACT_OPS_REPORT)" --report "$(MANAGED_CLOUD_TENANT_REPORT)"

cloud-backup-restore-check: backup-drill-check backup-offsite-check tenant-recovery-check
	python3 scripts/managed_cloud_gate_check.py --gate backup-restore --evidence backup_drill="$(BACKUP_DRILL_REPORT)" --evidence backup_offsite="$(BACKUP_OFFSITE_REPORT)" --evidence tenant_recovery="$(TENANT_RECOVERY_REPORT)" --report "$(MANAGED_CLOUD_BACKUP_REPORT)"

cloud-upgrade-check: deployment-upgrade-check migration-policy-check migration-compatibility-check
	python3 scripts/managed_cloud_gate_check.py --gate upgrade --evidence deployment_upgrade="$(DEPLOYMENT_UPGRADE_REPORT)" --report "$(MANAGED_CLOUD_UPGRADE_REPORT)"

managed-cloud-feasibility-check: cloud-tenant-lifecycle-check cloud-backup-restore-check cloud-upgrade-check
	python3 scripts/managed_cloud_feasibility_check.py --report "$(MANAGED_CLOUD_FEASIBILITY_REPORT)"

ann-production-no-fallback-check: ann-fixture-report ann-external-report ann-metric-matrix-report ann-domain-corpus-report ann-recall-probe-report
	cargo test -p cortex-engine hnsw_no_fallback
	python3 scripts/hnsw_no_fallback_gate_check.py --gate production-no-fallback --evidence fixture="$(ANN_FIXTURE_REPORT)" --evidence external="$(ANN_EXTERNAL_REPORT)" --evidence metric_matrix="$(ANN_METRIC_MATRIX_REPORT)" --evidence domain="$(ANN_DOMAIN_REPORT)" --evidence recall_probe="$(ANN_RECALL_PROBE_REPORT)" --report "$(HNSW_PRODUCTION_NO_FALLBACK_REPORT)"

ann-real-domain-history-check: ann-domain-corpus-report ann-history-fixture-check
	python3 scripts/hnsw_no_fallback_gate_check.py --gate real-domain-history --evidence domain="$(ANN_DOMAIN_REPORT)" --history "$(ANN_HISTORY_CLEAN_FIXTURE)" --report "$(HNSW_REAL_DOMAIN_HISTORY_REPORT)"

ann-public-corpus-history-check: ann-public-corpus-smoke ann-history-fixture-check
	python3 scripts/hnsw_no_fallback_gate_check.py --gate public-corpus-history --history "$(ANN_HISTORY_CLEAN_FIXTURE)" --report "$(HNSW_PUBLIC_CORPUS_HISTORY_REPORT)"

ann-graph-freshness-check:
	cargo test -p cortex-engine hnsw_no_fallback
	cargo test -p cortex-engine --test hnsw_persistence
	cargo test -p cortex-engine --test hnsw_manifest_profile
	cargo test -p cortex-engine --test validation_tests hnsw
	python3 scripts/hnsw_no_fallback_gate_check.py --gate graph-freshness --report "$(HNSW_GRAPH_FRESHNESS_REPORT)"

llm-inference-contract-check: openapi-contract-check
	python3 scripts/llm_inference_gate_check.py --gate contract --report "$(LLM_INFERENCE_CONTRACT_REPORT)"

llm-inference-safety-check:
	python3 scripts/llm_inference_gate_check.py --gate safety --report "$(LLM_INFERENCE_SAFETY_REPORT)"

llm-inference-smoke-check:
	cargo test -p cortex-server llm_inference
	python3 scripts/llm_inference_gate_check.py --gate smoke --report "$(LLM_INFERENCE_SMOKE_REPORT)"

secrets-check:
	python3 scripts/llm_inference_gate_check.py --gate secrets --report "$(LLM_INFERENCE_SECRETS_REPORT)"

oidc-auth-contract-check: openapi-contract-check
	python3 scripts/external_identity_gate_check.py --gate oidc-contract --report "$(OIDC_AUTH_CONTRACT_REPORT)"

identity-policy-mapping-check:
	cargo test -p cortex-server external_identity
	python3 scripts/external_identity_gate_check.py --gate policy-mapping --report "$(IDENTITY_POLICY_MAPPING_REPORT)"

auth-rotation-check:
	cargo test -p cortex-server external_identity
	python3 scripts/external_identity_gate_check.py --gate rotation --report "$(AUTH_ROTATION_REPORT)"

legal-verification-dataset-check:
	cargo test -p cortex-engine legal
	python3 scripts/legal_verification_gate_check.py --gate dataset --report "$(LEGAL_VERIFICATION_DATASET_REPORT)"

legal-verification-quality-check: verification-quality-check
	python3 scripts/legal_verification_gate_check.py --gate quality --evidence "$(VERIFICATION_QUALITY_REPORT)" --report "$(LEGAL_VERIFICATION_QUALITY_REPORT)"

legal-citation-policy-check:
	cargo test -p cortex-engine legal
	python3 scripts/legal_verification_gate_check.py --gate citation-policy --report "$(LEGAL_CITATION_POLICY_REPORT)"

binary-release-package:
	cargo build --release -p cortex-cli --bin cortexdb
	cargo build --release -p cortex-server --bin cortex-server
	python3 scripts/package_binaries.py --package-id "$(BINARY_RELEASE_ID)" --platform "$(BINARY_RELEASE_PLATFORM)" --version "$(BINARY_RELEASE_VERSION)" --archive "$(BINARY_RELEASE_ARCHIVE)"

binary-release-validate:
	python3 scripts/package_binaries.py --validate --archive "$(BINARY_RELEASE_ARCHIVE)"

binary-platform-matrix-check:
	python3 scripts/binary_platform_matrix_check.py --archive "$(BINARY_RELEASE_ARCHIVE)" --report "$(BINARY_PLATFORM_MATRIX_REPORT)"

install-script-check:
	python3 scripts/install_script_check.py

release-artifact-manifest-check:
	python3 scripts/release_artifact_manifest_check.py --version "$(BINARY_RELEASE_VERSION)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)" --manifest "$(RELEASE_ARTIFACT_MANIFEST)" --report "$(RELEASE_ARTIFACT_MANIFEST_REPORT)"

binary-release-check:
	python3 scripts/package_binaries.py --self-test
	$(MAKE) install-script-check
	$(MAKE) binary-release-package
	$(MAKE) binary-release-validate
	$(MAKE) binary-platform-matrix-check

beta-delta-check:
	python3 scripts/check_beta_delta.py

beta-foundation-check:
	python3 scripts/beta_foundation_check.py --root "$(BETA_FOUNDATION_ROOT)" --report "$(BETA_FOUNDATION_REPORT)"

beta-rc-check:
	python3 scripts/beta_rc_check.py --root "$(BETA_RC_ROOT)" --report "$(BETA_RC_REPORT)"

beta-landing-check:
	python3 scripts/beta_landing_check.py --report "$(BETA_LANDING_REPORT)"

beta-release-check: beta-landing-check
	python3 scripts/beta_release_bundle.py --root "$(BETA_RELEASE_ROOT)" --report "$(BETA_RELEASE_REPORT)" --archive "$(BETA_RELEASE_ARCHIVE)"

use-case-pack-check:
	python3 scripts/use_case_pack_check.py --report "$(USE_CASE_PACK_REPORT)"

contributor-onboarding-check:
	python3 scripts/contributor_onboarding_check.py --report "$(CONTRIBUTOR_ONBOARDING_REPORT)"

public-benchmarks-check:
	python3 scripts/public_benchmarks_check.py --report "$(PUBLIC_BENCHMARKS_REPORT)"

comparison-docs-check:
	python3 scripts/comparison_docs_check.py --report "$(COMPARISON_DOCS_REPORT)"

agent-memory-demo-check:
	python3 scripts/agent_memory_demo_check.py --report "$(AGENT_MEMORY_DEMO_REPORT)"

tool-registry-check:
	python3 scripts/tool_registry_check.py --report "$(TOOL_REGISTRY_REPORT)"

knowledge-graph-check:
	python3 scripts/knowledge_graph_check.py --report "$(KNOWLEDGE_GRAPH_REPORT)"

production-hardening-check:
	python3 scripts/production_hardening_check.py --root "$(PRODUCTION_HARDENING_ROOT)" --report "$(PRODUCTION_HARDENING_REPORT)"

production-candidate-check:
	python3 scripts/production_candidate_check.py --root "$(PRODUCTION_CANDIDATE_ROOT)" --report "$(PRODUCTION_CANDIDATE_REPORT)"

production-v1-check:
	python3 scripts/production_v1_check.py --root "$(PRODUCTION_V1_ROOT)" --report "$(PRODUCTION_V1_REPORT)"

public-claims-check:
	python3 scripts/check_public_claims.py --report "$(PUBLIC_CLAIMS_REPORT)"

load-smoke-check:
	cargo build -p cortex-server --bin cortex-server
	python3 scripts/load_smoke_check.py --server ./target/debug/cortex-server --root "$(LOAD_SMOKE_ROOT)" --report "$(LOAD_SMOKE_REPORT)" --cells "$(LOAD_SMOKE_CELLS)" --reads "$(LOAD_SMOKE_READS)" --searches "$(LOAD_SMOKE_SEARCHES)" --contexts "$(LOAD_SMOKE_CONTEXTS)" --verifies "$(LOAD_SMOKE_VERIFIES)" --workers "$(LOAD_SMOKE_WORKERS)"

longmemeval-v1-official-repo:
	@if [ ! -d "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/xiaowu0162/LongMemEval "$(LONGMEMEVAL_V1_OFFICIAL_REPO)"; \
	else \
	  git -C "$(LONGMEMEVAL_V1_OFFICIAL_REPO)" pull --ff-only; \
	fi

longmemeval-v1-official-lite-env: longmemeval-v1-official-repo
	@if [ ! -x "$(LONGMEMEVAL_V1_PYTHON)" ]; then python3 -m venv "$(LONGMEMEVAL_V1_VENV)"; fi
	"$(LONGMEMEVAL_V1_PYTHON)" -m pip install -r "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/requirements-lite.txt"

longmemeval-v1-official-data:
	python3 scripts/longmemeval/download_v1.py --data-root "$(LONGMEMEVAL_V1_DATA_ROOT)" --split "$(LONGMEMEVAL_V1_DATA_SPLIT)" --manifest "$(LONGMEMEVAL_V1_DATA_MANIFEST)"

longmemeval-v1-cortexdb-retrieval: longmemeval-v1-official-data
	cargo build --release -p cortex-cli --bin cortexdb
	@limit_args=""; \
	if [ -n "$(LONGMEMEVAL_V1_LIMIT)" ]; then limit_args="--limit $(LONGMEMEVAL_V1_LIMIT)"; fi; \
	python3 scripts/longmemeval/v1_cortexdb_retrieval.py \
	  --data-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --cortexdb-bin ./target/release/cortexdb \
	  --output-dir "$(LONGMEMEVAL_V1_OUTPUT_ROOT)" \
	  --granularity "$(LONGMEMEVAL_V1_GRANULARITY)" \
	  --index-mode "$(LONGMEMEVAL_V1_INDEX_MODE)" \
	  --context-mode "$(LONGMEMEVAL_V1_CONTEXT_MODE)" \
	  --max-turn-chars "$(LONGMEMEVAL_V1_MAX_TURN_CHARS)" \
	  --max-session-chars "$(LONGMEMEVAL_V1_MAX_SESSION_CHARS)" \
	  --top-k "$(LONGMEMEVAL_V1_TOPK)" \
	  $$limit_args

longmemeval-v1-official-retrieval-metrics: longmemeval-v1-official-lite-env longmemeval-v1-cortexdb-retrieval
	"$(LONGMEMEVAL_V1_PYTHON)" "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/evaluation/print_retrieval_metrics.py" "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" > "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)"
	cat "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)"

longmemeval-v1-official-generate: longmemeval-v1-official-repo longmemeval-v1-cortexdb-retrieval
	@if [ -z "$(LONGMEMEVAL_V1_READER_OPENAI_KEY)" ]; then echo "Set LONGMEMEVAL_V1_READER_OPENAI_KEY or OPENAI_API_KEY for official generation" >&2; exit 2; fi
	mkdir -p "$(LONGMEMEVAL_V1_GENERATION_ROOT)"
	@base_url_args=""; \
	if [ -n "$(LONGMEMEVAL_V1_READER_BASE_URL)" ]; then base_url_args="--openai_base_url $(LONGMEMEVAL_V1_READER_BASE_URL)"; fi; \
	python3 "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/generation/run_generation.py" \
	  --in_file "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --out_dir "$(LONGMEMEVAL_V1_GENERATION_ROOT)" \
	  --model_name "$(LONGMEMEVAL_V1_READER_MODEL_NAME)" \
	  --model_alias "$(LONGMEMEVAL_V1_READER_MODEL_ALIAS)" \
	  $$base_url_args \
	  --openai_key "$(LONGMEMEVAL_V1_READER_OPENAI_KEY)" \
	  --retriever_type flat-session \
	  --topk_context "$(LONGMEMEVAL_V1_GENERATION_TOPK)" \
	  --history_format json \
	  --useronly false \
	  --cot false \
	  --con false

longmemeval-v1-official-qa-score: longmemeval-v1-official-lite-env longmemeval-v1-official-data
	@if [ -z "$(OPENAI_API_KEY)" ]; then echo "Set OPENAI_API_KEY for official LongMemEval evaluator" >&2; exit 2; fi
	@if [ -z "$(LONGMEMEVAL_V1_HYPOTHESIS_FILE)" ]; then echo "Set LONGMEMEVAL_V1_HYPOTHESIS_FILE to the official generation output jsonl" >&2; exit 2; fi
	cd "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/evaluation" && \
	  "$(abspath $(LONGMEMEVAL_V1_PYTHON))" evaluate_qa.py "$(LONGMEMEVAL_V1_EVAL_MODEL)" "$(abspath $(LONGMEMEVAL_V1_HYPOTHESIS_FILE))" "$(abspath $(LONGMEMEVAL_V1_DATA_FILE))"

longmemeval-v1-official-score: longmemeval-v1-official-generate
	@echo "Generation completed. Re-run make longmemeval-v1-official-qa-score LONGMEMEVAL_V1_HYPOTHESIS_FILE=<generated-file>"

longmemeval-v1-package-submission:
	python3 scripts/longmemeval/package_v1_submission.py \
	  --package-name "$(LONGMEMEVAL_V1_PACKAGE_NAME)" \
	  --output-root "$(LONGMEMEVAL_V1_SUBMISSION_ROOT)" \
	  --force

longmemeval-v1-error-analysis:
	python3 scripts/longmemeval/analyze_v1_results.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --official-metrics "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)" \
	  --generation-dir "$(LONGMEMEVAL_V1_GENERATION_ROOT)" \
	  --output-root "$(LONGMEMEVAL_V1_ANALYSIS_ROOT)"

longmemeval-v1-deepseek-flash-falsecase-check:
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/compact_context_false_cases_retrieval.jsonl" \
	  --reference-file "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/false_cases_reference.json" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking "$(LONGMEMEVAL_V1_DEEPSEEK_GENERATION_THINKING)" \
	  --judge-thinking "$(LONGMEMEVAL_V1_DEEPSEEK_JUDGE_THINKING)"

longmemeval-v1-deepseek-flash-diff:
	python3 scripts/longmemeval/compare_deepseek_flash_runs.py \
	  --old-root "$(LONGMEMEVAL_V1_DEEPSEEK_FLASH_IMPLICIT_ROOT)" \
	  --new-root "$(LONGMEMEVAL_V1_DEEPSEEK_ROOT)" \
	  --reference-file "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/false_cases_reference.json" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_FLASH_DIFF_ROOT)"

longmemeval-v1-deepseek-flash-compact-50-check:
	python3 scripts/longmemeval/build_balanced_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --limit "$(LONGMEMEVAL_V1_COMPACT_50_LIMIT)" \
	  --output-root "$(LONGMEMEVAL_V1_COMPACT_50_INPUT_ROOT)" \
	  --output-prefix compact_50
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_50_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_COMPACT_50_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_COMPACT_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-compact-500-check:
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_COMPACT_500_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-preference-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type single-session-preference \
	  --output-root "$(LONGMEMEVAL_V1_PREFERENCE_INPUT_ROOT)" \
	  --output-prefix preference
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_PREFERENCE_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_PREFERENCE_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_PREFERENCE_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-single-session-user-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type single-session-user \
	  --output-root "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_INPUT_ROOT)" \
	  --output-prefix single_session_user
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-multi-session-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type multi-session \
	  --output-root "$(LONGMEMEVAL_V1_MULTI_SESSION_INPUT_ROOT)" \
	  --output-prefix multi_session
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_MULTI_SESSION_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_MULTI_SESSION_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_MULTI_SESSION_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-temporal-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type temporal-reasoning \
	  --output-root "$(LONGMEMEVAL_V1_TEMPORAL_INPUT_ROOT)" \
	  --output-prefix temporal
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_TEMPORAL_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_TEMPORAL_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_TEMPORAL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

multihop-rag-official-repo:
	@if [ ! -d "$(MULTIHOP_RAG_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/yixuantt/MultiHop-RAG "$(MULTIHOP_RAG_OFFICIAL_REPO)"; \
	else \
	  git -C "$(MULTIHOP_RAG_OFFICIAL_REPO)" pull --ff-only; \
	fi

multihop-rag-official-data:
	python3 scripts/multihop_rag/download.py \
	  --data-root "$(MULTIHOP_RAG_DATA_ROOT)" \
	  --manifest "$(MULTIHOP_RAG_DATA_MANIFEST)"

multihop-rag-preflight: multihop-rag-official-data
	python3 scripts/multihop_rag/preflight.py \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --report "$(MULTIHOP_RAG_PREFLIGHT_REPORT)"

multihop-rag-balanced-50: multihop-rag-preflight
	python3 scripts/multihop_rag/build_balanced_subset.py \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --limit "$(MULTIHOP_RAG_SUBSET_LIMIT)" \
	  --output-root "$(MULTIHOP_RAG_SUBSET_ROOT)" \
	  --output-prefix "$(MULTIHOP_RAG_SUBSET_PREFIX)"

multihop-rag-local-50-check: multihop-rag-balanced-50
	@echo "MultiHop-RAG local 50-query subset ready under $(MULTIHOP_RAG_SUBSET_ROOT)/$(MULTIHOP_RAG_SUBSET_PREFIX)"

multihop-rag-cortexdb-retrieval-50: multihop-rag-local-50-check
	cargo build --release -p cortex-engine --bin multihop_rag_retrieval
	./target/release/multihop_rag_retrieval \
	  --queries "$(MULTIHOP_RAG_SUBSET_ROOT)/$(MULTIHOP_RAG_SUBSET_PREFIX)/$(MULTIHOP_RAG_SUBSET_PREFIX)_multihop.json" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --db-root "$(MULTIHOP_RAG_DB_50)" \
	  --output "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --report "$(MULTIHOP_RAG_RETRIEVAL_50_REPORT)" \
	  --top-k "$(MULTIHOP_RAG_TOPK)" \
	  --reset-db

multihop-rag-official-retrieval-metrics-50: multihop-rag-official-repo multihop-rag-cortexdb-retrieval-50
	python3 "$(MULTIHOP_RAG_OFFICIAL_REPO)/retrieval_evaluate.py" --file "$(MULTIHOP_RAG_RETRIEVAL_50)" | tee "$(MULTIHOP_RAG_RETRIEVAL_50_METRICS)"

multihop-rag-cortexdb-retrieval-full: multihop-rag-preflight
	cargo build --release -p cortex-engine --bin multihop_rag_retrieval
	./target/release/multihop_rag_retrieval \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --db-root "$(MULTIHOP_RAG_DB_FULL)" \
	  --output "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --report "$(MULTIHOP_RAG_RETRIEVAL_FULL_REPORT)" \
	  --top-k "$(MULTIHOP_RAG_TOPK)" \
	  --reset-db

multihop-rag-official-retrieval-metrics-full: multihop-rag-official-repo multihop-rag-cortexdb-retrieval-full
	python3 "$(MULTIHOP_RAG_OFFICIAL_REPO)/retrieval_evaluate.py" --file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" | tee "$(MULTIHOP_RAG_RETRIEVAL_FULL_METRICS)"

multihop-rag-retrieval-full-existing-check:
	@test -f "$(MULTIHOP_RAG_RETRIEVAL_FULL)" || (echo "missing $(MULTIHOP_RAG_RETRIEVAL_FULL); run make multihop-rag-official-retrieval-metrics-full first" && exit 1)

multihop-rag-qa-full-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-full first" && exit 1)

multihop-rag-qa-hybrid-full-retry-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-hybrid-full-retry first" && exit 1)

multihop-rag-qa-hybrid-full-retry-v4-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-hybrid-full-retry-v4 first" && exit 1)

multihop-rag-deepseek-qa-50: multihop-rag-official-retrieval-metrics-50
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --output-root "$(MULTIHOP_RAG_QA_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-50-cache-metrics: multihop-rag-official-retrieval-metrics-50
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --output-root "$(MULTIHOP_RAG_QA_50_CACHE_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_50_CACHE_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_50_CACHE_METRICS))"

multihop-rag-official-qa-metrics-50: multihop-rag-official-repo multihop-rag-deepseek-qa-50
	$(MAKE) multihop-rag-official-qa-metrics-existing-50

multihop-rag-official-qa-metrics-existing-50: multihop-rag-official-repo
	test -f "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_50_METRICS))"

multihop-rag-qa-error-analysis-50:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_50_ANALYSIS)" \
	  --output-md "$(MULTIHOP_RAG_QA_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-full: multihop-rag-official-retrieval-metrics-full
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_FULL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-temporal-50-v3: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-official-qa-metrics-temporal-50-v3: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v3
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_METRICS))"

multihop-rag-qa-error-analysis-temporal-50-v3:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-50-v3-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-abstention-retry

multihop-rag-official-qa-metrics-temporal-50-v3-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v3-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_METRICS))"

multihop-rag-qa-error-analysis-temporal-50-v3-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-50-v4-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-decomposition-retry

multihop-rag-official-qa-metrics-temporal-50-v4-decompose-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v4-decompose-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-temporal-50-v4-decompose-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-chronology-50-v1: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --temporal-subtype chronology \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_CHRONOLOGY_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-chronology-retry

multihop-rag-official-qa-metrics-temporal-chronology-50-v1: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-chronology-50-v1
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-temporal-chronology-50-v1:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-comparison-50-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --max-queries "$(MULTIHOP_RAG_COMPARISON_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry

multihop-rag-official-qa-metrics-comparison-50-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-comparison-50-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-comparison-50-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-comparison-50-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --max-queries "$(MULTIHOP_RAG_COMPARISON_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry \
	  --comparison-retry-style decompose

multihop-rag-official-qa-metrics-comparison-50-decompose-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-comparison-50-decompose-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-comparison-50-decompose-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-v3: multihop-rag-official-retrieval-metrics-full
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-temporal-v3-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-abstention-retry

multihop-rag-deepseek-qa-comparison-v2-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry

multihop-rag-deepseek-qa-comparison-v3-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry \
	  --comparison-retry-style decompose

multihop-rag-combine-qa-full-hybrid: multihop-rag-deepseek-qa-full multihop-rag-deepseek-qa-temporal-v3
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_TEMPORAL_ROOT)/deepseek_qa.json" \
	  --question-type temporal_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa_report.json"

multihop-rag-combine-qa-full-hybrid-retry: multihop-rag-qa-full-existing-check multihop-rag-deepseek-qa-temporal-v3-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_TEMPORAL_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type temporal_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa_report.json"

multihop-rag-combine-qa-full-hybrid-retry-v4: multihop-rag-qa-hybrid-full-retry-existing-check multihop-rag-deepseek-qa-comparison-v2-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_COMPARISON_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type comparison_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa_report.json"

multihop-rag-postprocess-hybrid-full-retry-v5: multihop-rag-qa-hybrid-full-retry-v4-existing-check
	python3 scripts/multihop_rag/postprocess_qa_answers.py \
	  --input "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa_report.json" \
	  --temporal-answer-normalize

multihop-rag-combine-qa-full-hybrid-retry-v6: multihop-rag-postprocess-hybrid-full-retry-v5 multihop-rag-deepseek-qa-comparison-v3-decompose-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_COMPARISON_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type comparison_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa_report.json"

multihop-rag-official-qa-metrics-hybrid-full: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v4: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry-v4
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v5: multihop-rag-official-repo multihop-rag-postprocess-hybrid-full-retry-v5
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v6: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry-v6
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_METRICS))"

multihop-rag-official-qa-metrics-full: multihop-rag-official-repo multihop-rag-deepseek-qa-full
	$(MAKE) multihop-rag-official-qa-metrics-existing-full

multihop-rag-official-qa-metrics-existing-full: multihop-rag-official-repo
	test -f "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_FULL_METRICS))"

multihop-rag-qa-error-analysis-full:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_FULL_ANALYSIS)" \
	  --output-md "$(MULTIHOP_RAG_QA_FULL_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v4:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v5:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v6:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/qa_error_analysis.md"

multihop-rag-temporal-subtype-analysis-v6:
	python3 scripts/multihop_rag/analyze_temporal_subtypes.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/temporal_subtype_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/temporal_subtype_analysis.md"

single-node-performance-check:
	cargo run --release -p cortex-engine --bin single_node_performance_check -- --root "$(SINGLE_NODE_PERF_ROOT)" --report "$(SINGLE_NODE_PERF_REPORT)" --cells "$(SINGLE_NODE_PERF_CELLS)" --max-total-ms "$(SINGLE_NODE_PERF_MAX_TOTAL_MS)"

performance-trend-check:
	python3 scripts/performance_trend_check.py --load-report "$(LOAD_SMOKE_REPORT)" --single-node-report "$(SINGLE_NODE_PERF_REPORT)" --history-root "$(PERFORMANCE_HISTORY_ROOT)" --report "$(PERFORMANCE_TREND_REPORT)"

tenant-recovery-check:
	cargo build -p cortex-server --bin cortex-server
	cargo build -p cortex-cli --bin cortexdb
	python3 scripts/tenant_recovery_check.py --server ./target/debug/cortex-server --cli ./target/debug/cortexdb --root "$(TENANT_RECOVERY_ROOT)" --report "$(TENANT_RECOVERY_REPORT)"

context-verify-quality-check:
	cargo test -p cortex-engine --test context_verify_quality

dashboard-build:
	python3 scripts/dashboard_build.py

dashboard-standalone-build: dashboard-build

dashboard-check:
	python3 scripts/dashboard_build.py --check

dashboard-standalone-check: dashboard-check

dashboard-standalone-smoke: dashboard-standalone-check
	python3 scripts/dashboard_dist_smoke.py

dashboard-package: dashboard-standalone-build
	python3 scripts/dashboard_release.py --package-id $(DASHBOARD_PACKAGE_ID) --archive $(DASHBOARD_PACKAGE_ARCHIVE)

dashboard-validate-package:
	python3 scripts/dashboard_release.py --validate --archive $(DASHBOARD_PACKAGE_ARCHIVE)

dashboard-release-check:
	python3 scripts/dashboard_release.py --self-test
	$(MAKE) dashboard-package
	$(MAKE) dashboard-validate-package

dashboard-product-check: dashboard-standalone-smoke
	$(MAKE) dashboard-release-check
	python3 scripts/dashboard_product_check.py --report "$(DASHBOARD_PRODUCT_REPORT)"

dashboard-smoke: dashboard-check
	cargo build -p cortex-server
	npm ci
	@if [ -n "$$CI" ]; then npx playwright install --with-deps chromium; else npx playwright install chromium; fi
	npm run dashboard:smoke

dashboard-screenshots: dashboard-check
	cargo build -p cortex-server
	npm ci
	@if [ -n "$$CI" ]; then npx playwright install --with-deps chromium; else npx playwright install chromium; fi
	npm run dashboard:screenshots

ann-fixture-check:
	cargo run --release -p cortex-engine --bin ann_fixture_gate -- --baseline $(ANN_FIXTURE_BASELINE)

ann-fixture-report:
	cargo run --release -p cortex-engine --bin ann_fixture_gate -- --baseline $(ANN_FIXTURE_BASELINE) --output $(ANN_FIXTURE_REPORT)

ann-drift-check:
	cargo run --release -p cortex-engine --bin ann_drift_check -- --baseline $(ANN_DRIFT_BASELINE)

ann-drift-report:
	cargo run --release -p cortex-engine --bin ann_drift_check -- --baseline $(ANN_DRIFT_BASELINE) --output $(ANN_DRIFT_REPORT)

ann-external-check:
	cargo run --release -p cortex-engine --bin ann_external_fixture_check -- --baseline $(ANN_EXTERNAL_BASELINE)

ann-external-report:
	cargo run --release -p cortex-engine --bin ann_external_fixture_check -- --baseline $(ANN_EXTERNAL_BASELINE) --output $(ANN_EXTERNAL_REPORT)

ann-metric-matrix-check:
	cargo run --release -p cortex-engine --bin ann_metric_matrix_check -- --baseline $(ANN_METRIC_MATRIX_BASELINE)

ann-metric-matrix-report:
	cargo run --release -p cortex-engine --bin ann_metric_matrix_check -- --baseline $(ANN_METRIC_MATRIX_BASELINE) --output $(ANN_METRIC_MATRIX_REPORT)

ann-corpus-smoke-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH)

ann-corpus-smoke-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH) --output $(ANN_CORPUS_REPORT)

ann-domain-corpus-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535

ann-domain-corpus-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535 --output $(ANN_DOMAIN_REPORT)

ann-recall-probe-check:
	cargo build --release -p cortex-engine --bin ann_corpus_check
	python3 scripts/ann/recall_probe.py --runner ./target/release/ann_corpus_check --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --iterations $(ANN_RECALL_PROBE_ITERATIONS)

ann-recall-probe-report:
	cargo build --release -p cortex-engine --bin ann_corpus_check
	python3 scripts/ann/recall_probe.py --runner ./target/release/ann_corpus_check --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --iterations $(ANN_RECALL_PROBE_ITERATIONS) --output $(ANN_RECALL_PROBE_REPORT)

ann-production-slo-history-check:
	rm -rf "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)"
	cargo build --release -p cortex-engine --bin ann_corpus_check
	@i=1; while [ $$i -le "$(ANN_PRODUCTION_SLO_HISTORY_RUNS)" ]; do \
	  run_id=$$(printf "slo-%02d" $$i); \
	  scripts/ann/run_external_corpus.sh \
	    --vectors "$(ANN_DOMAIN_VECTORS)" \
	    --queries "$(ANN_DOMAIN_QUERIES)" \
	    --ground-truth "$(ANN_DOMAIN_GROUND_TRUTH)" \
	    --metric dot_product \
	    --output-root "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)" \
	    --run-id "$$run_id" \
	    --min-recall-q16 65535 \
	    --min-mean-recall-q16 65535; \
	  i=$$((i + 1)); \
	done
	python3 scripts/ann/history_gate.py \
	  --run-root "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)" \
	  --output "$(ANN_PRODUCTION_SLO_HISTORY_REPORT)" \
	  --fail-on-regression \
	  --min-runs "$(ANN_PRODUCTION_SLO_HISTORY_RUNS)" \
	  --min-corpora 1 \
	  --max-p95-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_P95_TOLERANCE_NANOS)" \
	  --max-p99-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_P99_TOLERANCE_NANOS)" \
	  --max-max-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_MAX_TOLERANCE_NANOS)"

ann-demo-domain-corpus-build:
	python3 scripts/ann/build_demo_domain_corpus.py --source-root $(ANN_DEMO_DOMAIN_SOURCE_ROOT) --source-root $(ANN_DEMO_DOMAIN_EXTRA_SOURCE_ROOT) --output-dir $(ANN_DEMO_DOMAIN_OUTPUT_DIR) --dimension $(ANN_DEMO_DOMAIN_DIMENSION) --limit $(ANN_DEMO_DOMAIN_LIMIT) --scale $(ANN_DEMO_DOMAIN_SCALE)

ann-demo-domain-corpus-run: ann-demo-domain-corpus-build
	scripts/ann/run_external_corpus.sh --vectors $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric dot_product --output-root $(ANN_DEMO_DOMAIN_RUN_ROOT) --run-id $(ANN_DEMO_DOMAIN_RUN_ID) --min-recall-q16 65535 --min-mean-recall-q16 65535 --max-neighbors $(ANN_DEMO_DOMAIN_MAX_NEIGHBORS) --ef-search $(ANN_DEMO_DOMAIN_EF_SEARCH) --ef-construction $(ANN_DEMO_DOMAIN_EF_CONSTRUCTION) --layer-count $(ANN_DEMO_DOMAIN_LAYER_COUNT)

ann-demo-domain-publish-baseline: ann-demo-domain-corpus-run
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_DEMO_DOMAIN_RUN_ROOT) --run-id $(ANN_DEMO_DOMAIN_RUN_ID) --baseline-id $(ANN_DEMO_DOMAIN_BASELINE_ID) --output-root $(ANN_DEMO_DOMAIN_BASELINE_ROOT)

ann-demo-domain-package-baseline: ann-demo-domain-publish-baseline
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_DEMO_DOMAIN_BASELINE_BUNDLE) --package-id $(ANN_DEMO_DOMAIN_BASELINE_ID) --output $(ANN_DEMO_DOMAIN_BASELINE_ARCHIVE)

ann-demo-domain-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_DEMO_DOMAIN_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-embedded-domain-corpus-build:
	@if [ -z "$(ANN_EMBEDDED_DOMAIN_SOURCE_ROOT)" ]; then echo "Set ANN_EMBEDDED_DOMAIN_SOURCE_ROOT to a JSONL payload directory with embedded vectors" >&2; exit 2; fi
	@if [ -z "$(ANN_EMBEDDED_DOMAIN_QUERIES)" ]; then echo "Set ANN_EMBEDDED_DOMAIN_QUERIES to a JSONL query file with embedded vectors" >&2; exit 2; fi
	python3 scripts/ann/build_embedded_domain_corpus.py --source-root $(ANN_EMBEDDED_DOMAIN_SOURCE_ROOT) --queries $(ANN_EMBEDDED_DOMAIN_QUERIES) --output-dir $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR) --limit $(ANN_EMBEDDED_DOMAIN_LIMIT)

ann-embedded-domain-corpus-run: ann-embedded-domain-corpus-build
	@slo_args="$$(python3 scripts/ann/slo_profile.py --profile $(ANN_EMBEDDED_DOMAIN_SLO_PROFILE) --format run-external-args)"; \
	custom_args=""; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS)" ]; then custom_args="$$custom_args --max-neighbors $(ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_EF_SEARCH)" ]; then custom_args="$$custom_args --ef-search $(ANN_EMBEDDED_DOMAIN_EF_SEARCH)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION)" ]; then custom_args="$$custom_args --ef-construction $(ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)" ]; then custom_args="$$custom_args --layer-count $(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)"; fi; \
	scripts/ann/run_external_corpus.sh --vectors $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric $(ANN_EMBEDDED_DOMAIN_METRIC) --output-root $(ANN_EMBEDDED_DOMAIN_RUN_ROOT) --run-id $(ANN_EMBEDDED_DOMAIN_RUN_ID) $$slo_args $$custom_args

ann-embedding-domain-export:
	@if [ -z "$(ANN_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_EMBEDDING_SOURCE_ROOT to a JSONL payload directory without vectors" >&2; exit 2; fi
	@if [ -z "$(ANN_EMBEDDING_QUERIES)" ]; then echo "Set ANN_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@if [ "$(ANN_EMBEDDING_PROVIDER)" = "command" ] && [ -z "$(ANN_EMBEDDING_COMMAND)" ]; then echo "Set ANN_EMBEDDING_COMMAND to a command that reads text on stdin and prints a JSON vector" >&2; exit 2; fi
	python3 scripts/ann/export_embedding_domain_corpus.py --source-root $(ANN_EMBEDDING_SOURCE_ROOT) --queries $(ANN_EMBEDDING_QUERIES) --output-dir $(ANN_EMBEDDING_OUTPUT_DIR) --provider $(ANN_EMBEDDING_PROVIDER) --embedding-command "$(ANN_EMBEDDING_COMMAND)" --normalization $(ANN_EMBEDDING_NORMALIZATION) --scale $(ANN_EMBEDDING_SCALE) --limit $(ANN_EMBEDDING_LIMIT)

ann-embedding-domain-corpus-run: ann-embedding-domain-export
	$(MAKE) ann-embedded-domain-corpus-run ANN_EMBEDDED_DOMAIN_SOURCE_ROOT=$(ANN_EMBEDDING_OUTPUT_DIR)/payloads ANN_EMBEDDED_DOMAIN_QUERIES=$(ANN_EMBEDDING_OUTPUT_DIR)/queries.jsonl ANN_EMBEDDED_DOMAIN_OUTPUT_DIR=$(ANN_EMBEDDING_OUTPUT_DIR)/converted ANN_EMBEDDED_DOMAIN_RUN_ROOT=$(ANN_EMBEDDING_RUN_ROOT) ANN_EMBEDDED_DOMAIN_RUN_ID=$(ANN_EMBEDDING_RUN_ID) ANN_EMBEDDED_DOMAIN_METRIC=$(ANN_EMBEDDING_METRIC) ANN_EMBEDDED_DOMAIN_LIMIT=$(ANN_EMBEDDING_LIMIT) ANN_EMBEDDED_DOMAIN_SLO_PROFILE=$(ANN_EMBEDDING_SLO_PROFILE) ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS=$(ANN_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDED_DOMAIN_EF_SEARCH=$(ANN_EMBEDDING_EF_SEARCH) ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION=$(ANN_EMBEDDING_EF_CONSTRUCTION) ANN_EMBEDDED_DOMAIN_LAYER_COUNT=$(ANN_EMBEDDING_LAYER_COUNT)

ann-real-embedding-readiness:
	@source_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ROOT)" ]; then source_args="$$source_args --source-root $(ANN_REAL_EMBEDDING_SOURCE_ROOT)"; fi; \
	query_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_QUERIES)" ]; then query_args="--queries $(ANN_REAL_EMBEDDING_QUERIES)"; fi; \
	required_env_args="--require-env CORTEXDB_EMBEDDING_URL --require-env CORTEXDB_EMBEDDING_MODEL"; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_API_KEY)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_API_KEY"; fi; \
	archive_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)" ]; then archive_args="$$archive_args --source-archive-manifest $(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_SOURCE_ARCHIVE)" = "true" ]; then archive_args="$$archive_args --require-source-archive"; fi; \
	python3 scripts/ann/real_embedding_readiness.py $$source_args $$query_args --embedding-command "$(ANN_REAL_EMBEDDING_COMMAND)" --metric $(ANN_REAL_EMBEDDING_METRIC) --normalization $(ANN_REAL_EMBEDDING_NORMALIZATION) --scale $(ANN_REAL_EMBEDDING_SCALE) --limit $(ANN_REAL_EMBEDDING_LIMIT) $$required_env_args $$archive_args --output $(ANN_REAL_EMBEDDING_READINESS_REPORT)

ann-real-embedding-preflight:
	@if [ -z "$(ANN_REAL_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_REAL_EMBEDDING_SOURCE_ROOT to a JSONL payload directory" >&2; exit 2; fi
	@if [ -z "$(ANN_REAL_EMBEDDING_QUERIES)" ]; then echo "Set ANN_REAL_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@required_env_args="--require-env CORTEXDB_EMBEDDING_URL --require-env CORTEXDB_EMBEDDING_MODEL"; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_API_KEY)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_API_KEY"; fi; \
	python3 scripts/ann/preflight_real_embedding_benchmark.py --source-root $(ANN_REAL_EMBEDDING_SOURCE_ROOT) --queries $(ANN_REAL_EMBEDDING_QUERIES) --embedding-command "$(ANN_REAL_EMBEDDING_COMMAND)" --metric $(ANN_REAL_EMBEDDING_METRIC) --normalization $(ANN_REAL_EMBEDDING_NORMALIZATION) --scale $(ANN_REAL_EMBEDDING_SCALE) --limit $(ANN_REAL_EMBEDDING_LIMIT) $$required_env_args --output $(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)

ann-real-embedding-benchmark: ann-real-embedding-preflight
	$(MAKE) ann-embedding-domain-corpus-run ANN_EMBEDDING_SOURCE_ROOT=$(ANN_REAL_EMBEDDING_SOURCE_ROOT) ANN_EMBEDDING_QUERIES=$(ANN_REAL_EMBEDDING_QUERIES) ANN_EMBEDDING_OUTPUT_DIR=$(ANN_REAL_EMBEDDING_OUTPUT_DIR) ANN_EMBEDDING_RUN_ROOT=$(ANN_REAL_EMBEDDING_RUN_ROOT) ANN_EMBEDDING_RUN_ID=$(ANN_REAL_EMBEDDING_RUN_ID) ANN_EMBEDDING_PROVIDER=command ANN_EMBEDDING_COMMAND="$(ANN_REAL_EMBEDDING_COMMAND)" ANN_EMBEDDING_NORMALIZATION=$(ANN_REAL_EMBEDDING_NORMALIZATION) ANN_EMBEDDING_SCALE=$(ANN_REAL_EMBEDDING_SCALE) ANN_EMBEDDING_METRIC=$(ANN_REAL_EMBEDDING_METRIC) ANN_EMBEDDING_LIMIT=$(ANN_REAL_EMBEDDING_LIMIT) ANN_EMBEDDING_SLO_PROFILE=$(ANN_REAL_EMBEDDING_SLO_PROFILE) ANN_EMBEDDING_MAX_NEIGHBORS=$(ANN_REAL_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDING_EF_SEARCH=$(ANN_REAL_EMBEDDING_EF_SEARCH) ANN_EMBEDDING_EF_CONSTRUCTION=$(ANN_REAL_EMBEDDING_EF_CONSTRUCTION) ANN_EMBEDDING_LAYER_COUNT=$(ANN_REAL_EMBEDDING_LAYER_COUNT)
	@if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)" ]; then \
	  python3 scripts/ann/attach_real_embedding_metadata.py \
	    --run-dir "$(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)" \
	    --preflight "$(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)" \
	    --export-manifest "$(ANN_REAL_EMBEDDING_OUTPUT_DIR)/manifest.json" \
	    --source-archive-manifest "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)"; \
	else \
	  python3 scripts/ann/attach_real_embedding_metadata.py \
	    --run-dir "$(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)" \
	    --preflight "$(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)" \
	    --export-manifest "$(ANN_REAL_EMBEDDING_OUTPUT_DIR)/manifest.json"; \
	fi

ann-real-embedding-compare:
	@if [ -z "$(ANN_REAL_EMBEDDING_BASELINE_REPORT)" ]; then echo "Set ANN_REAL_EMBEDDING_BASELINE_REPORT to a real embedding baseline report.json" >&2; exit 2; fi
	python3 scripts/ann/compare_reports.py --baseline $(ANN_REAL_EMBEDDING_BASELINE_REPORT) --candidate $(ANN_REAL_EMBEDDING_CANDIDATE_REPORT) --output $(ANN_REAL_EMBEDDING_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-benchmark-and-compare: ann-real-embedding-benchmark ann-real-embedding-compare

ann-real-embedding-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT) --max-p95-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-history-regression-check:
	python3 scripts/ann/history_gate.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT) --fail-on-regression --min-runs $(ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS) --min-corpora 1 --max-p95-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --run-id $(ANN_REAL_EMBEDDING_RUN_ID) --baseline-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output-root $(ANN_REAL_EMBEDDING_BASELINE_ROOT)

ann-real-embedding-package-baseline: ann-real-embedding-publish-baseline
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_REAL_EMBEDDING_BASELINE_BUNDLE) --package-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE)

ann-real-embedding-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth --require-real-embedding-metadata

ann-real-embedding-release-check: ann-real-embedding-benchmark
	@if [ -n "$(ANN_REAL_EMBEDDING_BASELINE_REPORT)" ]; then \
	  $(MAKE) ann-real-embedding-compare; \
	else \
	  echo "Skipping real embedding baseline comparison: ANN_REAL_EMBEDDING_BASELINE_REPORT is not set"; \
	fi
	$(MAKE) ann-real-embedding-history-regression-check
	$(MAKE) ann-real-embedding-package-baseline
	$(MAKE) ann-real-embedding-validate-baseline-package

ann-slo-profile:
	python3 scripts/ann/slo_profile.py --profile $(ANN_REAL_EMBEDDING_SLO_PROFILE) --format json

ann-scripts-check:
	python3 scripts/ann/build_demo_domain_corpus.py --self-test
	python3 scripts/ann/build_embedded_domain_corpus.py --self-test
	python3 scripts/ann/export_embedding_domain_corpus.py --self-test
	python3 scripts/ann/embed_text_command.py --self-test
	python3 scripts/ann/preflight_real_embedding_benchmark.py --self-test
	python3 scripts/ann/attach_real_embedding_metadata.py --self-test
	python3 scripts/ann/real_embedding_readiness.py --self-test
	python3 scripts/ann/slo_profile.py --self-test
	python3 scripts/ann/convert_public_corpus.py --self-test
	python3 scripts/ann/run_public_corpus.py --self-test
	python3 scripts/ann/exact_ground_truth.py --self-test
	python3 scripts/ann/recall_probe.py --self-test
	python3 scripts/ann/report_contract.py --self-test
	python3 scripts/ann/compare_reports.py --self-test
	python3 scripts/ann/summarize_history.py --self-test
	python3 scripts/ann/history_contract.py --self-test
	python3 scripts/ann/history_fixture_check.py --self-test
	python3 scripts/ann/history_gate.py --self-test
	python3 scripts/ann/publish_baseline.py --self-test
	python3 scripts/ann/package_baseline.py --self-test
	python3 scripts/ann/validate_baseline_package.py --self-test
	$(MAKE) ann-history-fixture-check
	mkdir -p target/ann
	python3 scripts/ann/exact_ground_truth.py --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output $(ANN_CORPUS_GENERATED_GROUND_TRUTH)
	diff -u $(ANN_CORPUS_GROUND_TRUTH) $(ANN_CORPUS_GENERATED_GROUND_TRUTH)

ann-convert-public-smoke:
	python3 scripts/ann/convert_public_corpus.py --self-test

ann-public-corpus-smoke:
	python3 scripts/ann/run_public_corpus.py --self-test

ann-public-corpus-run:
	@if [ -z "$(ANN_PUBLIC_SOURCE)" ]; then echo "Set ANN_PUBLIC_SOURCE to a URL, archive path, or extracted corpus directory" >&2; exit 2; fi
	@if [ -d "$(ANN_PUBLIC_SOURCE)" ]; then source_arg="--source-dir"; elif [ -f "$(ANN_PUBLIC_SOURCE)" ]; then source_arg="--source-archive"; else source_arg="--source-url"; fi; \
	python3 scripts/ann/run_public_corpus.py \
	  "$$source_arg" "$(ANN_PUBLIC_SOURCE)" \
	  --dataset-id "$(ANN_PUBLIC_DATASET_ID)" \
	  --format "$(ANN_PUBLIC_FORMAT)" \
	  --metric "$(ANN_PUBLIC_METRIC)" \
	  --normalization "$(ANN_PUBLIC_NORMALIZATION)" \
	  --scale "$(ANN_PUBLIC_SCALE)" \
	  --limit "$(ANN_PUBLIC_LIMIT)" \
	  --max-neighbors "$(ANN_PUBLIC_MAX_NEIGHBORS)" \
	  --ef-search "$(ANN_PUBLIC_EF_SEARCH)" \
	  --layer-count "$(ANN_PUBLIC_LAYER_COUNT)" \
	  --max-p99-latency-nanos "$(ANN_PUBLIC_MAX_P99_LATENCY_NANOS)" \
	  --output-root "$(ANN_PUBLIC_OUTPUT_ROOT)" \
	  --run-root "$(ANN_CORPUS_RUN_ROOT)" \
	  --run-id "$(ANN_PUBLIC_RUN_ID)"

ann-corpus-compare:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_REPORT) --candidate $(ANN_CANDIDATE_REPORT) --output $(ANN_REPORT_COMPARISON)

ann-corpus-run-smoke:
	scripts/ann/run_external_corpus.sh --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output-root $(ANN_CORPUS_RUN_ROOT) --run-id $(ANN_CORPUS_RUN_ID)

ann-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-history-regression-check:
	python3 scripts/ann/history_gate.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT) --fail-on-regression --min-runs 1 --min-corpora 1 --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-history-fixture-check:
	python3 scripts/ann/history_fixture_check.py \
	  --clean $(ANN_HISTORY_CLEAN_FIXTURE) \
	  --recall-regression $(ANN_HISTORY_RECALL_REGRESSION_FIXTURE) \
	  --latency-regression $(ANN_HISTORY_LATENCY_REGRESSION_FIXTURE)

ann-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_HISTORY_ROOT) --run-id $(ANN_BASELINE_RUN_ID) --baseline-id $(ANN_BASELINE_ID) --output-root $(ANN_BASELINE_ROOT)

ann-package-baseline:
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_BASELINE_BUNDLE) --package-id $(ANN_BASELINE_ID) --output $(ANN_BASELINE_ARCHIVE)

ann-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-compare-baseline-bundle:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_BUNDLE_REPORT) --candidate $(ANN_HISTORY_ROOT)/$(ANN_CANDIDATE_RUN_ID)/report.json --output $(ANN_BASELINE_BUNDLE_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-release-evidence-check:
	rm -rf $(ANN_RELEASE_EVIDENCE_ROOT)
	$(MAKE) ann-corpus-run-smoke ANN_CORPUS_RUN_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_CORPUS_RUN_ID=$(ANN_RELEASE_EVIDENCE_RUN_ID)
	$(MAKE) ann-history-regression-check ANN_HISTORY_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_HISTORY_REPORT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT)/history.json
	$(MAKE) ann-publish-baseline ANN_HISTORY_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_BASELINE_RUN_ID=$(ANN_RELEASE_EVIDENCE_RUN_ID) ANN_BASELINE_ID=$(ANN_RELEASE_EVIDENCE_BASELINE_ID) ANN_BASELINE_ROOT=$(ANN_RELEASE_EVIDENCE_BASELINE_ROOT)
	$(MAKE) ann-package-baseline ANN_BASELINE_BUNDLE=$(ANN_RELEASE_EVIDENCE_BASELINE_BUNDLE) ANN_BASELINE_ID=$(ANN_RELEASE_EVIDENCE_BASELINE_ID) ANN_BASELINE_ARCHIVE=$(ANN_RELEASE_EVIDENCE_BASELINE_ARCHIVE)
	$(MAKE) ann-validate-baseline-package ANN_BASELINE_ARCHIVE=$(ANN_RELEASE_EVIDENCE_BASELINE_ARCHIVE)
	$(MAKE) ann-demo-domain-package-baseline
	$(MAKE) ann-demo-domain-validate-baseline-package
	@echo "=== ANN release evidence check passed ==="

backup-drill-check:
	scripts/backup_drill_check.sh "$(BACKUP_DRILL_ROOT)" "$(BACKUP_DRILL_REPORT)" "$(BACKUP_DRILL_KEEP_LATEST)" "$(BACKUP_DRILL_PREFIX)"

backup-offsite-check:
	scripts/backup_offsite_check.sh "$(BACKUP_OFFSITE_ROOT)" "$(BACKUP_OFFSITE_REPORT)" "$(BACKUP_OFFSITE_ID)"

encrypted-backup-check:
	cargo test -p cortex-engine encrypted_backup
	cargo test -p cortex-cli backup_encrypted_and_restore_encrypted_commands_roundtrip_database

backup-restore-production-pack-check:
	$(MAKE) backup-drill-check
	$(MAKE) backup-offsite-check
	$(MAKE) encrypted-backup-check
	python3 scripts/backup_restore_production_pack.py --backup-drill-report "$(BACKUP_DRILL_REPORT)" --backup-offsite-report "$(BACKUP_OFFSITE_REPORT)" --output "$(BACKUP_RESTORE_PACK_REPORT)"

crash-fault-check:
	scripts/crash_fault_check.sh "$(CRASH_FAULT_ROOT)" "$(CRASH_FAULT_REPORT)"

chaos-restart-check:
	python3 scripts/chaos_restart_check.py --root "$(CHAOS_RESTART_ROOT)" --report "$(CHAOS_RESTART_REPORT)" --seed "$(CHAOS_RESTART_SEED)" --steps "$(CHAOS_RESTART_STEPS)"

storage-soak-check:
	python3 scripts/storage_soak_check.py --root "$(STORAGE_SOAK_ROOT)" --report "$(STORAGE_SOAK_REPORT)" --cycles "$(STORAGE_SOAK_CYCLES)" --cells-per-cycle "$(STORAGE_SOAK_CELLS_PER_CYCLE)" --kill-delay-ms "$(STORAGE_SOAK_KILL_DELAY_MS)"

storage-soak-history-check: storage-soak-check
	python3 scripts/storage_soak_history_check.py --soak-report "$(STORAGE_SOAK_REPORT)" --history-jsonl "$(STORAGE_SOAK_HISTORY_FILE)" --output "$(STORAGE_SOAK_HISTORY_REPORT)" --min-runs "$(STORAGE_SOAK_HISTORY_MIN_RUNS)" --min-duration-hours "$(STORAGE_SOAK_HISTORY_MIN_HOURS)"

storage-soak-24h-campaign:
	python3 scripts/storage_soak_campaign.py --target-hours "$(STORAGE_SOAK_CAMPAIGN_TARGET_HOURS)" --max-runs "$(STORAGE_SOAK_CAMPAIGN_MAX_RUNS)" --cycles "$(STORAGE_SOAK_CAMPAIGN_CYCLES)" --cells-per-cycle "$(STORAGE_SOAK_CAMPAIGN_CELLS_PER_CYCLE)" --kill-delay-ms "$(STORAGE_SOAK_KILL_DELAY_MS)" --soak-root "$(STORAGE_SOAK_ROOT)" --soak-report "$(STORAGE_SOAK_REPORT)" --history-jsonl "$(STORAGE_SOAK_HISTORY_FILE)" --history-report "$(STORAGE_SOAK_HISTORY_REPORT)" --campaign-report "$(STORAGE_SOAK_CAMPAIGN_REPORT)"

storage-soak-campaign-status:
	python3 scripts/storage_soak_campaign_status.py --format "$(STORAGE_SOAK_CAMPAIGN_STATUS_FORMAT)"

storage-soak-campaign-watchdog:
	python3 scripts/storage_soak_campaign_status.py --require-active --max-stale-minutes "30"

storage-soak-epic-finalize:
	python3 scripts/storage_soak_epic_finalize.py

next-60-epics-audit:
	python3 scripts/next_60_epics_audit.py

next-60-epics-completion-check:
	python3 scripts/next_60_epics_audit.py --require-complete --output "target/next-60-epics-audit/completion_report.json"

replication-partition-check:
	python3 scripts/replication_partition_check.py --root "$(REPLICATION_PARTITION_ROOT)" --report "$(REPLICATION_PARTITION_REPORT)"

replication-lifecycle-check:
	python3 scripts/replication_lifecycle_check.py --root "$(REPLICATION_LIFECYCLE_ROOT)" --report "$(REPLICATION_LIFECYCLE_REPORT)"

production-evidence-sweep:
	scripts/production_evidence_sweep.sh "$(PRODUCTION_EVIDENCE_ROOT)" "$(PRODUCTION_EVIDENCE_REPORT)"

smoke-test:
	scripts/smoke_test.sh

sdk-smoke-test:
	python3 scripts/sdk_smoke_test.py

rag-demo-smoke:
	python3 examples/rag_demo/smoke.py

alpha-check:
	RUSTFLAGS="-D warnings" cargo check --workspace
	RUSTFLAGS="-D warnings" cargo test --workspace --all-features
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	$(MAKE) sdk-check
	$(MAKE) openapi-check
	$(MAKE) openapi-contract-check
	$(MAKE) sdk-contract-check
	$(MAKE) migration-policy-check
	$(MAKE) migration-compatibility-check
	$(MAKE) beta-delta-check
	$(MAKE) public-claims-check
	$(MAKE) load-smoke-check
	$(MAKE) single-node-performance-check
	$(MAKE) performance-trend-check
	$(MAKE) tenant-recovery-check
	$(MAKE) context-verify-quality-check
	$(MAKE) dashboard-check
	$(MAKE) dashboard-smoke
	$(MAKE) dashboard-screenshots
	$(MAKE) dashboard-release-check
	$(MAKE) ann-fixture-check
	$(MAKE) ann-drift-check
	$(MAKE) ann-external-check
	$(MAKE) ann-metric-matrix-check
	$(MAKE) ann-corpus-smoke-check
	$(MAKE) ann-domain-corpus-check
	$(MAKE) ann-scripts-check
	$(MAKE) ann-corpus-run-smoke
	$(MAKE) ann-demo-domain-corpus-run
	cargo bench -p cortex-engine --bench core_baseline
	./examples/demo/investment_projects/run.sh
	$(MAKE) rag-demo-smoke

release-check: alpha-check
	$(MAKE) binary-release-check
	$(MAKE) production-evidence-sweep
	$(MAKE) backup-offsite-check
	$(MAKE) crash-fault-check
	$(MAKE) chaos-restart-check
	$(MAKE) replication-lifecycle-check
	$(MAKE) smoke-test
	$(MAKE) sdk-smoke-test
	@echo "=== Release check passed ==="

demo:
	./examples/demo/investment_projects/run.sh
