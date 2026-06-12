.PHONY: memtable-clone-gate-check descriptor-hot-path-gate-check

check: memtable-clone-gate-check descriptor-hot-path-gate-check
	cargo check --workspace

file-size-report:
	python3 scripts/file_size_report.py --output "$(FILE_SIZE_REPORT)"

file-size-check:
	python3 scripts/file_size_report.py --baseline "$(FILE_SIZE_BASELINE)" --check --output "$(FILE_SIZE_REPORT)"

memtable-clone-gate-check:
	python3 scripts/memtable_clone_gate_check.py

descriptor-hot-path-gate-check:
	python3 scripts/descriptor_hot_path_gate_check.py

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

storage-format-freeze-check:
	cargo test -p cortex-storage --test format_tests
	cargo test -p cortex-storage --test wal_tests
	cargo test -p cortex-storage --test segment_index_tests
	cargo test -p cortex-storage --test lexical_index_tests
	cargo test -p cortex-storage --test vector_index_tests
	cargo test -p cortex-storage --test hnsw_graph_tests
	cargo test -p cortex-storage --test manifest_profile_tests
	python3 scripts/storage_format_freeze_check.py --report "$(STORAGE_FORMAT_FREEZE_REPORT)"
	$(MAKE) storage-format-change-note-check

storage-format-change-note-check:
	python3 scripts/check_storage_format_change_notes.py --report "$(STORAGE_FORMAT_CHANGE_NOTE_REPORT)"

storage-compat-check:
	python3 scripts/storage_compat_check.py --root "$(STORAGE_COMPAT_ROOT)" --report "$(STORAGE_COMPAT_REPORT)"

engine-public-api-freeze-check:
	python3 scripts/engine_api_check.py --root "$(ENGINE_API_ROOT)" --report "$(ENGINE_API_REPORT)"

engine-api-compat-check:
	python3 scripts/engine_api_compat_check.py --root "$(ENGINE_API_COMPAT_ROOT)" --report "$(ENGINE_API_COMPAT_REPORT)"

engine-error-model-check:
	python3 scripts/engine_error_model_check.py --report "$(ENGINE_ERROR_MODEL_REPORT)"

engine-feature-flags-check:
	python3 scripts/engine_feature_flags_check.py --report "$(ENGINE_FEATURE_FLAGS_REPORT)"

module-ownership-check:
	python3 scripts/module_ownership_check.py --report "$(MODULE_OWNERSHIP_REPORT)"

engine-internal-boundary-check:
	python3 scripts/engine_internal_boundary_check.py --report "$(ENGINE_INTERNAL_BOUNDARY_REPORT)"

engine-determinism-check:
	python3 scripts/engine_determinism_check.py --report "$(ENGINE_DETERMINISM_REPORT)"

engine-panic-audit-check:
	python3 scripts/engine_panic_audit_check.py --report "target/engine-panic-audit/report.json"

engine-api-check: engine-public-api-freeze-check engine-api-compat-check engine-error-model-check engine-feature-flags-check module-ownership-check engine-internal-boundary-check engine-determinism-check engine-panic-audit-check

.PHONY: aql-changelog-policy-check
aql-changelog-policy-check:
	python3 scripts/check_aql_changelog_policy.py --report "target/aql-changelog-policy/report.json"

aql-compat-check:
	python3 scripts/aql_compat_check.py --root "$(AQL_COMPAT_ROOT)" --report "$(AQL_COMPAT_REPORT)"

retrieval-quality-history-check:
	python3 scripts/retrieval_quality_history_self_test.py
	python3 scripts/retrieval_quality_history.py --domain-root examples/real_domains --output "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --min-domains 4 --history-runs $(RETRIEVAL_QUALITY_HISTORY_RUNS) --fail-on-regression --max-p95-regression-nanos $(RETRIEVAL_QUALITY_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(RETRIEVAL_QUALITY_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(RETRIEVAL_QUALITY_MAX_MAX_REGRESSION_NANOS)

