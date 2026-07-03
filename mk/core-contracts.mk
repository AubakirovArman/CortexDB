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

sdk-public-registry-smoke:
	python3 scripts/sdk_public_registry_smoke.py --report "$(SDK_PUBLIC_REGISTRY_SMOKE_REPORT)"

openapi-check:
	python3 scripts/check_openapi_coverage.py

openapi-sdk-codegen-control-check:
	python3 scripts/check_openapi_sdk_codegen_control.py

openapi-sdk-generated-types-check:
	python3 scripts/generate_openapi_sdk_types.py --check

openapi-contract-check:
	$(MAKE) openapi-check
	python3 scripts/check_openapi_contract.py
	python3 scripts/check_error_taxonomy_contract.py
	$(MAKE) openapi-sdk-generated-types-check
	$(MAKE) openapi-sdk-codegen-control-check

sdk-contract-check:
	python3 scripts/check_sdk_contract.py

sdk-e2e-release-check:
	$(MAKE) sdk-release-contract-check
	$(MAKE) sdk-deprecation-check
	$(MAKE) sdk-release-artifacts-check
	$(MAKE) sdk-registry-gate-check
	$(MAKE) sdk-contract-check
	python3 scripts/sdk_e2e_release_check.py --report "$(SDK_E2E_RELEASE_REPORT)"

sdk-productization-check: sdk-e2e-release-check sdk-public-registry-smoke
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
	$(MAKE) verify-determinism-check
	python3 scripts/engine_determinism_check.py --report "$(ENGINE_DETERMINISM_REPORT)"

canonical-serialization-check:
	cargo test -p cortex-engine canonical
	python3 scripts/canonical_serialization_check.py --report "$(CANONICAL_SERIALIZATION_REPORT)"

accountability-canonical-check: canonical-serialization-check

accountability-cell-hash-check:
	cargo test -p cortex-engine --test accountability_cell_hash --all-features
	python3 scripts/accountability_cell_hash_check.py --root "." --report "$(ACCOUNTABILITY_CELL_HASH_REPORT)"

context-access-decision-capture-check:
	cargo test -p cortex-engine --lib retrieve_execution_report_captures_permission_denials_without_forbidden_payload
	cargo test -p cortex-engine --test context_access_decision_capture --all-features
	python3 scripts/context_access_decision_capture_check.py --root "." --report "$(CONTEXT_ACCESS_DECISION_CAPTURE_REPORT)"

.PHONY: receipt-emission-budget-check
receipt-emission-budget-check:
	CORTEX_RECEIPT_EMISSION_BUDGET_REPORT="$(CURDIR)/target/receipt-emission-budget/report.json" cargo test -p cortex-engine --lib receipt_emission_p99_is_within_budget --release

.PHONY: memory-consolidation-check
memory-consolidation-check:
	cargo test -p cortex-engine --lib semantic_compression::memory_class_tests
	cargo test -p cortex-engine --test semantic_compression --all-features

.PHONY: read-after-seq-check
read-after-seq-check:
	cargo test -p cortex-engine --test multi_agent_consistency require_seq_visible --all-features

.PHONY: idempotency-ledger-check
idempotency-ledger-check:
	cargo test -p cortex-engine --lib idempotency
	cargo test -p cortex-engine --test multi_agent_consistency idempoten --all-features

.PHONY: handoff-ledger-check
handoff-ledger-check:
	cargo test -p cortex-engine --test multi_agent_consistency handoff --all-features

.PHONY: agent-transactions-contract-check
agent-transactions-contract-check:
	cargo test -p cortex-api-types agent_transaction

.PHONY: agent-handoff-route-check
agent-handoff-route-check:
	cargo test -p cortex-server agent_transaction::tests
	python3 scripts/check_openapi_coverage.py

.PHONY: memory-consolidate-route-check
memory-consolidate-route-check:
	cargo test -p cortex-api-types memory_consolidation
	cargo test -p cortex-server memory_consolidation::tests
	python3 scripts/check_openapi_coverage.py

