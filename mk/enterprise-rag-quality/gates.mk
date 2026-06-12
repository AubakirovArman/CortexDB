enterprise-rag-bench-retrieval-quality-fixture-check:
	python3 scripts/enterprise_rag_bench/retrieval_quality_gate.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_ROOT)/questions.jsonl" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_ROOT)/retrieval.clean.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_MARKDOWN)" \
	  --top-k 3 \
	  --min-average-recall-pct 83 \
	  --min-hit-questions 3 \
	  --min-full-recall-questions 2 \
	  --max-average-invalid-extra-docs 1.34 \
	  --min-mrr 0.61 \
	  --min-ndcg 0.60 \
	  --include-details \
	  --progress-every 1 \
	  --log-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_FIXTURE_STATUS)"

enterprise-rag-bench-category-dashboard-check:
	python3 scripts/enterprise_rag_bench/category_retrieval_dashboard.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RETRIEVAL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_MARKDOWN)" \
	  --history "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_HISTORY)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_TOPK)" \
	  --run-id "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RUN_ID)" \
	  $(if $(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_COMMIT),--commit "$(ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_COMMIT)",)

enterprise-rag-bench-heldout-check:
	python3 scripts/enterprise_rag_bench/heldout_no_overfit_check.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_HELDOUT_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_HELDOUT_RETRIEVAL)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_HELDOUT_OUTPUT_ROOT)" \
	  --report "$(ENTERPRISE_RAG_BENCH_HELDOUT_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_HELDOUT_MARKDOWN)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_HELDOUT_TOPK)" \
	  --heldout-size "$(ENTERPRISE_RAG_BENCH_HELDOUT_SIZE)" \
	  --seed "$(ENTERPRISE_RAG_BENCH_HELDOUT_SEED)" \
	  --max-absolute-recall-delta-pct "$(ENTERPRISE_RAG_BENCH_HELDOUT_MAX_ABS_RECALL_DELTA_PCT)"

erb-holdout-check: enterprise-rag-bench-heldout-check

enterprise-rag-bench-category-regression-check:
	python3 scripts/enterprise_rag_bench/category_regression_gate.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_BASELINE)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_CANDIDATE)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MARKDOWN)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_TOPK)" \
	  --max-category-recall-regression-pct "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MAX_RECALL_DROP_PCT)" \
	  --max-category-precision-regression-pct "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MAX_PRECISION_DROP_PCT)" \
	  --max-category-invalid-extra-regression "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MAX_INVALID_EXTRA_INCREASE)" \
	  --max-category-mrr-regression "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MAX_MRR_DROP)" \
	  --max-category-ndcg-regression "$(ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_MAX_NDCG_DROP)"

erb-category-regression-check: enterprise-rag-bench-category-regression-check

enterprise-rag-bench-hybrid-parity-fixture-check:
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_ROOT)/questions.jsonl" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_ROOT)/reference.python_fusion.jsonl" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_ROOT)/candidate.engine_hybrid.jsonl" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_HYBRID_PARITY_FIXTURE_MARKDOWN)" \
	  --limit 3 \
	  --min-average-recall-delta-pct 0 \
	  --min-full-recall-delta 0 \
	  --min-hit-delta 0 \
	  --max-regressed-questions 0

enterprise-rag-bench-query-understanding-lift-check:
	cargo run -p cortex-engine --bin query_understanding_lift_check -- \
	  --documents "$(ENTERPRISE_RAG_BENCH_QUERY_UNDERSTANDING_LIFT_FIXTURE_ROOT)/documents.jsonl" \
	  --questions "$(ENTERPRISE_RAG_BENCH_QUERY_UNDERSTANDING_LIFT_FIXTURE_ROOT)/questions.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_QUERY_UNDERSTANDING_LIFT_REPORT)" \
	  --top-k 1 \
	  --min-average-recall-delta-pct 50 \
	  --min-full-recall-delta 3 \
	  --min-engine-average-recall-pct 100

