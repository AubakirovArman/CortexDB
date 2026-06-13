.PHONY: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check context-pack-schema-contract-check decode-fuzz-check

check: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check context-pack-schema-contract-check decode-fuzz-check
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

context-pack-schema-contract-check:
	python3 scripts/context_pack_schema_contract_check.py

decode-fuzz-check:
	cargo test -p cortex-engine --test decode_fuzz --all-features

test:
	cargo test --workspace

include mk/core-contracts.mk
include mk/core-retrieval-context.mk
include mk/core-security-ops.mk
include mk/core-release-public.mk