.PHONY: canonical-jcs-cross-language-check
canonical-jcs-cross-language-check:
	cargo test -p cortex-engine --lib canonical::tests::jcs_cross_language_vectors_match
	cargo test -p cortex-engine --lib accountability::receipt_tests::merkle_root_matches_cross_language_vectors
	cargo test -p cortex-engine --lib accountability::receipt_tests::ed25519_signature_matches_cross_language_vectors
	cargo test -p cortex-engine --lib accountability::receipt_tests::pack_root_matches_cross_language_vector
	cargo test -p cortex-engine --lib accountability::receipt_tests::pack_leaf_families_match_cross_language_vector
	python3 scripts/canonical_jcs_cross_language_check.py

accountability-receipt-schema-check:
	cargo test -p cortexdb-sdk context_pack_v1_deserializes_optional_accountability_receipt
	python3 scripts/accountability_receipt_schema_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_SCHEMA_REPORT)"

accountability-receipt-determinism-check:
	cargo test -p cortex-engine accountability_receipt_body --all-features
	python3 scripts/accountability_receipt_determinism_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_DETERMINISM_REPORT)"

accountability-receipt-sign-check:
	cargo test -p cortex-engine accountability_receipt_header --all-features
	python3 scripts/accountability_receipt_sign_check.py --root "." --report "$(ACCOUNTABILITY_RECEIPT_SIGN_REPORT)"

accountability-receipt-verify-check:
	cargo test -p cortex-receipt-verify
	cargo run -p cortex-receipt-verify -- --input "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)"
	python3 scripts/accountability_receipt_verify_check.py --root "." --fixture "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)" --report "$(ACCOUNTABILITY_RECEIPT_VERIFY_REPORT)"

accountability-receipt-tamper-check:
	python3 scripts/accountability_receipt_tamper_check.py --root "." --fixture "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)" --report "$(ACCOUNTABILITY_RECEIPT_TAMPER_REPORT)"

canonical-schema-field-binding-check:
	cargo test -p cortex-engine --lib canonical_field_sets_are_bound_to_schema_versions

accountability-receipt-check:
	$(MAKE) accountability-receipt-schema-check
	$(MAKE) canonical-schema-field-binding-check
	$(MAKE) accountability-receipt-determinism-check
	$(MAKE) accountability-receipt-sign-check
	$(MAKE) accountability-receipt-verify-check
	$(MAKE) accountability-receipt-tamper-check
	python3 scripts/accountability_receipt_check.py --root "." --schema-report "$(ACCOUNTABILITY_RECEIPT_SCHEMA_REPORT)" --determinism-report "$(ACCOUNTABILITY_RECEIPT_DETERMINISM_REPORT)" --sign-report "$(ACCOUNTABILITY_RECEIPT_SIGN_REPORT)" --verify-report "$(ACCOUNTABILITY_RECEIPT_VERIFY_REPORT)" --tamper-report "$(ACCOUNTABILITY_RECEIPT_TAMPER_REPORT)" --report "$(ACCOUNTABILITY_RECEIPT_REPORT)"

receipt-replica-invariance-check:
	cargo test -p cortex-engine --test receipt_replica_invariance --all-features
	cargo test -p cortex-engine accountability_receipt_header_is_replica_invariant_for_same_committed_inputs --all-features
	cargo test -p cortex-engine accountability_receipt_header_changes_when_audit_chain_head_changes --all-features
	cargo test -p cortex-receipt-verify
	$(MAKE) accountability-receipt-schema-check
	$(MAKE) accountability-receipt-verify-check
	$(MAKE) transparency-anchor-check
	python3 scripts/receipt_replica_invariance_check.py --root "." --fixture "$(ACCOUNTABILITY_RECEIPT_VERIFY_FIXTURE)" --report "$(RECEIPT_REPLICA_INVARIANCE_REPORT)"

