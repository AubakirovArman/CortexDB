enterprise-rag-bench-deepseek-answers-50: enterprise-rag-bench-cortexdb-retrieval-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_ANSWER_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_MAX_CHARS_PER_DOC)" \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-50: enterprise-rag-bench-embedding-rerank-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_MAX_CHARS_PER_DOC)" \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-v2-50:
	test -f "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)"
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V2_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V2_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V2_MAX_TOKENS)" \
	  --prompt-style fact-focused-v2 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-v3-windowed-50:
	test -f "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)"
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_TOKENS)" \
	  --context-mode question-window \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v4-windowed-50: enterprise-rag-bench-embedding-rerank-fused-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_TOKENS)" \
	  --context-mode question-window \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v5-windowed-50: enterprise-rag-bench-embedding-rerank-fused-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V5_MAX_TOKENS)" \
	  --context-mode question-window \
	  --prompt-style evidence-selection-v5 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v6-lexical-windowed-50: enterprise-rag-bench-embedding-rerank-fused-v6-lexical-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_V6_LEXICAL_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V5_MAX_TOKENS)" \
	  --context-mode question-window \
	  --prompt-style evidence-selection-v5 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v8-selective-lexical-windowed-50: enterprise-rag-bench-routed-v8-selective-lexical-retrieval-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V5_MAX_TOKENS)" \
	  --context-mode question-window \
	  --prompt-style evidence-selection-v5 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v9-type-aware-windowed-50: enterprise-rag-bench-routed-v8-selective-lexical-retrieval-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V9_MAX_TOKENS)" \
	  --context-mode question-window \
	  --prompt-style type-aware-v9 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v10-project-chain-windowed-50: enterprise-rag-bench-routed-v10-project-chain-retrieval-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V10_MAX_TOKENS)" \
	  --context-mode question-window \
	  --prompt-style type-aware-v9 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v11-evidence-audit-windowed-50: enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V11_MAX_TOKENS)" \
	  --context-mode question-window-digest \
	  --prompt-style evidence-audit-v11 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v12-type-aware-digest-windowed-50: enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V12_MAX_TOKENS)" \
	  --context-mode question-window-digest \
	  --prompt-style type-aware-v9 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v13-source-truth-digest-windowed-50: enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V13_MAX_TOKENS)" \
	  --context-mode question-window-digest \
	  --prompt-style type-aware-v13 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v15-coverage-ranked-windowed-50: enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V3_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V3_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V15_MAX_TOKENS)" \
	  --context-mode question-window-digest-ranked \
	  --prompt-style type-aware-v15 \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)"

enterprise-rag-bench-deepseek-answers-routed-v17-evidence-first-50: enterprise-rag-bench-current-best-balanced-50 enterprise-rag-bench-evidence-plan-check enterprise-rag-bench-evidence-table-check
	python3 scripts/enterprise_rag_bench/run_deepseek_answers.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_CURRENT_BEST_BALANCED_50)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_QA_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_QA_MODEL)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_QA_V17_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_QA_V17_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_QA_V17_MAX_TOKENS)" \
	  --context-mode evidence-first \
	  --prompt-style evidence-first-v18 \
	  --include-evidence-plan \
	  --evidence-plan-file "$(ENTERPRISE_RAG_BENCH_EVIDENCE_PLAN_JSONL)" \
	  --include-evidence-table \
	  --evidence-table-file "$(ENTERPRISE_RAG_BENCH_EVIDENCE_TABLE_JSONL)" \
	  --max-evidence-table-rows "$(ENTERPRISE_RAG_BENCH_QA_V17_MAX_EVIDENCE_TABLE_ROWS)" \
	  --workers "$(ENTERPRISE_RAG_BENCH_QA_WORKERS)" \
	  --omit-thinking-field \
	  --log-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_STATUS)"

