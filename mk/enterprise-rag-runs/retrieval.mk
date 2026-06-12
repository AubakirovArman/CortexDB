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