search-quality-gate-v2-check:
	python3 scripts/search_quality_gate_v2.py --self-test
	python3 scripts/search_quality_gate_v2.py --thresholds "$(SEARCH_QUALITY_GATE_V2_THRESHOLDS)" --beta-report "$(RETRIEVAL_BETA_REPORT)" --history-report "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --ann-history "$(ANN_REAL_EMBEDDING_HISTORY_REPORT)" --output "$(SEARCH_QUALITY_GATE_V2_REPORT)"

retrieval-quality-check:
	cd examples/real_domains/investment_projects && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/support_tickets && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/legal_policies && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	cd examples/real_domains/technical_docs && python3 scripts/validate_corpus.py && python3 scripts/validate_ground_truth.py
	$(MAKE) ann-real-embedding-history-regression-check
	python3 scripts/retrieval_quality_check.py --source-root "$(RETRIEVAL_QUALITY_SOURCE_ROOT)" --queries "$(RETRIEVAL_QUALITY_QUERIES)" --ground-truth "$(RETRIEVAL_QUALITY_GROUND_TRUTH)" --history "$(ANN_REAL_EMBEDDING_HISTORY_REPORT)" --benchmarks docs/BENCHMARKS.md --output "$(RETRIEVAL_QUALITY_REPORT)" --min-docs $(RETRIEVAL_QUALITY_MIN_DOCS) --min-chunks $(RETRIEVAL_QUALITY_MIN_CHUNKS) --min-queries $(RETRIEVAL_QUALITY_MIN_QUERIES) --min-history-runs $(ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS)
	python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output "$(RETRIEVAL_BETA_REPORT)" --min-domains 4 --repeat-runs 5
	$(MAKE) retrieval-quality-history-check
	python3 scripts/retrieval_quality_dashboard_self_test.py
	python3 scripts/retrieval_quality_dashboard.py --report "$(RETRIEVAL_QUALITY_REPORT)" --beta-report "$(RETRIEVAL_BETA_REPORT)" --history-report "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --output "$(RETRIEVAL_QUALITY_DASHBOARD)"
	$(MAKE) search-quality-gate-v2-check

context-pack-quality-check:
	cargo test -p cortex-engine --test context_pack
	cargo test -p cortex-engine --test context_verify_quality
	$(MAKE) context-pack-explain-v2-check
	$(MAKE) context-pack-prompt-export-check
	$(MAKE) context-pack-answerability-check
	$(MAKE) context-pack-conflict-visibility-check
	$(MAKE) context-pack-private-scope-check
	$(MAKE) context-pack-token-estimator-check
	$(MAKE) context-pack-large-cell-policy-check
	$(MAKE) context-pack-span-packing-check
	python3 scripts/context_pack_quality_check.py --fixture "$(CONTEXT_PACK_QUALITY_FIXTURE)" --report "$(CONTEXT_PACK_QUALITY_REPORT)"
	$(MAKE) context-pack-quality-v3-check

.PHONY: context-pack-span-packing-check
context-pack-span-packing-check:
	cargo test -p cortex-engine --test context_pack_span_packing
	python3 scripts/context_pack_span_packing_check.py --root "." --report "$(CONTEXT_PACK_SPAN_PACKING_REPORT)"

.PHONY: context-pack-large-cell-policy-check
context-pack-large-cell-policy-check:
	cargo test -p cortex-engine --test context_pack_large_cell_policy
	python3 scripts/context_pack_large_cell_policy_check.py --root "." --report "$(CONTEXT_PACK_LARGE_CELL_POLICY_REPORT)"

.PHONY: context-pack-token-estimator-check
context-pack-token-estimator-check:
	cargo test -p cortex-engine --test context_pack_token_estimator
	python3 scripts/context_pack_token_estimator_check.py --root "." --report "$(CONTEXT_PACK_TOKEN_ESTIMATOR_REPORT)"

