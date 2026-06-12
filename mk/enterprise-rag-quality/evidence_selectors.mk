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

