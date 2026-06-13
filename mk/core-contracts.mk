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
