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

enterprise-rag-bench-completeness-coverage:
	python3 scripts/enterprise_rag_bench/build_doc_view_subset.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V22)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V30)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --candidate-limit 50 \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --report "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_REPORT)"
	python3 scripts/enterprise_rag_bench/doc_view_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V22)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V30)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V46)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v46_top10_report.json" \
	  --score-candidate-limit 100 \
	  --limit 10 \
	  --seed-count 3 \
	  --protect-baseline-prefix 9 \
	  --route-question-types completeness

enterprise-rag-bench-anchor-candidate-coverage:
	python3 scripts/enterprise_rag_bench/multi_index_candidate_generation.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --base-retrieval-file "$(ENTERPRISE_RAG_BENCH_BASE_CANDIDATES_500)" \
	  --extra-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_TOP5)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CANDIDATES_1000)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_multi_index_v55_neighbor_tail_candidates_top1000_report.json" \
	  --top-k 1000 \
	  --base-limit 500 \
	  --extra-limit 500 \
	  --path-candidate-limit 1200 \
	  --content-candidate-limit 1600 \
	  --content-boost-limit 120 \
	  --content-preview-chars 2200 \
	  --max-posting 12000 \
	  --neighbor-expansion-limit 1200 \
	  --neighbor-seed-limit 40 \
	  --neighbor-max-per-seed 16 \
	  --neighbor-max-posting 250 \
	  --weight-neighbor 0.05 \
	  --diagnostics-top-k 0

enterprise-rag-bench-neighbor-candidate-coverage:
	python3 scripts/enterprise_rag_bench/multi_index_candidate_generation.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --base-retrieval-file "$(ENTERPRISE_RAG_BENCH_BASE_CANDIDATES_500)" \
	  --extra-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_TOP5)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_multi_index_v55_neighbor_tail_candidates_top1000_report.json" \
	  --top-k 1000 \
	  --base-limit 500 \
	  --extra-limit 500 \
	  --path-candidate-limit 1200 \
	  --content-candidate-limit 1600 \
	  --content-boost-limit 120 \
	  --content-preview-chars 2200 \
	  --max-posting 12000 \
	  --neighbor-expansion-limit 1200 \
	  --neighbor-seed-limit 40 \
	  --neighbor-max-per-seed 16 \
	  --neighbor-max-posting 250 \
	  --weight-neighbor 0.05 \
	  --diagnostics-top-k 0
	python3 scripts/enterprise_rag_bench/candidate_depth_audit.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v55_top1000_depth_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v55_top1000_depth_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v55_top1000_depth_report.md" \
	  --depths 10,50,100,500,1000
	python3 scripts/enterprise_rag_bench/candidate_generator_gate.py \
	  --depth-report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v55_top1000_depth_report.json" \
	  --output "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v55_top1000_gate.json" \
	  --min-recall-500 85 \
	  --min-recall-1000 90 \
	  --min-full-recall-1000 400
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V52)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v52_vs_v55_candidate_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v52_vs_v55_candidate_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v52_vs_v55_candidate_comparison_report.md" \
	  --limit 1000

