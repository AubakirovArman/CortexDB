enterprise-rag-bench-official-retrieval-only-metrics-50: enterprise-rag-bench-official-env enterprise-rag-bench-cortexdb-retrieval-50
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_50))" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/questions_updated.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

enterprise-rag-bench-official-retrieval-only-metrics-existing-50: enterprise-rag-bench-official-env
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_50))" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/questions_updated.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates: enterprise-rag-bench-balanced-50
	test -d "$(ENTERPRISE_RAG_BENCH_DB_50)"
	cargo build --release -p cortex-engine --bin enterprise_rag_bench_retrieval
	./target/release/enterprise_rag_bench_retrieval \
	  --questions "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --db-root "$(ENTERPRISE_RAG_BENCH_DB_50)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_REPORT)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_RERANK_CANDIDATE_TOPK)" \
	  --skip-ingest \
	  --progress-every 10

enterprise-rag-bench-embedding-rerank-existing-50: enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates
	python3 scripts/enterprise_rag_bench/rerank_with_embeddings.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50_REPORT)" \
	  --cache-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_CACHE)" \
	  --env-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_ENV_FILE)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_RERANK_FINAL_TOPK)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_RERANK_BATCH_SIZE)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_RERANK_MAX_CHARS_PER_DOC)" \
	  $$(if [ -n "$(ENTERPRISE_RAG_BENCH_RERANK_LIMIT)" ]; then printf '%s ' --limit "$(ENTERPRISE_RAG_BENCH_RERANK_LIMIT)"; fi)

enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates-wide: enterprise-rag-bench-balanced-50
	test -d "$(ENTERPRISE_RAG_BENCH_DB_50)"
	cargo build --release -p cortex-engine --bin enterprise_rag_bench_retrieval
	./target/release/enterprise_rag_bench_retrieval \
	  --questions "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --db-root "$(ENTERPRISE_RAG_BENCH_DB_50)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_WIDE)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_WIDE_REPORT)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_RERANK_WIDE_CANDIDATE_TOPK)" \
	  --skip-ingest \
	  --progress-every 10

enterprise-rag-bench-embedding-rerank-wide-existing-50: enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates-wide
	python3 scripts/enterprise_rag_bench/rerank_with_embeddings.py \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_WIDE)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_WIDE_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_WIDE_50_REPORT)" \
	  --cache-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_CACHE)" \
	  --env-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_ENV_FILE)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_RERANK_FINAL_TOPK)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_RERANK_BATCH_SIZE)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_RERANK_MAX_CHARS_PER_DOC)" \
	  $$(if [ -n "$(ENTERPRISE_RAG_BENCH_RERANK_LIMIT)" ]; then printf '%s ' --limit "$(ENTERPRISE_RAG_BENCH_RERANK_LIMIT)"; fi)

enterprise-rag-bench-embedding-rerank-fused-existing-50: enterprise-rag-bench-embedding-rerank-existing-50 enterprise-rag-bench-embedding-rerank-wide-existing-50
	python3 scripts/enterprise_rag_bench/fuse_retrieval_outputs.py \
	  --input "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --input "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_WIDE_50)" \
	  --output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50_REPORT)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_RERANK_FINAL_TOPK)"

enterprise-rag-bench-embedding-rerank-fused-v6-lexical-existing-50: enterprise-rag-bench-embedding-rerank-existing-50 enterprise-rag-bench-embedding-rerank-wide-existing-50
	python3 scripts/enterprise_rag_bench/fuse_retrieval_outputs.py \
	  --input "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50)" \
	  --input "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_WIDE_50)" \
	  --input "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES)" \
	  --weight 1 \
	  --weight 1 \
	  --weight "$(ENTERPRISE_RAG_BENCH_FUSED_V6_LEXICAL_WEIGHT)" \
	  --rrf-k "$(ENTERPRISE_RAG_BENCH_FUSED_V6_LEXICAL_RRF_K)" \
	  --output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_V6_LEXICAL_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_V6_LEXICAL_50_REPORT)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_RERANK_FINAL_TOPK)"