.PHONY: context-pack-private-scope-check
context-pack-private-scope-check:
	cargo test -p cortex-engine --test context_pack_private_scope
	python3 scripts/context_pack_private_scope_check.py --root "." --report "$(CONTEXT_PACK_PRIVATE_SCOPE_REPORT)"

.PHONY: context-pack-conflict-visibility-check
context-pack-conflict-visibility-check:
	cargo test -p cortex-engine --test context_pack_conflict_visibility
	python3 scripts/context_pack_conflict_visibility_check.py --root "." --report "$(CONTEXT_PACK_CONFLICT_VISIBILITY_REPORT)"

.PHONY: context-pack-answerability-check
context-pack-answerability-check:
	cargo test -p cortex-engine --test context_pack_answerability
	python3 scripts/context_pack_answerability_check.py --root "." --report "$(CONTEXT_PACK_ANSWERABILITY_REPORT)"

.PHONY: context-pack-prompt-export-check
context-pack-prompt-export-check:
	cargo test -p cortex-engine --test context_pack_prompt_export
	python3 scripts/context_pack_prompt_export_check.py --root "." --report "$(CONTEXT_PACK_PROMPT_EXPORT_REPORT)"

.PHONY: context-pack-explain-v2-check
context-pack-explain-v2-check:
	cargo test -p cortex-engine --test context_pack_explain_v2
	python3 scripts/context_pack_explain_v2_check.py --root "." --report "$(CONTEXT_PACK_EXPLAIN_V2_REPORT)"

.PHONY: context-pack-quality-v3-check
context-pack-quality-v3-check:
	python3 scripts/context_pack_quality_v3_check.py --seed-fixture "$(CONTEXT_PACK_QUALITY_FIXTURE)" --datasets "$(CONTEXT_PACK_QUALITY_V3_DATASETS)" --thresholds "$(CONTEXT_PACK_QUALITY_V3_THRESHOLDS)" --report "$(CONTEXT_PACK_QUALITY_V3_REPORT)"

verification-quality-check:
	cargo test -p cortex-engine --test verification_tests
	cargo test -p cortex-engine --test verification_graph_tests
	cargo test -p cortex-engine --test verification_guards
	cargo test -p cortex-engine --test verification_natural_language
	cargo test -p cortex-engine --test verification_evaluation
	python3 scripts/verification_quality_check.py --fixture "$(VERIFICATION_QUALITY_FIXTURE)" --report "$(VERIFICATION_QUALITY_REPORT)"
	python3 scripts/verification_quality_dashboard_self_test.py
	python3 scripts/verification_quality_dashboard.py --report "$(VERIFICATION_QUALITY_REPORT)" --dashboard-json "$(VERIFICATION_QUALITY_DASHBOARD_JSON)" --dashboard-md "$(VERIFICATION_QUALITY_DASHBOARD_MD)"

ingestion-jobs-v2-check:
	cargo test -p cortex-engine --test ingestion_job_tests
	cargo test -p cortex-server ingest_tests
	cargo test -p cortex-cli ingest
	python3 scripts/ingestion_jobs_v2_check.py --self-test
	python3 scripts/ingestion_jobs_v2_check.py --report "$(INGESTION_JOBS_V2_REPORT)"

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
	cargo test -p cortex-server policy_store_context_budget_clamps_agent_view_context_pack_budget
	cargo test -p cortex-server rate_limit_returns_typed_429_when_enabled
	cargo test -p cortex-cli auth_review_rejects_zero_quota
	cargo test -p cortex-cli auth_review_rejects_zero_context_budget
	python3 scripts/enterprise_rbac_gate_check.py --gate quota-policy --report "$(QUOTA_POLICY_REPORT)"

audit-chain-check:
	cargo test -p cortex-server audit_tests
	cargo test -p cortex-cli cli_audit_chain_tests
	cargo test -p cortex-cli audit_command_can_verify_chain
	cargo test -p cortex-cli audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering
	python3 scripts/enterprise_rbac_gate_check.py --gate audit-chain --report "$(AUDIT_CHAIN_REPORT)"

