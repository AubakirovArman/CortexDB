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

