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

include mk/core-contracts.mk
include mk/core-retrieval-context.mk
include mk/core-security-ops.mk
include mk/core-release-public.mk