audit-export-retention-check:
	cargo test -p cortex-cli cli_audit_siem_tests
	python3 scripts/audit_export_retention_check.py --report "$(AUDIT_EXPORT_RETENTION_REPORT)"

security-hardening-check: security-check rbac-policy-store-check quota-policy-check audit-chain-check audit-export-retention-check
	python3 scripts/security_hardening_check.py --report "$(SECURITY_HARDENING_REPORT)"

security-gate-v2-check: security-hardening-check
	python3 scripts/security_gate_v2_check.py --security-report "$(SECURITY_REPORT)" --security-hardening-report "$(SECURITY_HARDENING_REPORT)" --rbac-report "$(RBAC_POLICY_STORE_REPORT)" --quota-report "$(QUOTA_POLICY_REPORT)" --audit-chain-report "$(AUDIT_CHAIN_REPORT)" --audit-export-retention-report "$(AUDIT_EXPORT_RETENTION_REPORT)" --report "$(SECURITY_GATE_V2_REPORT)"

security-release-report-check: security-gate-v2-check compliance-boundary-check
	python3 scripts/security_release_report_check.py --security-gate-v2-report "$(SECURITY_GATE_V2_REPORT)" --compliance-boundary-report "$(COMPLIANCE_BOUNDARY_REPORT)" --report "$(SECURITY_RELEASE_REPORT)"

compliance-boundary-check:
	python3 scripts/compliance_boundary_check.py --report "$(COMPLIANCE_BOUNDARY_REPORT)"

metrics-contract-v2-check:
	cargo test -p cortex-server metrics
	python3 scripts/metrics_contract_v2_check.py --report "$(METRICS_CONTRACT_V2_REPORT)"

observability-check: metrics-contract-v2-check
	python3 scripts/observability_check.py --report "$(OBSERVABILITY_REPORT)"

service-manager-smoke-check:
	python3 scripts/service_manager_smoke_check.py --report "$(SERVICE_MANAGER_REPORT)"

docker-hardening-check:
	python3 scripts/docker_hardening_check.py --report "$(DOCKER_HARDENING_REPORT)"

docker-production-compose-check:
	python3 scripts/docker_production_compose_check.py --report "$(DOCKER_PRODUCTION_COMPOSE_REPORT)"

upgrade-rollback-cli-flow-check:
	cargo test -p cortex-cli upgrade_prepare_validate_and_rollback_flow
	cargo test -p cortex-cli upgrade_prepare_json_reports_next_commands
	cargo test -p cortex-cli migrate_offline_creates_backup_drill_rewrites_and_preserves_data
	python3 scripts/upgrade_rollback_cli_flow_check.py --report "$(UPGRADE_ROLLBACK_CLI_FLOW_REPORT)"

package-manager-feasibility-check:
	python3 scripts/package_manager_feasibility_check.py --report "$(PACKAGE_MANAGER_FEASIBILITY_REPORT)"

deployment-upgrade-check: service-manager-smoke-check
	python3 scripts/deployment_upgrade_check.py --report "$(DEPLOYMENT_UPGRADE_REPORT)"

operations-runbook-check:
	python3 scripts/operations_runbook_check.py --report "$(OPERATIONS_RUNBOOK_REPORT)"

incident-playbooks-check:
	python3 scripts/incident_playbooks_check.py --report "target/incident-playbooks/report.json"

doctor-check:
	cargo test -p cortex-cli doctor

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

ann-production-no-fallback-check: ann-fixture-report ann-external-report ann-metric-matrix-report ann-domain-corpus-report ann-recall-probe-report ann-reference-suite-report
	cargo test -p cortex-engine hnsw_no_fallback
	python3 scripts/hnsw_no_fallback_gate_check.py --gate production-no-fallback --evidence fixture="$(ANN_FIXTURE_REPORT)" --evidence external="$(ANN_EXTERNAL_REPORT)" --evidence metric_matrix="$(ANN_METRIC_MATRIX_REPORT)" --evidence domain="$(ANN_DOMAIN_REPORT)" --evidence recall_probe="$(ANN_RECALL_PROBE_REPORT)" --evidence reference_suite="$(ANN_REFERENCE_SUITE_REPORT)" --report "$(HNSW_PRODUCTION_NO_FALLBACK_REPORT)"

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

