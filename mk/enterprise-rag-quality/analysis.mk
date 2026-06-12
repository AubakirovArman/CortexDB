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

