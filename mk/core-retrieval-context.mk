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
