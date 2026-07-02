.PHONY: retrieval-recall-baseline-check
retrieval-recall-baseline-check:
	cargo test -p cortex-engine --test retrieval_recall_eval

.PHONY: cli-embedded-query-check
cli-embedded-query-check:
	cargo test -p cortex-embed-client
	cargo test -p cortex-cli search_reports_vector_source_literal_when_vector_given
	cargo test -p cortex-cli search_hybrid_without_vector_or_embedding_config_fails_closed

.PHONY: embedding-model-selection-check
embedding-model-selection-check:
	python3 scripts/embedding_model_eval.py --self-test
	python3 scripts/embedding_model_eval.py --report "$(EMBEDDING_MODEL_SELECTION_REPORT)"

.PHONY: retrieval-diversify-check
retrieval-diversify-check:
	cargo test -p cortex-engine --lib retrieval_rank::diversify

.PHONY: retrieval-temporal-window-check
retrieval-temporal-window-check:
	cargo test -p cortex-engine --lib retrieval_rank::temporal_tests

.PHONY: retrieval-two-stage-rerank-check
retrieval-two-stage-rerank-check:
	cargo test -p cortex-engine --lib retrieval_rank::two_stage
	cargo test -p cortex-engine --lib two_stage_rerank_

.PHONY: retrieval-temporal-supersede-check
retrieval-temporal-supersede-check:
	cargo test -p cortex-engine --lib retrieval_rank::supersede
	cargo test -p cortex-engine --lib verification::temporal_index
	cargo test -p cortex-engine --lib temporal_supersession_

.PHONY: hnsw-build-scaling-check
hnsw-build-scaling-check:
	cargo test -p cortex-engine --lib search::hnsw::index::tests
	cargo test -p cortex-engine synthetic_baseline_gate_passes

.PHONY: ann-guarded-sampling-check
ann-guarded-sampling-check:
	cargo test -p cortex-engine --lib search::ann::guarded_recall

.PHONY: lexical-field-weight-parity-check
lexical-field-weight-parity-check:
	cargo test -p cortex-engine --lib query::metadata::fields
	cargo build -p cortex-bench --bin enterprise_rag_bench_retrieval

retrieval-quality-history-check:
	python3 scripts/retrieval_quality_history_self_test.py
	python3 scripts/retrieval_quality_history.py --domain-root examples/real_domains --output "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --min-domains 4 --history-runs $(RETRIEVAL_QUALITY_HISTORY_RUNS) --fail-on-regression --max-p95-regression-nanos $(RETRIEVAL_QUALITY_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(RETRIEVAL_QUALITY_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(RETRIEVAL_QUALITY_MAX_MAX_REGRESSION_NANOS)

baseline-comparison-check:
	python3 scripts/baseline_comparison_check.py --self-test
	python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output "$(RETRIEVAL_BETA_REPORT)" --min-domains 4 --repeat-runs $(BASELINE_COMPARISON_REPEAT_RUNS)
	python3 scripts/context_pack_quality_v3_check.py --seed-fixture "$(CONTEXT_PACK_QUALITY_FIXTURE)" --datasets "$(CONTEXT_PACK_QUALITY_V3_DATASETS)" --thresholds "$(CONTEXT_PACK_QUALITY_V3_THRESHOLDS)" --report "$(CONTEXT_PACK_QUALITY_V3_REPORT)"
	python3 scripts/baseline_comparison_check.py --datasets "$(CONTEXT_PACK_QUALITY_V3_DATASETS)" --features "$(BASELINE_COMPARISON_FEATURES)" --cortexdb-retrieval-report "$(RETRIEVAL_BETA_REPORT)" --context-pack-report "$(CONTEXT_PACK_QUALITY_V3_REPORT)" --report "$(BASELINE_COMPARISON_REPORT)" --markdown "$(BASELINE_COMPARISON_MARKDOWN)" --top-k $(BASELINE_COMPARISON_TOP_K) --repeat-runs $(BASELINE_COMPARISON_REPEAT_RUNS) --min-domains 4

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
	$(MAKE) ann-budget-disclosure-check
	$(MAKE) context-pack-private-scope-check
	$(MAKE) scope-leak-bench-check
	$(MAKE) context-pack-token-estimator-check
	$(MAKE) context-pack-large-cell-policy-check
	$(MAKE) context-pack-span-packing-check
	$(MAKE) context-pack-value-per-token-check
	python3 scripts/context_pack_quality_check.py --fixture "$(CONTEXT_PACK_QUALITY_FIXTURE)" --report "$(CONTEXT_PACK_QUALITY_REPORT)"
	$(MAKE) context-pack-quality-v3-check

