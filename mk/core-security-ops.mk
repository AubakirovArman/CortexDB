security-check:
	cargo test -p cortex-server security_tests
	cargo test -p cortex-server auth_policy_tests
	cargo test -p cortex-server error_taxonomy_tests
	$(MAKE) openapi-contract-check
	python3 scripts/security_beta_check.py --report "$(SECURITY_REPORT)"

crypto-deps-readiness-check:
	python3 scripts/crypto_deps_readiness_check.py --root "." --report "$(CRYPTO_DEPS_READINESS_REPORT)"

crypto-deps-policy-check:
	$(MAKE) crypto-deps-readiness-check
	python3 scripts/crypto_deps_policy_check.py --root "." --report "$(CRYPTO_DEPS_POLICY_REPORT)"

crypto-primitives-check:
	cargo test -p cortex-crypto
	python3 scripts/crypto_primitives_check.py --root "." --report "$(CRYPTO_PRIMITIVES_REPORT)"

key-management-check:
	cargo test -p cortex-crypto receipt_key
	cargo test -p cortex-server audit_tests
	cargo test -p cortex-server parse_audit_log_mac_key_validates_hex_and_key_id
	cargo test -p cortex-server parse_receipt_signing_key
	cargo test -p cortex-server parse_receipt_external_signer
	cargo test -p cortex-server receipt_signer
	cargo test -p cortex-cli receipt_key_generate_export_and_rotate_preserves_dual_trust
	cargo test -p cortex-cli receipt_key_rotate_writes_verifiable_reanchor_record
	cargo test -p cortex-cli audit_review_verify_chain_requires_mac_key_for_v2_and_rejects_mac_tampering
	cargo test -p cortex-cli audit_verify_alias_accepts_keyed_v2_chain_with_key_file
	python3 scripts/key_management_check.py --root "." --report "$(KEY_MANAGEMENT_REPORT)"

database-instance-identity-check:
	cargo test -p cortex-server database_instance_id --all-features
	python3 scripts/database_instance_identity_check.py --root "." --report "$(DATABASE_INSTANCE_IDENTITY_REPORT)"

crypto-claims-honesty-check:
	python3 scripts/crypto_claims_honesty_check.py --root "." --report "$(CRYPTO_CLAIMS_HONESTY_REPORT)"

crypto-foundation-check:
	$(MAKE) crypto-deps-policy-check
	$(MAKE) crypto-primitives-check
	$(MAKE) encrypted-backup-check
	$(MAKE) encrypted-backup-legacy-refuse-check
	$(MAKE) audit-chain-check
	$(MAKE) audit-receipt-binding-check
	$(MAKE) key-management-check
	$(MAKE) database-instance-identity-check
	$(MAKE) secrets-check
	$(MAKE) crypto-claims-honesty-check
	python3 scripts/crypto_foundation_check.py --root "." --crypto-deps-policy-report "$(CRYPTO_DEPS_POLICY_REPORT)" --crypto-primitives-report "$(CRYPTO_PRIMITIVES_REPORT)" --encrypted-backup-report "$(ENCRYPTED_BACKUP_REPORT)" --encrypted-backup-legacy-refuse-report "$(ENCRYPTED_BACKUP_LEGACY_REFUSE_REPORT)" --audit-chain-report "$(AUDIT_CHAIN_REPORT)" --audit-receipt-binding-report "$(AUDIT_RECEIPT_BINDING_REPORT)" --key-management-report "$(KEY_MANAGEMENT_REPORT)" --database-instance-identity-report "$(DATABASE_INSTANCE_IDENTITY_REPORT)" --llm-secrets-report "$(LLM_INFERENCE_SECRETS_REPORT)" --secrets-hygiene-report "$(SECRETS_HYGIENE_REPORT)" --crypto-claims-honesty-report "$(CRYPTO_CLAIMS_HONESTY_REPORT)" --report "$(CRYPTO_FOUNDATION_REPORT)"

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