enterprise-rag-bench-source-link-candidate-coverage:
	python3 scripts/enterprise_rag_bench/multi_index_candidate_generation.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --base-retrieval-file "$(ENTERPRISE_RAG_BENCH_BASE_CANDIDATES_500)" \
	  --extra-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_TOP5)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_multi_index_v58_source_links_candidates_top1000_report.json" \
	  --top-k 1000 \
	  --base-limit 500 \
	  --extra-limit 500 \
	  --path-candidate-limit 1200 \
	  --content-candidate-limit 1600 \
	  --content-boost-limit 120 \
	  --content-preview-chars 2200 \
	  --max-posting 12000 \
	  --neighbor-expansion-limit 1200 \
	  --neighbor-seed-limit 40 \
	  --neighbor-max-per-seed 16 \
	  --neighbor-max-posting 250 \
	  --weight-neighbor 0.05 \
	  --enable-source-link-neighbors \
	  --diagnostics-top-k 0
	python3 scripts/enterprise_rag_bench/candidate_depth_audit.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v58_top1000_depth_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v58_top1000_depth_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v58_top1000_depth_report.md" \
	  --depths 10,50,100,500,1000
	python3 scripts/enterprise_rag_bench/candidate_generator_gate.py \
	  --depth-report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v58_top1000_depth_report.json" \
	  --output "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/candidate_v58_top1000_gate.json" \
	  --min-recall-500 85 \
	  --min-recall-1000 90 \
	  --min-full-recall-1000 400
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v55_vs_v58_candidate_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v55_vs_v58_candidate_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v55_vs_v58_candidate_comparison_report.md" \
	  --limit 1000

enterprise-rag-bench-semantic-coverage: enterprise-rag-bench-completeness-coverage
	python3 scripts/enterprise_rag_bench/multi_index_candidate_generation.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --base-retrieval-file "$(ENTERPRISE_RAG_BENCH_BASE_CANDIDATES_500)" \
	  --extra-retrieval-file "$(ENTERPRISE_RAG_BENCH_HYBRID_TOP5)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V48)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_multi_index_v48_candidates_top1000_report.json" \
	  --top-k 1000 \
	  --base-limit 500 \
	  --extra-limit 500 \
	  --path-candidate-limit 1200 \
	  --content-candidate-limit 1600 \
	  --content-boost-limit 120 \
	  --content-preview-chars 2200 \
	  --max-posting 12000 \
	  --diagnostics-top-k 0
	python3 scripts/enterprise_rag_bench/build_doc_view_subset.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V48)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V46)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --candidate-limit 50 \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --report "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_REPORT)"
	python3 scripts/enterprise_rag_bench/doc_view_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V48)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V46)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V51)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v51_top10_report.json" \
	  --score-candidate-limit 100 \
	  --limit 10 \
	  --seed-count 3 \
	  --protect-baseline-prefix 9 \
	  --route-question-types semantic