enterprise-rag-bench-intent-check:
	cargo run -p cortex-engine --bin enterprise_rag_intent_check -- \
	  --questions "$(ENTERPRISE_RAG_BENCH_INTENT_CHECK_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_INTENT_CHECK_REPORT)" \
	  --offset "$(ENTERPRISE_RAG_BENCH_INTENT_CHECK_OFFSET)" \
	  --min-accuracy-pct "$(ENTERPRISE_RAG_BENCH_INTENT_MIN_ACCURACY_PCT)" \
	  $(if $(ENTERPRISE_RAG_BENCH_INTENT_CHECK_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_INTENT_CHECK_LIMIT)",)

enterprise-rag-bench-decomposition-check:
	cargo run -p cortex-engine --bin enterprise_rag_decomposition_check -- \
	  --questions "$(ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_REPORT)" \
	  --offset "$(ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_OFFSET)" \
	  --min-multi-coverage-pct "$(ENTERPRISE_RAG_BENCH_DECOMPOSITION_MIN_MULTI_COVERAGE_PCT)" \
	  $(if $(ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_LIMIT)",)

enterprise-rag-bench-scope-mapping-check:
	cargo run -p cortex-engine --bin enterprise_rag_scope_mapping_check -- \
	  --questions "$(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_REPORT)" \
	  --offset "$(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_OFFSET)" \
	  --min-project-related-coverage-pct "$(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_MIN_PROJECT_RELATED_COVERAGE_PCT)" \
	  $(if $(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_LIMIT)",)

enterprise-rag-bench-synonym-dictionary-check:
	cargo run -p cortex-engine --bin enterprise_rag_synonym_dictionary_check -- \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT)" \
	  --report "$(ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT)" \
	  --min-terms-with-synonyms "$(ENTERPRISE_RAG_BENCH_SYNONYM_MIN_TERMS_WITH_SYNONYMS)" \
	  --min-term-document-frequency "$(ENTERPRISE_RAG_BENCH_SYNONYM_MIN_TERM_DF)" \
	  --min-pair-document-frequency "$(ENTERPRISE_RAG_BENCH_SYNONYM_MIN_PAIR_DF)" \
	  --max-synonyms-per-term "$(ENTERPRISE_RAG_BENCH_SYNONYM_MAX_SYNONYMS_PER_TERM)" \
	  --max-terms "$(ENTERPRISE_RAG_BENCH_SYNONYM_MAX_TERMS)" \
	  --max-terms-per-document "$(ENTERPRISE_RAG_BENCH_SYNONYM_MAX_TERMS_PER_DOCUMENT)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_SYNONYM_PROGRESS_EVERY)" \
	  $(if $(ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT)",)

enterprise-rag-bench-condition-check:
	cargo run -p cortex-engine --bin enterprise_rag_condition_check -- \
	  --questions "$(ENTERPRISE_RAG_BENCH_CONDITION_CHECK_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CONDITION_CHECK_REPORT)" \
	  --offset "$(ENTERPRISE_RAG_BENCH_CONDITION_CHECK_OFFSET)" \
	  --min-constrained-coverage-pct "$(ENTERPRISE_RAG_BENCH_CONDITION_MIN_CONSTRAINED_COVERAGE_PCT)" \
	  $(if $(ENTERPRISE_RAG_BENCH_CONDITION_CHECK_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_CONDITION_CHECK_LIMIT)",)

enterprise-rag-bench-calibration-profile-check:
	cargo run -p cortex-engine --bin enterprise_rag_calibration_check -- \
	  --questions "$(ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_REPORT)" \
	  --offset "$(ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_OFFSET)" \
	  --min-calibrated-pct "$(ENTERPRISE_RAG_BENCH_CALIBRATION_MIN_CALIBRATED_PCT)" \
	  --min-semantic-vector-pct "$(ENTERPRISE_RAG_BENCH_CALIBRATION_MIN_SEMANTIC_VECTOR_PCT)" \
	  --min-constrained-condition-pct "$(ENTERPRISE_RAG_BENCH_CALIBRATION_MIN_CONSTRAINED_CONDITION_PCT)" \
	  $(if $(ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_LIMIT),--limit "$(ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_LIMIT)",)

enterprise-rag-bench-candidate-depth-check:
	python3 scripts/enterprise_rag_bench/candidate_depth_audit.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_1000)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_CANDIDATE_DEPTH_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CANDIDATE_DEPTH_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CANDIDATE_DEPTH_MARKDOWN)" \
	  --depths 10,50,100,500,1000
	python3 scripts/enterprise_rag_bench/candidate_generator_gate.py \
	  --depth-report "$(ENTERPRISE_RAG_BENCH_CANDIDATE_DEPTH_REPORT)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CANDIDATE_GATE_REPORT)" \
	  --min-recall-500 85 \
	  --min-recall-1000 90 \
	  --min-full-recall-1000 400