consensus-failover-binder-check:
	cargo test -p cortex-engine --test cluster_fail_closed --all-features
	cargo test -p cortex-engine --test replication_partition_matrix --all-features
	python3 scripts/consensus_failover_binder_check.py --root "." --report "$(CONSENSUS_FAILOVER_BINDER_REPORT)"

multi-agent-cluster-consistency-check:
	$(MAKE) multi-agent-consistency-check
	cargo test -p cortex-engine --test multi_agent_cluster_consistency --all-features
	python3 scripts/multi_agent_cluster_consistency_check.py --root "." --report "$(MULTI_AGENT_CLUSTER_CONSISTENCY_REPORT)"

http-raft-routing-accountability-check:
	cargo test -p cortex-server http_raft_arbitrary_node_context_receipts_use_replicated_snapshot --all-features
	python3 scripts/http_raft_routing_accountability_check.py --root "." --report "$(HTTP_RAFT_ROUTING_ACCOUNTABILITY_REPORT)"

raft-ingress-production-guard-check:
	cargo test -p cortex-server cluster_ingress_guard_tests --all-features
	python3 scripts/raft_ingress_production_guard_check.py --root "." --report "$(RAFT_INGRESS_PRODUCTION_GUARD_REPORT)"

raft-ingress-forwarding-check:
	cargo test -p cortex-server cluster_ingress_guard_tests --all-features
	python3 scripts/raft_ingress_forwarding_check.py --root "." --report "$(RAFT_INGRESS_FORWARDING_REPORT)"

raft-ingress-leader-hint-check:
	cargo test -p cortex-server cluster_ingress_leader_hint_tests --all-features
	cargo test -p cortex-server parse_cluster_ingress_leader_accepts_positive_node_id
	python3 scripts/raft_ingress_leader_hint_check.py --root "." --report "$(RAFT_INGRESS_LEADER_HINT_REPORT)"

raft-ingress-auto-discovery-check:
	cargo test -p cortex-engine --test replication_cluster_config cluster_config_roundtrips_optional_ingress_addresses --all-features
	cargo test -p cortex-engine --test replication_transport replication_status_frame_reports_known_leader_without_log_mutation --all-features
	cargo test -p cortex-server cluster_ingress_discovery_tests --all-features
	python3 scripts/raft_ingress_auto_discovery_check.py --root "." --report "$(RAFT_INGRESS_AUTO_DISCOVERY_REPORT)"

raft-ingress-health-routing-check:
	cargo test -p cortex-server cluster_ingress_health_tests --all-features
	python3 scripts/raft_ingress_health_routing_check.py --root "." --report "$(RAFT_INGRESS_HEALTH_ROUTING_REPORT)"

raft-ingress-lifecycle-monitor-check:
	cargo test -p cortex-server production_monitor_uses_cached_leader_after_status_peer_exits --all-features
	python3 scripts/raft_ingress_lifecycle_monitor_check.py --root "." --report "$(RAFT_INGRESS_LIFECYCLE_MONITOR_REPORT)"

raft-ingress-load-policy-check:
	cargo test -p cortex-server cluster_ingress_load_tests --all-features
	python3 scripts/raft_ingress_load_policy_check.py --root "." --report "$(RAFT_INGRESS_LOAD_POLICY_REPORT)"

raft-ingress-adaptive-scheduling-check: raft-ingress-load-policy-check
	cargo test -p cortex-server cluster_ingress_adaptive_tests --all-features
	python3 scripts/raft_ingress_adaptive_scheduling_check.py --root "." --report "$(RAFT_INGRESS_ADAPTIVE_SCHEDULING_REPORT)"

raft-ingress-load-metrics-check: raft-ingress-adaptive-scheduling-check
	cargo test -p cortex-server parse_cluster_ingress_max_in_flight --all-features
	cargo test -p cortex-server metrics_prometheus_output_contains_contract_series --all-features
	python3 scripts/raft_ingress_load_metrics_check.py --root "." --report "$(RAFT_INGRESS_LOAD_METRICS_REPORT)"

