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