enterprise-rag-bench-routed-v8-selective-lexical-retrieval-50:
	test -f "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50)"
	test -f "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_V6_LEXICAL_50)"
	python3 scripts/enterprise_rag_bench/combine_routed_retrieval_outputs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_50)" \
	  --routed-retrieval-file "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_FUSED_V6_LEXICAL_50)" \
	  --output "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50_REPORT)" \
	  --routed-question-types "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_ROUTE_TYPES)"

enterprise-rag-bench-routed-v10-project-chain-retrieval-50: enterprise-rag-bench-routed-v8-selective-lexical-retrieval-50 enterprise-rag-bench-cortexdb-retrieval-existing-50-candidates-wide
	$(MAKE) enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50

enterprise-rag-bench-routed-v10-project-chain-retrieval-existing-50:
	test -f "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50)"
	test -f "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_WIDE)"
	python3 scripts/enterprise_rag_bench/project_chain_rerank.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-retrieval-file "$(ENTERPRISE_RAG_BENCH_ROUTED_V8_SELECTIVE_LEXICAL_50)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_CANDIDATES_WIDE)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROUTED_V10_PROJECT_CHAIN_50_REPORT)"

enterprise-rag-bench-official-retrieval-only-metrics-embedding-rerank-existing-50: enterprise-rag-bench-official-env enterprise-rag-bench-embedding-rerank-existing-50
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50))" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_EMBEDDING_RERANK_50_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_ROOT)/retrieval/questions_updated_embedding_rerank.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

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

enterprise-rag-bench-official-answer-metrics-50: enterprise-rag-bench-official-env enterprise-rag-bench-deepseek-answers-50
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_ANSWER_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_ANSWER_50_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_ANSWER_50_ROOT)/questions_updated.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

enterprise-rag-bench-official-answer-metrics-embedding-rerank-50: enterprise-rag-bench-official-env enterprise-rag-bench-deepseek-answers-embedding-rerank-50
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/questions_updated.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-smoke:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_SMOKE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_SMOKE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_JUDGE_SMOKE_LIMIT)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_SMOKE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-v2-judge-50: enterprise-rag-bench-deepseek-answers-embedding-rerank-v2-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-v3-windowed-judge-50: enterprise-rag-bench-deepseek-answers-embedding-rerank-v3-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v4-windowed-judge-50: enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v4-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v5-windowed-judge-50: enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v5-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-embedding-rerank-fused-v6-lexical-windowed-judge-50: enterprise-rag-bench-deepseek-answers-embedding-rerank-fused-v6-lexical-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v8-selective-lexical-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v8-selective-lexical-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v9-type-aware-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v9-type-aware-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v10-project-chain-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v10-project-chain-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v11-evidence-audit-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v11-evidence-audit-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v12-type-aware-digest-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v12-type-aware-digest-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-official-answer-metrics-routed-v13-source-truth-digest-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v13-source-truth-digest-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-routed-v14-completeness-source-truth-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_METRICS)"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/combine_routed_answer_outputs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl" \
	  --default-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_METRICS)" \
	  --routed-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl" \
	  --routed-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_METRICS)" \
	  --output-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_ROOT)/answers.jsonl" \
	  --output-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_METRICS)" \
	  --output-report-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_REPORT)" \
	  --policy-name v14_completeness_source_truth \
	  --routed-question-types "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_ROUTE_TYPES)"

enterprise-rag-bench-official-answer-metrics-routed-v15-coverage-ranked-windowed-judge-50: enterprise-rag-bench-deepseek-answers-routed-v15-coverage-ranked-windowed-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"

enterprise-rag-bench-routed-v16-conflict-coverage-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_METRICS)"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/combine_routed_answer_outputs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_ROOT)/answers.jsonl" \
	  --default-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_METRICS)" \
	  --routed-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl" \
	  --routed-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_METRICS)" \
	  --output-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl" \
	  --output-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_METRICS)" \
	  --output-report-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_REPORT)" \
	  --policy-name v16_conflict_coverage \
	  --routed-question-types "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_ROUTE_TYPES)"

