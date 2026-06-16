.PHONY: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check policy-rewrite-gate-check multi-brain-contract-check lexical-index-contract-check context-pack-schema-contract-check provenance-model-inventory decode-fuzz-check erb-oracle-audit semantic-memory-compression-check multi-agent-consistency-check

check: memtable-clone-gate-check descriptor-hot-path-gate-check indexed-retrieve-gate-check query-scan-inventory-check policy-rewrite-gate-check multi-brain-contract-check lexical-index-contract-check tool-registry-check knowledge-graph-check context-pack-schema-contract-check decode-fuzz-check erb-oracle-audit erb-category-regression-check
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

multi-brain-contract-check:
	python3 scripts/multi_brain_contract_check.py

lexical-index-contract-check:
	python3 scripts/lexical_index_contract_check.py

context-pack-schema-contract-check:
	python3 scripts/context_pack_schema_contract_check.py
agent-transaction-semantics-check:
	python3 scripts/agent_transaction_semantics_check.py --report "$(AGENT_TRANSACTION_SEMANTICS_REPORT)"

learned-ranking-calibration-check:
	python3 scripts/learned_ranking_calibration_check.py \
	  --fixture "$(LEARNED_RANKING_CALIBRATION_FIXTURE)" \
	  --report "$(LEARNED_RANKING_CALIBRATION_REPORT)" \
	  --min-heldout-mrr-lift-bps "$(LEARNED_RANKING_MIN_HELDOUT_MRR_LIFT_BPS)" \
	  --min-heldout-win-rate-pct "$(LEARNED_RANKING_MIN_HELDOUT_WIN_RATE_PCT)"

semantic-memory-compression-check:
	python3 scripts/semantic_memory_compression_check.py --report "$(SEMANTIC_MEMORY_COMPRESSION_REPORT)"

multi-agent-consistency-check:
	python3 scripts/multi_agent_consistency_check.py --report "$(MULTI_AGENT_CONSISTENCY_REPORT)"

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