enterprise-rag-bench-project-related-coverage:
	python3 scripts/enterprise_rag_bench/build_doc_view_subset.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V51)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --candidate-limit 800 \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V55)" \
	  --report "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V55_REPORT)"
	python3 scripts/enterprise_rag_bench/doc_view_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V55)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V51)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V55)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V56)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v56_v55_wide_top10_report.json" \
	  --score-candidate-limit 800 \
	  --limit 10 \
	  --seed-count 3 \
	  --protect-baseline-prefix 9 \
	  --route-question-types project_related \
	  --diagnostics-top-k 0
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V51)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V56)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v51_vs_v56_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v51_vs_v56_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v51_vs_v56_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-github-semantic-query-expansion:
	python3 scripts/enterprise_rag_bench/build_doc_view_subset.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V56)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --candidate-limit 800 \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V58)" \
	  --report "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V58_REPORT)"
	python3 scripts/enterprise_rag_bench/doc_view_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V56)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V58)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V61)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v61_github_semantic_query_expansion_top10_report.json" \
	  --score-candidate-limit 800 \
	  --limit 10 \
	  --seed-count 3 \
	  --protect-baseline-prefix 9 \
	  --route-question-types semantic \
	  --route-source-types github \
	  --diagnostics-top-k 0
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V56)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V61)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v56_vs_v61_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v56_vs_v61_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v56_vs_v61_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-basic-google-drive-tail-rescue:
	python3 scripts/enterprise_rag_bench/candidate_tail_rescue.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V61)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V62)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v62_basic_google_drive_tail_rescue_top10_report.json" \
	  --policy-name basic_google_drive_tail_rescue_v62 \
	  --route-question-types basic \
	  --route-source-types google_drive \
	  --tail-slots 1 \
	  --candidate-rank-limit 50 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V61)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V62)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v61_vs_v62_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v61_vs_v62_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v61_vs_v62_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-completeness-selector:
	python3 scripts/enterprise_rag_bench/confluence_completeness_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V62)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V63)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v63_confluence_completeness_top10_report.json" \
	  --policy-name confluence_completeness_selector_v63 \
	  --candidate-rank-limit 50 \
	  --protect-baseline-prefix 2 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V62)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V63)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v62_vs_v63_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v62_vs_v63_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v62_vs_v63_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-collection-selector:
	python3 scripts/enterprise_rag_bench/confluence_collection_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V63)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V64)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v64_confluence_collection_top10_report.json" \
	  --policy-name confluence_collection_selector_v64 \
	  --candidate-rank-limit 400 \
	  --protect-baseline-prefix 2 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V63)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V64)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v63_vs_v64_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v63_vs_v64_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v63_vs_v64_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-jira-project-source-selector:
	python3 scripts/enterprise_rag_bench/jira_project_source_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V64)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V65)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v65_jira_project_source_top10_report.json" \
	  --policy-name jira_project_source_selector_v65 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V64)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V65)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v64_vs_v65_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v64_vs_v65_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v64_vs_v65_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-jira-completeness-source-selector:
	python3 scripts/enterprise_rag_bench/jira_completeness_source_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V65)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V66)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v66_jira_completeness_source_top10_report.json" \
	  --policy-name jira_completeness_source_selector_v66 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V65)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V66)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v65_vs_v66_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v65_vs_v66_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v65_vs_v66_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-content-completeness-selector:
	python3 scripts/enterprise_rag_bench/confluence_content_completeness_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V66)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V67)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v67_confluence_content_completeness_top10_report.json" \
	  --policy-name confluence_content_completeness_selector_v67 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V66)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V67)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v66_vs_v67_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v66_vs_v67_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v66_vs_v67_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-project-source-selector:
	python3 scripts/enterprise_rag_bench/confluence_project_source_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V67)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V68)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v68_confluence_project_source_top10_report.json" \
	  --policy-name confluence_project_source_selector_v68 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V67)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V68)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v67_vs_v68_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v67_vs_v68_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v67_vs_v68_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-process-completeness-selector:
	python3 scripts/enterprise_rag_bench/confluence_process_completeness_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V68)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V69)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v69_confluence_process_completeness_top10_report.json" \
	  --policy-name confluence_process_completeness_selector_v69 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V68)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V69)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v68_vs_v69_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v68_vs_v69_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v68_vs_v69_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-slack-gmail-source-selector:
	python3 scripts/enterprise_rag_bench/slack_gmail_source_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V69)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V70)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v70_slack_gmail_source_top10_report.json" \
	  --policy-name slack_gmail_source_selector_v70 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V69)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V70)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v69_vs_v70_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v69_vs_v70_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v69_vs_v70_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-hubspot-drive-anchor-selector:
	python3 scripts/enterprise_rag_bench/hubspot_drive_anchor_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V70)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V71)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v71_hubspot_drive_anchor_top10_report.json" \
	  --policy-name hubspot_drive_anchor_selector_v71 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V70)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V71)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v70_vs_v71_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v70_vs_v71_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v70_vs_v71_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-github-project-source-selector:
	python3 scripts/enterprise_rag_bench/github_project_source_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V71)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V72)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v72_github_project_source_top10_report.json" \
	  --policy-name github_project_source_selector_v72 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V71)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V72)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v71_vs_v72_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v71_vs_v72_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v71_vs_v72_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-sdk-auth-completeness-selector:
	python3 scripts/enterprise_rag_bench/sdk_auth_completeness_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V72)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V73)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v73_sdk_auth_completeness_top10_report.json" \
	  --policy-name sdk_auth_completeness_selector_v73 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V72)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V73)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v72_vs_v73_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v72_vs_v73_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v72_vs_v73_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-postmortem-variant-selector:
	python3 scripts/enterprise_rag_bench/confluence_postmortem_variant_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V73)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V74)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v74_confluence_postmortem_variant_top10_report.json" \
	  --policy-name confluence_postmortem_variant_selector_v74 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V73)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V74)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v73_vs_v74_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v73_vs_v74_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v73_vs_v74_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-slack-basic-promotion-selector:
	python3 scripts/enterprise_rag_bench/slack_basic_promotion_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V74)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V75)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v75_slack_basic_promotion_top10_report.json" \
	  --policy-name slack_basic_promotion_selector_v75 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V74)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V75)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v74_vs_v75_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v74_vs_v75_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v74_vs_v75_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-jira-semantic-promotion-selector:
	python3 scripts/enterprise_rag_bench/jira_semantic_promotion_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V75)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V76)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v76_jira_semantic_promotion_top10_report.json" \
	  --policy-name jira_semantic_promotion_selector_v76 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V75)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V76)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v75_vs_v76_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v75_vs_v76_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v75_vs_v76_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-semantic-variant-selector:
	python3 scripts/enterprise_rag_bench/confluence_semantic_variant_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V76)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V77)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v77_confluence_semantic_variant_top10_report.json" \
	  --policy-name confluence_semantic_variant_selector_v77 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V76)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V77)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v76_vs_v77_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v76_vs_v77_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v76_vs_v77_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-linear-semantic-promotion-selector:
	python3 scripts/enterprise_rag_bench/linear_semantic_promotion_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V77)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V78)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v78_linear_semantic_promotion_top10_report.json" \
	  --policy-name linear_semantic_promotion_selector_v78 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V77)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V78)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v77_vs_v78_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v77_vs_v78_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v77_vs_v78_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-jira-project-evidence-selector:
	python3 scripts/enterprise_rag_bench/jira_project_evidence_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V78)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V79)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v79_jira_project_evidence_top10_report.json" \
	  --policy-name jira_project_evidence_selector_v79 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V78)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V79)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v78_vs_v79_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v78_vs_v79_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v78_vs_v79_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-gmail-project-evidence-selector:
	python3 scripts/enterprise_rag_bench/gmail_project_evidence_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V79)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V80)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v80_gmail_project_evidence_top10_report.json" \
	  --policy-name gmail_project_evidence_selector_v80 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V79)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V80)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v79_vs_v80_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v79_vs_v80_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v79_vs_v80_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-confluence-project-discovery-selector:
	python3 scripts/enterprise_rag_bench/confluence_project_discovery_selector.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V80)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V81)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/cortexdb_full_doc_view_v81_confluence_project_discovery_top10_report.json" \
	  --policy-name confluence_project_discovery_selector_v81 \
	  --limit 10
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V80)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V81)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v80_vs_v81_retrieval_comparison_details.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v80_vs_v81_retrieval_comparison_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/analysis/v80_vs_v81_retrieval_comparison_report.md" \
	  --limit 10