enterprise-rag-bench-score-summary-routed-v16-50: enterprise-rag-bench-routed-v16-conflict-coverage-judge-50
	python3 scripts/enterprise_rag_bench/summarize_score.py \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_METRICS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_SCORE_SUMMARY)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_SCORE_MARKDOWN)" \
	  --run-label "routed-v16-conflict-coverage-50"

enterprise-rag-bench-token-tracked-judge-routed-v16-50: enterprise-rag-bench-routed-v16-conflict-coverage-judge-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_TOKEN_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_TOKEN_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)"
	python3 scripts/enterprise_rag_bench/summarize_score.py \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_TOKEN_JUDGE_METRICS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_TOKEN_SCORE_SUMMARY)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_TOKEN_SCORE_MARKDOWN)" \
	  --run-label "routed-v16-conflict-coverage-50-token-tracked-judge"

enterprise-rag-bench-official-answer-metrics-routed-v17-evidence-first-judge-50: enterprise-rag-bench-deepseek-answers-routed-v17-evidence-first-50
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)/answers.jsonl"
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_JUDGE_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_JUDGE_STATUS)"

enterprise-rag-bench-score-summary-routed-v17-50: enterprise-rag-bench-official-answer-metrics-routed-v17-evidence-first-judge-50
	python3 scripts/enterprise_rag_bench/summarize_score.py \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_JUDGE_METRICS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)/answers.jsonl" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_SCORE_SUMMARY)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_SCORE_MARKDOWN)" \
	  --run-label "routed-v17-evidence-first-50"

enterprise-rag-bench-token-tracked-judge-routed-v17-50: enterprise-rag-bench-official-answer-metrics-routed-v17-evidence-first-judge-50
	python3 scripts/enterprise_rag_bench/run_deepseek_answer_metrics.py \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)/answers.jsonl)" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_JUDGE_METRICS))" \
	  --judgments-file "$(abspath $(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_JUDGE_ROWS))" \
	  --api-key-file "$(ENTERPRISE_RAG_BENCH_JUDGE_API_KEY_FILE)" \
	  --base-url "$(ENTERPRISE_RAG_BENCH_JUDGE_BASE_URL)" \
	  --model "$(ENTERPRISE_RAG_BENCH_JUDGE_MODEL)" \
	  --timeout-seconds "$(ENTERPRISE_RAG_BENCH_JUDGE_TIMEOUT_SECONDS)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_JUDGE_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_JUDGE_STATUS)"
	python3 scripts/enterprise_rag_bench/summarize_score.py \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_JUDGE_METRICS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_ROOT)/answers.jsonl" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_SCORE_SUMMARY)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V17_50_TOKEN_SCORE_MARKDOWN)" \
	  --run-label "routed-v17-evidence-first-50-token-tracked-judge"

enterprise-rag-bench-calibration-50: enterprise-rag-bench-score-summary-routed-v16-50

enterprise-rag-bench-calibration-100-prep: enterprise-rag-bench-balanced-100

enterprise-rag-bench-routed-v7-selective-lexical-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/combine_routed_answer_outputs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl" \
	  --default-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)" \
	  --routed-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl" \
	  --routed-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)" \
	  --output-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_ROOT)/answers.jsonl" \
	  --output-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_METRICS)" \
	  --output-report-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_REPORT)" \
	  --routed-question-types "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_ROUTE_TYPES)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-50: enterprise-rag-bench-official-answer-metrics-embedding-rerank-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-v2-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-v3-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v4-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v5-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v6-lexical-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v7-selective-lexical-judge-50: enterprise-rag-bench-routed-v7-selective-lexical-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v8-selective-lexical-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v8-selective-lexical-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v9-type-aware-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v9-type-aware-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v10-project-chain-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v10-project-chain-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v11-evidence-audit-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v11-evidence-audit-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v12-type-aware-digest-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v12-type-aware-digest-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v13-source-truth-digest-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v13-source-truth-digest-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v14-completeness-source-truth-judge-50: enterprise-rag-bench-routed-v14-completeness-source-truth-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v15-coverage-ranked-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v15-coverage-ranked-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v16-conflict-coverage-judge-50: enterprise-rag-bench-routed-v16-conflict-coverage-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_ANALYSIS)"

