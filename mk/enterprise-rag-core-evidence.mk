enterprise-rag-bench-evidence-plan-check: enterprise-rag-bench-balanced-50
	python3 scripts/enterprise_rag_bench/evidence_slot_plan_check.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_EVIDENCE_PLAN_QUESTIONS)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_EVIDENCE_PLAN_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EVIDENCE_PLAN_REPORT)"

enterprise-rag-bench-evidence-table-check: enterprise-rag-bench-balanced-50
	test -f "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_RETRIEVAL)"
	python3 scripts/enterprise_rag_bench/evidence_table_check.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_REPORT)" \
	  --top-docs 10 \
	  --max-facts-per-doc 6

enterprise-rag-bench-evidence-table-extractor-check:
	python3 scripts/enterprise_rag_bench/test_evidence_table_extractor.py

erb-evidence-table-extractor-check: enterprise-rag-bench-evidence-table-extractor-check

enterprise-rag-bench-completeness-plan-check: enterprise-rag-bench-current-best-balanced-50
	test -f "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_RETRIEVAL)"
	python3 scripts/enterprise_rag_bench/completeness_planner.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_REPORT)" \
	  --top-docs "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_TOP_DOCS)" \
	  --max-spans-per-doc "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_MAX_SPANS_PER_DOC)" \
	  --max-chars-per-span "$(ENTERPRISE_RAG_BENCH_COMPLETENESS_PLAN_MAX_CHARS_PER_SPAN)"

erb-completeness-plan-check: enterprise-rag-bench-completeness-plan-check

enterprise-rag-bench-project-answer-synth-check: enterprise-rag-bench-current-best-balanced-50
	test -f "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_RETRIEVAL)"
	python3 scripts/enterprise_rag_bench/project_answer_synthesizer.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_REPORT)" \
	  --top-docs "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_TOP_DOCS)" \
	  --max-rows-per-doc "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_MAX_ROWS_PER_DOC)" \
	  --max-rows-total "$(ENTERPRISE_RAG_BENCH_PROJECT_SYNTH_MAX_ROWS_TOTAL)"

erb-project-answer-synth-check: enterprise-rag-bench-project-answer-synth-check

enterprise-rag-bench-conflict-synth-check: enterprise-rag-bench-current-best-balanced-50
	python3 scripts/enterprise_rag_bench/test_conflict_resolution_synthesizer.py
	test -f "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_RETRIEVAL)"
	python3 scripts/enterprise_rag_bench/conflict_resolution_synthesizer.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_REPORT)" \
	  --top-docs "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_TOP_DOCS)" \
	  --max-rows-total "$(ENTERPRISE_RAG_BENCH_CONFLICT_SYNTH_MAX_ROWS_TOTAL)"

erb-conflict-synth-check: enterprise-rag-bench-conflict-synth-check

enterprise-rag-bench-answer-guard-check:
	python3 scripts/enterprise_rag_bench/test_answer_guard.py

erb-answer-guard-check: enterprise-rag-bench-answer-guard-check

enterprise-rag-bench-answer-context-check:
	python3 scripts/enterprise_rag_bench/test_answer_context.py

erb-answer-context-check: enterprise-rag-bench-answer-context-check

enterprise-rag-bench-answer-intent-check:
	python3 scripts/enterprise_rag_bench/test_answer_intent.py

erb-answer-intent-check: enterprise-rag-bench-answer-intent-check

enterprise-rag-bench-answer-repair-check:
	python3 scripts/enterprise_rag_bench/test_answer_repair.py

erb-answer-repair-check: enterprise-rag-bench-answer-repair-check

enterprise-rag-bench-anchor-overlap-check: enterprise-rag-bench-current-best-balanced-50
	python3 scripts/enterprise_rag_bench/test_anchor_overlap_diagnostics.py
	test -f "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_QUESTIONS)"
	test -f "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_RETRIEVAL)"
	python3 scripts/enterprise_rag_bench/anchor_overlap_diagnostics.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_RETRIEVAL)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_REPORT)" \
	  --details-jsonl "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_DETAILS)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_TOPK)" \
	  --min-recall-pct "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_MIN_RECALL_PCT)" \
	  --max-average-invalid-extra-docs "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_MAX_INVALID_EXTRA_DOCS)" \
	  --min-overlap-doc-pct "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_MIN_OVERLAP_DOC_PCT)" \
	  --strong-overlap-min-anchors "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_STRONG_MIN_ANCHORS)" \
	  --min-strong-overlap-doc-pct "$(ENTERPRISE_RAG_BENCH_ANCHOR_OVERLAP_MIN_STRONG_DOC_PCT)"

erb-anchor-overlap-check: enterprise-rag-bench-anchor-overlap-check

enterprise-rag-bench-current-best-balanced-50: enterprise-rag-bench-balanced-50
	test -f "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)"
	python3 scripts/enterprise_rag_bench/filter_retrieval_to_questions.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST)" \
	  --output "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST_BALANCED_50)"