audit-receipt-binding-check:
	cargo test -p cortex-server receipt_hash
	cargo test -p cortex-cli audit_review_verify_chain_rejects_receipt_hash_tampering
	python3 scripts/audit_receipt_binding_check.py --root "." --report "$(AUDIT_RECEIPT_BINDING_REPORT)"

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

security-gate-v2-check: security-hardening-check crypto-foundation-check
	python3 scripts/security_gate_v2_check.py --security-report "$(SECURITY_REPORT)" --security-hardening-report "$(SECURITY_HARDENING_REPORT)" --rbac-report "$(RBAC_POLICY_STORE_REPORT)" --quota-report "$(QUOTA_POLICY_REPORT)" --audit-chain-report "$(AUDIT_CHAIN_REPORT)" --audit-export-retention-report "$(AUDIT_EXPORT_RETENTION_REPORT)" --crypto-foundation-report "$(CRYPTO_FOUNDATION_REPORT)" --report "$(SECURITY_GATE_V2_REPORT)"

security-release-report-check: security-gate-v2-check compliance-boundary-check
	python3 scripts/security_release_report_check.py --security-gate-v2-report "$(SECURITY_GATE_V2_REPORT)" --compliance-boundary-report "$(COMPLIANCE_BOUNDARY_REPORT)" --report "$(SECURITY_RELEASE_REPORT)"

compliance-boundary-check:
	python3 scripts/compliance_boundary_check.py --report "$(COMPLIANCE_BOUNDARY_REPORT)" $(COMPLIANCE_BOUNDARY_ARGS) $(COMPLIANCE_BOUNDARY_PRODUCTION_ORIGIN_ARGS)

receipt-kms-hsm-custody-check:
	python3 scripts/receipt_kms_hsm_custody_check.py --root "." --report "$(RECEIPT_KMS_HSM_CUSTODY_REPORT)" $(RECEIPT_KMS_HSM_CUSTODY_ARGS) $(RECEIPT_KMS_HSM_CUSTODY_PRODUCTION_ORIGIN_ARGS)

receipt-production-readiness-check:
	$(MAKE) accountability-receipt-check
	$(MAKE) transparency-slo-check
	$(MAKE) key-management-check
	$(MAKE) receipt-kms-hsm-custody-check
	$(MAKE) receipt-production-evidence-handoff-consistency-check
	$(MAKE) security-release-report-check
	python3 scripts/receipt_production_readiness_check.py --root "." --accountability-receipt-report "$(ACCOUNTABILITY_RECEIPT_REPORT)" --transparency-slo-report "$(TRANSPARENCY_SLO_REPORT)" --key-management-report "$(KEY_MANAGEMENT_REPORT)" --receipt-kms-hsm-custody-report "$(RECEIPT_KMS_HSM_CUSTODY_REPORT)" --handoff-consistency-report "$(RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT)" --security-release-report "$(SECURITY_RELEASE_REPORT)" --compliance-boundary-report "$(COMPLIANCE_BOUNDARY_REPORT)" --report "$(RECEIPT_PRODUCTION_READINESS_REPORT)"

receipt-production-evidence-preflight-check: production-evidence-origin-check receipt-production-evidence-handoff-consistency-check
	python3 scripts/receipt_production_evidence_preflight.py --report "$(RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT)" $(RECEIPT_KMS_HSM_CUSTODY_ARGS) $(COMPLIANCE_BOUNDARY_ARGS)

receipt-production-evidence-production-preflight-check: production-evidence-origin-check receipt-production-evidence-handoff-consistency-check
	python3 scripts/receipt_production_evidence_preflight.py --report "$(RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT)" --require-production-origin-proof $(RECEIPT_KMS_HSM_CUSTODY_ARGS) $(COMPLIANCE_BOUNDARY_ARGS) $(RECEIPT_PRODUCTION_ORIGIN_ARGS)