enterprise-rag-bench-gold-missing-bottlenecks:
	python3 scripts/enterprise_rag_bench/gold_missing_reason_classifier.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --final-retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_MARKDOWN)" \
	  --compare-retrieval-file v80="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V80)" \
	  --compare-retrieval-file v79="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V79)" \
	  --compare-retrieval-file v78="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V78)" \
	  --compare-retrieval-file v77="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V77)" \
	  --compare-retrieval-file v76="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V76)" \
	  --compare-retrieval-file v75="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V75)" \
	  --compare-retrieval-file v74="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V74)" \
	  --compare-retrieval-file v73="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V73)" \
	  --compare-retrieval-file v72="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V72)" \
	  --compare-retrieval-file v71="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V71)" \
	  --compare-retrieval-file v70="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V70)" \
	  --compare-retrieval-file v69="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V69)" \
	  --compare-retrieval-file v68="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V68)" \
	  --compare-retrieval-file v67="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V67)" \
	  --compare-retrieval-file v66="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V66)" \
	  --compare-retrieval-file v65="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V65)" \
	  --compare-retrieval-file v64="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V64)" \
	  --compare-retrieval-file v63="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V63)" \
	  --compare-retrieval-file v62="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V62)" \
	  --compare-retrieval-file v61="$(ENTERPRISE_RAG_BENCH_DOC_VIEW_V61)"
	python3 scripts/enterprise_rag_bench/gold_missing_bottleneck_report.py \
	  --details-file "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_DETAILS)" \
	  --source-report "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_REPORT)" \
	  --report "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_BOTTLENECK_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_GOLD_MISSING_BOTTLENECK_MARKDOWN)" \
	  --top-limit 12

