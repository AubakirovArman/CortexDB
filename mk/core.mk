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

gce-spec-doc-check:
	python3 scripts/gce_spec_doc_check.py --root "." --report "$(GCE_SPEC_DOC_REPORT)"

receipt-threat-model-check:
	python3 scripts/receipt_threat_model_check.py --root "." --report "$(RECEIPT_THREAT_MODEL_REPORT)"

aab-conformance-check:
	$(MAKE) gce-spec-doc-check
	$(MAKE) receipt-threat-model-check
	$(MAKE) accountability-receipt-verify-check
	$(MAKE) accountability-receipt-tamper-check
	$(MAKE) pack-determinism-hash-check
	$(MAKE) fail-closed-end-to-end-check
	$(MAKE) cosine-metric-correctness-check
	$(MAKE) verification-quality-check
	$(MAKE) audit-receipt-binding-check
	python3 scripts/aab_conformance_check.py --root "." --fixture "$(AAB_CONFORMANCE_FIXTURE)" --report "$(AAB_CONFORMANCE_REPORT)"

agent-transaction-semantics-check:
	python3 scripts/agent_transaction_semantics_check.py --report "$(AGENT_TRANSACTION_SEMANTICS_REPORT)"

learned-ranking-calibration-check:
	python3 scripts/learned_ranking_calibration_check.py \
	  --fixture "$(LEARNED_RANKING_CALIBRATION_FIXTURE)" \
	  --report "$(LEARNED_RANKING_CALIBRATION_REPORT)" \
	  --min-heldout-mrr-lift-bps "$(LEARNED_RANKING_MIN_HELDOUT_MRR_LIFT_BPS)" \
	  --min-heldout-win-rate-pct "$(LEARNED_RANKING_MIN_HELDOUT_WIN_RATE_PCT)"

ranking-frozen-weights-check:
	cargo test -p cortex-engine rerank --all-features
	cargo test -p cortex-engine routing --all-features
	python3 scripts/ranking_frozen_weights_check.py \
	  --root "." \
	  --fixture "$(RANKING_FROZEN_WEIGHTS_FIXTURE)" \
	  --report "$(RANKING_FROZEN_WEIGHTS_REPORT)"

ranking-weights-drift-check:
	python3 scripts/ranking_weights_drift_check.py \
	  --fixture "$(LEARNED_RANKING_CALIBRATION_FIXTURE)" \
	  --checked-in-artifact "$(RANKING_FROZEN_WEIGHTS_FIXTURE)" \
	  --generated-artifact "$(RANKING_WEIGHTS_DRIFT_GENERATED_ARTIFACT)" \
	  --calibration-report "$(RANKING_WEIGHTS_DRIFT_CALIBRATION_REPORT)" \
	  --module-report "$(RANKING_WEIGHTS_DRIFT_MODULE_REPORT)" \
	  --report "$(RANKING_WEIGHTS_DRIFT_REPORT)" \
	  --min-heldout-mrr-lift-bps "$(LEARNED_RANKING_MIN_HELDOUT_MRR_LIFT_BPS)" \
	  --min-heldout-win-rate-pct "$(LEARNED_RANKING_MIN_HELDOUT_WIN_RATE_PCT)"
	$(MAKE) learned-ranking-calibration-check

ranking-learned-lift-check:
	$(MAKE) ranking-weights-drift-check
	CORTEX_RANKING_LEARNED_LIFT_REPORT="$(CURDIR)/$(RANKING_LEARNED_LIFT_REPORT)" \
	  cargo test -p cortex-engine --test ranking_learned_lift --all-features

ranking-explain-faithfulness-check:
	$(MAKE) ranking-frozen-weights-check
	CORTEX_RANKING_EXPLAIN_FAITHFULNESS_REPORT="$(CURDIR)/$(RANKING_EXPLAIN_FAITHFULNESS_REPORT)" \
	  cargo test -p cortex-engine --test ranking_explain_faithfulness --all-features

weights-version-binding-check:
	CORTEX_WEIGHTS_VERSION_BINDING_REPORT="$(CURDIR)/$(WEIGHTS_VERSION_BINDING_REPORT)" \
	  cargo test -p cortex-engine --test weights_version_binding --all-features

pack-determinism-hash-check:
	CORTEX_PACK_DETERMINISM_HASH_REPORT="$(CURDIR)/$(PACK_DETERMINISM_HASH_REPORT)" \
	  cargo test -p cortex-engine --test pack_determinism_hash --all-features

pack-completeness-signal-check:
	$(MAKE) ann-budget-disclosure-check
	CORTEX_PACK_COMPLETENESS_SIGNAL_REPORT="$(CURDIR)/$(PACK_COMPLETENESS_SIGNAL_REPORT)" \
	  cargo test -p cortex-engine --test pack_completeness_signal --all-features

semantic-memory-compression-check:
	python3 scripts/semantic_memory_compression_check.py --report "$(SEMANTIC_MEMORY_COMPRESSION_REPORT)"

multi-agent-consistency-check:
	python3 scripts/multi_agent_consistency_check.py --report "$(MULTI_AGENT_CONSISTENCY_REPORT)"

formal-invariants-check:
	python3 scripts/formal_invariants_check.py --report "$(FORMAL_INVARIANTS_REPORT)"

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