transparency-anchor-check:
	cargo test -p cortex-engine transparency_log --all-features
	cargo test -p cortex-server parse_transparency_log_path
	python3 scripts/transparency_anchor_check.py --root "." --report "$(TRANSPARENCY_ANCHOR_REPORT)"

transparency-witness-check:
	$(MAKE) transparency-anchor-check
	cargo test -p cortex-engine transparency_witness --all-features
	python3 scripts/transparency_witness_check.py --root "." --report "$(TRANSPARENCY_WITNESS_REPORT)"

transparency-witness-quorum-check:
	$(MAKE) transparency-witness-check
	cargo test -p cortex-engine transparency_witness_quorum --all-features
	python3 scripts/transparency_witness_quorum_check.py --root "." --report "$(TRANSPARENCY_WITNESS_QUORUM_REPORT)"

transparency-inclusion-check:
	$(MAKE) transparency-witness-quorum-check
	cargo test -p cortex-engine transparency_inclusion --all-features
	python3 scripts/transparency_inclusion_check.py --root "." --report "$(TRANSPARENCY_INCLUSION_REPORT)"

transparency-consistency-check:
	$(MAKE) transparency-inclusion-check
	cargo test -p cortex-engine transparency_consistency --all-features
	python3 scripts/transparency_consistency_check.py --root "." --report "$(TRANSPARENCY_CONSISTENCY_REPORT)"

transparency-availability-check:
	$(MAKE) transparency-consistency-check
	cargo test -p cortex-engine transparency_availability --all-features
	python3 scripts/transparency_availability_check.py --root "." --report "$(TRANSPARENCY_AVAILABILITY_REPORT)"

transparency-gossip-check:
	$(MAKE) transparency-availability-check
	cargo test -p cortex-engine transparency_gossip --all-features
	python3 scripts/transparency_gossip_check.py --root "." --report "$(TRANSPARENCY_GOSSIP_REPORT)"

transparency-slo-check:
	$(MAKE) transparency-gossip-check
	cargo test -p cortex-engine transparency_slo --all-features
	python3 scripts/transparency_slo_check.py --root "." --report "$(TRANSPARENCY_SLO_REPORT)"

correctness-prerequisites-check:
	$(MAKE) cosine-metric-correctness-check
	$(MAKE) cell-id-collision-check
	$(MAKE) conflict-normalization-check
	$(MAKE) ann-budget-disclosure-check
	$(MAKE) ann-metric-matrix-check
	$(MAKE) context-pack-conflict-visibility-check
	$(MAKE) engine-determinism-check
	python3 scripts/correctness_prerequisites_check.py --root "." --report "$(CORRECTNESS_PREREQUISITES_REPORT)"

cell-id-collision-check:
	cargo test -p cortex-engine --test agent_session_tests session_cell_ids
	cargo test -p cortex-engine --test feedback_tests feedback_cell_ids
	cargo test -p cortex-engine --test remember_write_contract_tests remember_
	python3 scripts/cell_id_collision_check.py --report "$(CELL_ID_COLLISION_REPORT)"

engine-panic-audit-check:
	python3 scripts/engine_panic_audit_check.py --report "target/engine-panic-audit/report.json"

engine-api-check: engine-public-api-freeze-check engine-api-compat-check engine-error-model-check engine-feature-flags-check module-ownership-check engine-internal-boundary-check engine-determinism-check engine-panic-audit-check

.PHONY: aql-changelog-policy-check
aql-changelog-policy-check:
	python3 scripts/check_aql_changelog_policy.py --report "target/aql-changelog-policy/report.json"

aql-compat-check:
	python3 scripts/aql_compat_check.py --root "$(AQL_COMPAT_ROOT)" --report "$(AQL_COMPAT_REPORT)"