receipt-production-evidence-handoff-check:
	python3 scripts/receipt_production_evidence_handoff.py --report "$(RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_REPORT)"

receipt-production-evidence-handoff-consistency-check:
	python3 scripts/receipt_production_evidence_handoff_check.py --report "$(RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT)"

production-evidence-origin-check:
	python3 scripts/evidence_origin_check.py --root "." --report "$(PRODUCTION_EVIDENCE_ORIGIN_REPORT)"

receipt-production-ready-check:
	$(MAKE) receipt-production-evidence-production-preflight-check
	$(MAKE) receipt-production-readiness-check
	python3 scripts/receipt_production_readiness_check.py --root "." --accountability-receipt-report "$(ACCOUNTABILITY_RECEIPT_REPORT)" --transparency-slo-report "$(TRANSPARENCY_SLO_REPORT)" --key-management-report "$(KEY_MANAGEMENT_REPORT)" --receipt-kms-hsm-custody-report "$(RECEIPT_KMS_HSM_CUSTODY_REPORT)" --handoff-consistency-report "$(RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT)" --security-release-report "$(SECURITY_RELEASE_REPORT)" --compliance-boundary-report "$(COMPLIANCE_BOUNDARY_REPORT)" --production-evidence-preflight-report "$(RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT)" --report "$(RECEIPT_PRODUCTION_READY_REPORT)" --require-production-ready

metrics-contract-v2-check:
	cargo test -p cortex-server metrics
	python3 scripts/metrics_contract_v2_check.py --report "$(METRICS_CONTRACT_V2_REPORT)"

observability-check: metrics-contract-v2-check
	python3 scripts/observability_check.py --report "$(OBSERVABILITY_REPORT)"

route-timeout-check:
	cargo test -p cortex-server route_timeout
	cargo test -p cortex-server slow_loris_body_times_out_without_blocking_follow_up_request
	cargo test -p cortex-server metrics_prometheus_output_contains_contract_series

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
	cargo test -p cortex-engine --features experimental-replication --test replication_log
	cargo test -p cortex-engine --features experimental-replication --test replication_log_matching
	cargo test -p cortex-engine --features experimental-replication --test replication_commit
	cargo test -p cortex-engine --features experimental-replication --test replication_election
	cargo test -p cortex-engine --features experimental-replication --test replication_membership
	cargo test -p cortex-engine --features experimental-replication --test replication_replay_apply
	python3 scripts/consensus_gate_check.py --gate distributed-consensus --report "$(CONSENSUS_CORE_REPORT)"

consensus-partition-soak-check: replication-partition-check
	python3 scripts/consensus_gate_check.py --gate partition-soak --evidence "$(REPLICATION_PARTITION_REPORT)" --report "$(CONSENSUS_PARTITION_SOAK_REPORT)"

consensus-failover-slo-check: replication-partition-check
	python3 scripts/consensus_gate_check.py --gate failover-slo --evidence "$(REPLICATION_PARTITION_REPORT)" --report "$(CONSENSUS_FAILOVER_SLO_REPORT)"

consensus-rejoin-check: replication-partition-check replication-lifecycle-check
	python3 scripts/consensus_gate_check.py --gate rejoin --evidence "$(REPLICATION_PARTITION_REPORT)" --evidence "$(REPLICATION_LIFECYCLE_REPORT)" --report "$(CONSENSUS_REJOIN_REPORT)"

distributed-consensus-research-check: distributed-consensus-check consensus-partition-soak-check consensus-failover-slo-check consensus-rejoin-check
	python3 scripts/distributed_consensus_research_check.py --report "$(CONSENSUS_RESEARCH_REPORT)"

consensus-release-lane-check:
	python3 scripts/consensus_release_lane_check.py --runs "$(CONSENSUS_RELEASE_LANE_RUNS)" --run-root "$(CONSENSUS_RELEASE_LANE_ROOT)" --report "$(CONSENSUS_RELEASE_LANE_REPORT)"

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
