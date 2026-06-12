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