enterprise-rag-bench-semantic-source-route-sweep:
	python3 scripts/enterprise_rag_bench/source_route_sweep.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_V58)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_V58)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output-dir "$(ENTERPRISE_RAG_BENCH_SOURCE_ROUTE_SWEEP_DIR)" \
	  --report "$(ENTERPRISE_RAG_BENCH_SOURCE_ROUTE_SWEEP_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_SOURCE_ROUTE_SWEEP_MARKDOWN)" \
	  --run-id semantic_source_route_v72 \
	  --source-types "$(ENTERPRISE_RAG_BENCH_SOURCE_ROUTE_SWEEP_TYPES)" \
	  --route-question-types semantic \
	  --score-candidate-limit 800 \
	  --limit 10 \
	  --seed-count 3 \
	  --protect-baseline-prefix 9

enterprise-rag-bench-local-retrieval-gate:
	python3 scripts/enterprise_rag_bench/candidate_depth_audit.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_CURRENT_DEPTH_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CURRENT_DEPTH_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CURRENT_DEPTH_MARKDOWN)"
	python3 scripts/enterprise_rag_bench/evaluate_evidence_pack.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --mode leading \
	  --top-k 10 \
	  --max-chars-per-doc 5000 \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_CURRENT_EVIDENCE_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CURRENT_EVIDENCE_REPORT)"
	python3 scripts/enterprise_rag_bench/summarize_local_calibration.py \
	  --depth-report "$(ENTERPRISE_RAG_BENCH_CURRENT_DEPTH_REPORT)" \
	  --evidence-report "$(ENTERPRISE_RAG_BENCH_CURRENT_EVIDENCE_REPORT)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CURRENT_GATE_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CURRENT_GATE_MARKDOWN)" \
	  --min-top10-recall-pct 70.1 \
	  --max-invalid-extra-docs 8.1
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_BASELINE_BEST)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_CURRENT_COMPARISON_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CURRENT_COMPARISON_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_CURRENT_COMPARISON_MARKDOWN)" \
	  --limit 10

enterprise-rag-bench-high-level-coverage:
	python3 scripts/enterprise_rag_bench/build_doc_view_subset.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_1000)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --candidate-limit 50 \
	  --output "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --report "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES_REPORT)"
	python3 scripts/enterprise_rag_bench/doc_view_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CANDIDATES_1000)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --doc-views-file "$(ENTERPRISE_RAG_BENCH_DOC_VIEWS_CANDIDATES)" \
	  --embedding-cache "$(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/embedding_cache.jsonl" \
	  --output "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_RETRIEVAL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_RETRIEVAL_REPORT)" \
	  --score-candidate-limit 50 \
	  --limit 10 \
	  --seed-count 4 \
	  --protect-baseline-prefix 0 \
	  --route-question-types high_level
	python3 scripts/enterprise_rag_bench/evaluate_high_level_coverage.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --mode "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_COVERAGE_MODE)" \
	  --top-k 10 \
	  --max-chars-per-doc 5000 \
	  --min-fact-token-coverage-pct 60 \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_COVERAGE_DETAILS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_HIGH_LEVEL_COVERAGE_REPORT)"