.PHONY: conflict-normalization-check
conflict-normalization-check:
	cargo test -p cortex-engine numeric
	cargo test -p cortex-engine --test conflict_normalization
	$(MAKE) context-pack-conflict-visibility-check
	python3 scripts/conflict_normalization_check.py --root "." --report "$(CONFLICT_NORMALIZATION_REPORT)"

.PHONY: ann-budget-disclosure-check
ann-budget-disclosure-check:
	cargo test -p cortex-engine --test ann_budget_disclosure --all-features
	python3 scripts/ann_budget_disclosure_check.py --root "." --report "$(ANN_BUDGET_DISCLOSURE_REPORT)"

.PHONY: context-pack-value-per-token-check
context-pack-value-per-token-check:
	python3 scripts/context_pack_value_per_token_check.py --report "$(CONTEXT_PACK_VALUE_PER_TOKEN_REPORT)"

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

.PHONY: scope-leak-bench-check
scope-leak-bench-check:
	cargo test -p cortex-engine --test scope_leak_bench --all-features
	python3 scripts/scope_leak_bench_check.py --root "." --report "$(SCOPE_LEAK_BENCH_REPORT)"

.PHONY: segment-pruning-parity-boundary-check
segment-pruning-parity-boundary-check:
	python3 scripts/segment_pruning_parity_boundary_check.py --root "." --report "$(SEGMENT_PRUNING_PARITY_BOUNDARY_REPORT)"

.PHONY: ann-scope-parity-check
ann-scope-parity-check:
	cargo test -p cortex-engine search::database::tests::persisted_search_bound_plan_allowed_set_filters_status_and_where --all-features
	python3 scripts/ann_scope_parity_check.py --root "." --report "$(ANN_SCOPE_PARITY_REPORT)"

.PHONY: ann-sparse-scope-recall-check
ann-sparse-scope-recall-check:
	cargo test -p cortex-engine sparse_allowed_set_routes_to_exact_before_hnsw_budget --all-features
	python3 scripts/ann_sparse_scope_recall_check.py --root "." --report "$(ANN_SPARSE_SCOPE_RECALL_REPORT)"

.PHONY: fail-closed-invariant-model-check
fail-closed-invariant-model-check:
	cargo test -p cortex-aql --test fail_closed_invariant_model
	cargo test -p cortex-engine fail_closed_invariant_model_tests::persisted_ann_and_lexical_paths_respect_fail_closed_model --all-features
	cargo test -p cortex-engine --test fail_closed_invariant_model --all-features -- --nocapture
	python3 scripts/fail_closed_invariant_model_check.py --root "." --report "$(FAIL_CLOSED_INVARIANT_MODEL_REPORT)"

.PHONY: fail-closed-end-to-end-check
fail-closed-end-to-end-check:
	$(MAKE) hnsw-cosine-correctness-check
	$(MAKE) ann-scope-parity-check
	$(MAKE) ann-sparse-scope-recall-check
	$(MAKE) ann-budget-disclosure-check
	$(MAKE) context-access-decision-capture-check
	$(MAKE) scope-leak-bench-check
	$(MAKE) fail-closed-invariant-model-check
	$(MAKE) context-pack-private-scope-check
	$(MAKE) engine-determinism-check
	python3 scripts/fail_closed_end_to_end_check.py --root "." --report "$(FAIL_CLOSED_END_TO_END_REPORT)"

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
	$(MAKE) verify-numeric-normalization-check
	$(MAKE) verify-multivalue-extraction-check
	$(MAKE) verify-temporal-conflict-check
	$(MAKE) verify-citation-conflict-check
	$(MAKE) verify-determinism-check
	$(MAKE) verify-conflict-recall-check
	$(MAKE) verify-docs-claims-check
	cargo test -p cortex-engine --test verification_tests
	cargo test -p cortex-engine --test verification_graph_tests
	cargo test -p cortex-engine --test verification_guards
	cargo test -p cortex-engine --test verification_natural_language
	cargo test -p cortex-engine --test verification_evaluation
	python3 scripts/verification_quality_check.py --fixture "$(VERIFICATION_QUALITY_FIXTURE)" --report "$(VERIFICATION_QUALITY_REPORT)"
	python3 scripts/verification_quality_dashboard_self_test.py
	python3 scripts/verification_quality_dashboard.py --report "$(VERIFICATION_QUALITY_REPORT)" --dashboard-json "$(VERIFICATION_QUALITY_DASHBOARD_JSON)" --dashboard-md "$(VERIFICATION_QUALITY_DASHBOARD_MD)"