release-artifact-manifest-production-check:
	python3 scripts/release_artifact_manifest_check.py --version "$(BINARY_RELEASE_VERSION)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)" --evidence-bundle "$(RELEASE_EVIDENCE_BUNDLE_ARCHIVE)" --require-evidence-bundle --manifest "$(RELEASE_ARTIFACT_MANIFEST)" --report "$(RELEASE_ARTIFACT_MANIFEST_REPORT)"

release-evidence-bundle-check:
	python3 scripts/release_evidence_bundle.py --root "$(RELEASE_EVIDENCE_BUNDLE_ROOT)" --manifest "$(RELEASE_EVIDENCE_BUNDLE_MANIFEST)" --report "$(RELEASE_EVIDENCE_BUNDLE_REPORT)" --archive "$(RELEASE_EVIDENCE_BUNDLE_ARCHIVE)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)"

release-notes-generate:
	python3 scripts/generate_release_notes.py --version "$(BINARY_RELEASE_VERSION)" --production-evidence-report "$(PRODUCTION_EVIDENCE_REPORT)" --evidence-bundle-report "$(RELEASE_EVIDENCE_BUNDLE_REPORT)" --release-manifest "$(RELEASE_ARTIFACT_MANIFEST)" --output "$(GENERATED_RELEASE_NOTES)"

evidence-artifact-retention-check:
	python3 scripts/evidence_artifact_retention_check.py --report "$(EVIDENCE_ARTIFACT_RETENTION_REPORT)"

versioning-policy-check:
	python3 scripts/versioning_policy_check.py --report "$(VERSIONING_POLICY_REPORT)"

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

community-roadmap-check:
	python3 scripts/community_roadmap_check.py --report "$(COMMUNITY_ROADMAP_REPORT)"

public-retrieval-benchmark-page-check:
	python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output "$(RETRIEVAL_BETA_REPORT)" --min-domains 4 --repeat-runs 5
	$(MAKE) retrieval-quality-history-check
	python3 scripts/public_retrieval_benchmark_check.py --page docs/PUBLIC_RETRIEVAL_BENCHMARKS.md --beta-report "$(RETRIEVAL_BETA_REPORT)" --history-report "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --report "$(PUBLIC_RETRIEVAL_BENCHMARKS_REPORT)"

public-benchmarks-check: public-retrieval-benchmark-page-check
	python3 scripts/public_benchmarks_check.py --report "$(PUBLIC_BENCHMARKS_REPORT)"

comparison-docs-check:
	python3 scripts/comparison_docs_check.py --report "$(COMPARISON_DOCS_REPORT)"

docs-link-check:
	python3 scripts/docs_link_check.py

getting-started-check:
	python3 scripts/getting_started_check.py

agent-memory-demo-check:
	python3 scripts/agent_memory_demo_check.py --report "$(AGENT_MEMORY_DEMO_REPORT)"

agent-session-check:
	cargo test -p cortex-engine --test agent_session_tests

feedback-learning-check:
	cargo test -p cortex-engine --test feedback_tests --test context_pack

memory-quality-benchmark-check:
	python3 scripts/memory_quality_benchmark_check.py --report "$(MEMORY_QUALITY_BENCHMARK_REPORT)"

tool-registry-check:
	python3 scripts/tool_registry_check.py --report "$(TOOL_REGISTRY_REPORT)"

context-pack-tool-recommendation-check:
	python3 scripts/context_pack_tool_recommendation_check.py --report "$(CONTEXT_PACK_TOOL_RECOMMENDATION_REPORT)"

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

load-suite-check:
	cargo build -p cortex-server --bin cortex-server
	python3 scripts/load_suite_check.py --server ./target/debug/cortex-server --root "target/load-suite" --report "target/load-suite/report.json"

