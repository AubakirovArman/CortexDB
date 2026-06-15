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

audit-productization-check:
	cargo test -p cortex-server audit_tests
	cargo test -p cortex-server denied_ingestion_audit_event_does_not_leak_query_body_or_token
	cargo test -p cortex-cli audit_command_filters_and_checks_redaction
	cargo test -p cortex-cli cli_audit_siem_tests
	python3 scripts/audit_productization_check.py --report "$(AUDIT_PRODUCTIZATION_REPORT)"

security-hardening-check: security-check rbac-policy-store-check quota-policy-check audit-chain-check audit-export-retention-check audit-productization-check
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

docker-quickstart-check:
	python3 scripts/docker_quickstart_check.py --report "$(DOCKER_QUICKSTART_REPORT)"

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
	cargo test -p cortex-cli auth_review_rejects_inline_tokens_argument_without_echoing_value
	cargo test -p cortex-cli encrypted_backup_rejects_passphrase_argument_without_echoing_value
	cargo test -p cortex-server denied_ingestion_audit_event_does_not_leak_query_body_or_token
	python3 scripts/llm_inference_gate_check.py --gate secrets --report "$(LLM_INFERENCE_SECRETS_REPORT)"
	python3 scripts/secrets_hygiene_check.py --report "$(SECRETS_HYGIENE_REPORT)"

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