.PHONY: verify-numeric-normalization-check
verify-numeric-normalization-check:
	cargo test -p cortex-engine numeric --all-features
	python3 scripts/verify_numeric_normalization_check.py --root "." --report "$(VERIFY_NUMERIC_NORMALIZATION_REPORT)"

.PHONY: verify-multivalue-extraction-check
verify-multivalue-extraction-check:
	cargo test -p cortex-engine numeric --all-features
	cargo test -p cortex-engine --test verification_guards verify_fact_detects_conflict_from_multivalue_evidence_body --all-features
	cargo test -p cortex-engine --test verification_conflict_numeric --all-features
	python3 scripts/verify_multivalue_extraction_check.py --root "." --report "$(VERIFY_MULTIVALUE_EXTRACTION_REPORT)"

.PHONY: verify-temporal-conflict-check
verify-temporal-conflict-check:
	cargo test -p cortex-engine numeric --all-features
	cargo test -p cortex-engine --test verification_guards dated_verify_fact_still_reports_numeric_conflict --all-features
	cargo test -p cortex-engine --test verification_guards stale_evidence_does_not_create_numeric_contradiction --all-features
	cargo test -p cortex-engine --test verification_conflict_numeric temporal_numeric_conflict_index_respects_overlapping_windows --all-features
	python3 scripts/verify_temporal_conflict_check.py --root "." --report "$(VERIFY_TEMPORAL_CONFLICT_REPORT)"

.PHONY: verify-citation-conflict-check
verify-citation-conflict-check:
	cargo test -p cortex-engine --test verification_guards verify_fact_reports_citation_numeric_conflict_kind --all-features
	cargo test -p cortex-engine --test verification_conflict_numeric citation_numeric_conflict_index_tracks_same_source_disagreement --all-features
	cargo test -p cortex-engine --test verification_conflict_numeric same_source_equal_value_is_not_citation_conflict --all-features
	python3 scripts/verify_citation_conflict_check.py --root "." --report "$(VERIFY_CITATION_CONFLICT_REPORT)"

.PHONY: verify-determinism-check
verify-determinism-check:
	cargo test -p cortex-engine --test determinism verification_canonical_conflict_bytes_are_repeatable_and_clock_free --all-features
	cargo test -p cortex-engine canonical --all-features
	python3 scripts/verify_determinism_check.py --root "." --report "$(VERIFY_DETERMINISM_REPORT)"

.PHONY: verify-conflict-recall-check
verify-conflict-recall-check:
	CORTEXDB_VERIFY_CONFLICT_RECALL_REPORT="$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)" cargo test -p cortex-engine --test verification_conflict_recall --all-features
	python3 scripts/verify_conflict_recall_check.py --root "." --report "$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)"

.PHONY: verify-docs-claims-check
verify-docs-claims-check:
	python3 scripts/verify_docs_claims_check.py --root "." --recall-report "$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)" --report "$(CURDIR)/$(VERIFY_DOCS_CLAIMS_REPORT)"

.PHONY: docs-claims-check
docs-claims-check:
	$(MAKE) verify-conflict-recall-check
	$(MAKE) verify-docs-claims-check

ingestion-jobs-v2-check:
	cargo test -p cortex-engine --test ingestion_job_tests
	cargo test -p cortex-server ingest_tests
	cargo test -p cortex-cli ingest
	python3 scripts/ingestion_jobs_v2_check.py --self-test
	python3 scripts/ingestion_jobs_v2_check.py --report "$(INGESTION_JOBS_V2_REPORT)"
