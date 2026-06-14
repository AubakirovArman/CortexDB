.PHONY: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check policy-rewrite-gate-check context-pack-schema-contract-check provenance-model-inventory decode-fuzz-check erb-oracle-audit

check: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check policy-rewrite-gate-check context-pack-schema-contract-check decode-fuzz-check erb-oracle-audit
	cargo check --workspace

file-size-report:
	python3 scripts/file_size_report.py --output "$(FILE_SIZE_REPORT)"

file-size-check:
	python3 scripts/file_size_report.py --baseline "$(FILE_SIZE_BASELINE)" --check --output "$(FILE_SIZE_REPORT)"

memtable-clone-gate-check:
	python3 scripts/memtable_clone_gate_check.py

descriptor-hot-path-gate-check:
	python3 scripts/descriptor_hot_path_gate_check.py

indexed-retrieve-gate-check:
	python3 scripts/indexed_retrieve_gate_check.py

query-scan-inventory-check:
	python3 scripts/query_scan_inventory_check.py

policy-rewrite-gate-check:
	python3 scripts/policy_rewrite_gate_check.py

context-pack-schema-contract-check:
	python3 scripts/context_pack_schema_contract_check.py
provenance-model-inventory: ; python3 scripts/provenance_model_inventory.py
erb-oracle-audit:
	python3 scripts/enterprise_rag_bench/oracle_inference_guard.py
decode-fuzz-check:
	cargo test -p cortex-engine --test decode_fuzz --all-features

test:
	cargo test --workspace

include mk/core-contracts.mk
include mk/core-retrieval-context.mk
include mk/core-security-ops.mk
include mk/core-release-public.mk
