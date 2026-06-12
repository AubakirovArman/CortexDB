enterprise-rag-bench-cortexdb-retrieval-smoke: enterprise-rag-bench-balanced-50
	cargo build -p cortex-engine --bin enterprise_rag_bench_retrieval
	./target/debug/enterprise_rag_bench_retrieval \
	  --questions "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --db-root "$(ENTERPRISE_RAG_BENCH_DB_SMOKE)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_SMOKE)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_SMOKE_REPORT)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_TOPK)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_INGEST_BATCH_SIZE)" \
	  --max-documents "$(ENTERPRISE_RAG_BENCH_SMOKE_MAX_DOCUMENTS)" \
	  --reset-db \
	  --progress-every 250

enterprise-rag-bench-official-retrieval-only-metrics-smoke: enterprise-rag-bench-official-env enterprise-rag-bench-cortexdb-retrieval-smoke
	cd "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" && "$(abspath $(ENTERPRISE_RAG_BENCH_PYTHON))" -m src.scripts.answer_evaluation.metrics_based_eval \
	  --answers-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_SMOKE))" \
	  --questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS))" \
	  --results-file "$(abspath $(ENTERPRISE_RAG_BENCH_RETRIEVAL_SMOKE_METRICS))" \
	  --updated-questions-file "$(abspath $(ENTERPRISE_RAG_BENCH_ROOT)/smoke/questions_updated.jsonl)" \
	  --uuid-index-cache-file "generated_data/uuid_index.json" \
	  --no-correction \
	  --skip-citation-stripping

enterprise-rag-bench-cortexdb-retrieval-50: enterprise-rag-bench-balanced-50
	cargo build --release -p cortex-engine --bin enterprise_rag_bench_retrieval
	./target/release/enterprise_rag_bench_retrieval \
	  --questions "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --db-root "$(ENTERPRISE_RAG_BENCH_DB_50)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_50_REPORT)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_TOPK)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_INGEST_BATCH_SIZE)" \
	  --reset-db

enterprise-rag-bench-cortexdb-retrieval-full: enterprise-rag-bench-preflight
	cargo build --release -p cortex-engine --bin enterprise_rag_bench_retrieval
	./target/release/enterprise_rag_bench_retrieval \
	  --questions "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --db-root "$(ENTERPRISE_RAG_BENCH_DB_FULL)" \
	  --output "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_FULL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_FULL_REPORT)" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_TOPK)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_INGEST_BATCH_SIZE)" \
	  --reset-db

enterprise-rag-bench-official-clean-vectors-50: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/build_official_clean_vectors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-query-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS_50)" \
	  --output-document-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS_50)" \
	  --report "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_REPORT)" \
	  --cache-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_CACHE)" \
	  --env-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_ENV_FILE)" \
	  --limit-questions 50 \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT),--limit-documents "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT)",) \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_MAX_CHARS_PER_DOC)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_BATCH_SIZE)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_PROGRESS_EVERY)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_STATUS)"

enterprise-rag-bench-official-clean-vectors-500: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/build_official_clean_vectors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --output-query-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS_500)" \
	  --output-document-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS_500)" \
	  --report "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_REPORT)" \
	  --cache-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_CACHE)" \
	  --env-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_ENV_FILE)" \
	  --limit-questions 500 \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT),--limit-documents "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_DOCUMENT_LIMIT)",) \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_MAX_CHARS_PER_DOC)" \
	  --batch-size "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_BATCH_SIZE)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_PROGRESS_EVERY)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_VECTOR_STATUS)"

enterprise-rag-bench-embedding-coverage-check: enterprise-rag-bench-official-repo
	cargo run -p cortex-engine --bin embedding_coverage_check -- \
	  $(if $(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_MANIFEST),--expected-manifest "$(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_MANIFEST)",--uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)") \
	  --embeddings "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS_500)" \
	  --output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_COVERAGE_REPORT)" \
	  --retry-ids-output "$(ENTERPRISE_RAG_BENCH_EMBEDDING_RETRY_IDS)" \
	  --min-coverage-bps "$(ENTERPRISE_RAG_BENCH_EMBEDDING_COVERAGE_MIN_BPS)" \
	  $(if $(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_MODEL),--expected-model "$(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_MODEL)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_DIMENSION),--expected-dimension "$(ENTERPRISE_RAG_BENCH_EMBEDDING_EXPECTED_DIMENSION)",)

enterprise-rag-bench-official-clean-compare-retrieval:
	python3 scripts/enterprise_rag_bench/compare_retrieval_runs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --baseline-retrieval-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_BASELINE_RETRIEVAL)" \
	  --candidate-retrieval-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CANDIDATE_RETRIEVAL)" \
	  --output-jsonl "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_COMPARISON_JSONL)" \
	  --report "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_COMPARISON_REPORT)" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_COMPARISON_MD)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_TOPK)"

enterprise-rag-bench-official-clean-retrieval-smoke-50:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=smoke-maxdocs-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=cached-lexical \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS=50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY=10

enterprise-rag-bench-official-clean-retrieval-50-cached:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=cached-lexical \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=cached-lexical \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK=none

enterprise-rag-bench-official-clean-retrieval-50-engine-aql:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-aql \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-aql \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK=weighted

enterprise-rag-bench-official-clean-retrieval-50-engine-keyword:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-keyword \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-keyword

enterprise-rag-bench-official-clean-retrieval-50-engine-keyword-rerank:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-keyword-rerank \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-keyword \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK=weighted

enterprise-rag-bench-official-clean-retrieval-50-engine-hybrid:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-hybrid \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-hybrid \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS="$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS_50)" \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS="$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS_50)"

enterprise-rag-bench-official-clean-retrieval-50-engine-hybrid-rerank:
	$(MAKE) enterprise-rag-bench-official-clean-50 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=retrieval \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL=engine-hybrid-rerank \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE=engine-hybrid-rerank \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK=weighted \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS="$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS_50)" \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS="$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS_50)"
