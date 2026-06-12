enterprise-rag-bench-official-repo:
	@if [ ! -d "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/onyx-dot-app/EnterpriseRAG-Bench "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)"; \
	else \
	  git -C "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" pull --ff-only; \
	fi

enterprise-rag-bench-official-env: enterprise-rag-bench-official-repo $(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp

$(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp: $(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/requirements.txt
	python3 -m venv "$(ENTERPRISE_RAG_BENCH_VENV)"
	"$(ENTERPRISE_RAG_BENCH_PYTHON)" -m pip install --upgrade pip
	"$(ENTERPRISE_RAG_BENCH_PYTHON)" -m pip install -r "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)/requirements.txt"
	touch "$(ENTERPRISE_RAG_BENCH_VENV)/.requirements.stamp"

enterprise-rag-bench-preflight: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/preflight.py \
	  --bench-root "$(ENTERPRISE_RAG_BENCH_OFFICIAL_REPO)" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --uuid-index "$(ENTERPRISE_RAG_BENCH_UUID_INDEX)" \
	  --sources-dir "$(ENTERPRISE_RAG_BENCH_SOURCES_DIR)" \
	  --report "$(ENTERPRISE_RAG_BENCH_PREFLIGHT_REPORT)"

enterprise-rag-bench-balanced-50: enterprise-rag-bench-preflight
	python3 scripts/enterprise_rag_bench/build_balanced_subset.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --limit "$(ENTERPRISE_RAG_BENCH_SUBSET_LIMIT)" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_SUBSET_ROOT)" \
	  --output-prefix "$(ENTERPRISE_RAG_BENCH_SUBSET_PREFIX)"

enterprise-rag-bench-balanced-100: enterprise-rag-bench-preflight
	python3 scripts/enterprise_rag_bench/build_balanced_subset.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_QUESTIONS)" \
	  --limit "100" \
	  --output-root "$(ENTERPRISE_RAG_BENCH_SUBSET_ROOT)" \
	  --output-prefix "$(ENTERPRISE_RAG_BENCH_SUBSET_100_PREFIX)"

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

enterprise-rag-bench-official-clean-50: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
	  --size 50 \
	  --split-name primary \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL),--run-label "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL)",) \
	  --stage "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE)" \
	  --answer-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER)" \
	  --judge-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  --retrieval-progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY)" \
	  --retrieval-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE)" \
	  --rerank "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK)" \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS),--max-documents "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS),--query-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS),--document-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL),--prefilter-retrieval "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT),--db-root "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SKIP_CHECKPOINT)),--skip-checkpoint,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB)),--reuse-db,)

enterprise-rag-bench-official-clean-500: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
	  --size 500 \
	  --split-name primary \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL),--run-label "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL)",) \
	  --stage "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE)" \
	  --answer-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER)" \
	  --judge-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  --retrieval-progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY)" \
	  --retrieval-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE)" \
	  --rerank "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK)" \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS),--max-documents "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS),--query-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS),--document-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL),--prefilter-retrieval "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT),--db-root "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SKIP_CHECKPOINT)),--skip-checkpoint,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB)),--reuse-db,)

enterprise-rag-bench-official-clean-heldout: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
	  --size 100 \
	  --split-name heldout \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL),--run-label "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL)",) \
	  --stage "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE)" \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_EXTRA_QUESTIONS)" \
	  --answer-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER)" \
	  --judge-provider "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER)" \
	  --top-k-context "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_CHARS_PER_DOC)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  --retrieval-progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY)" \
	  --retrieval-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_MODE)" \
	  --rerank "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK)" \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS),--max-documents "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_DOCUMENTS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS),--query-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_QUERY_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS),--document-vectors "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DOCUMENT_VECTORS)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL),--prefilter-retrieval "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PREFILTER_RETRIEVAL)",) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT),--db-root "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SKIP_CHECKPOINT)),--skip-checkpoint,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB)),--reuse-db,)

enterprise-rag-bench-official-clean-heldout-smoke-check: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
	  --size 2 \
	  --split-name heldout \
	  --run-label heldout-smoke-check \
	  --stage prepare \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_EXTRA_QUESTIONS)" \
	  --answer-provider deepseek \
	  --judge-provider deepseek
	python3 scripts/enterprise_rag_bench/official_clean_gate.py \
	  --run-report "$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/2/heldout-smoke-check/answer-deepseek/official_clean_run_report.json" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/2/heldout-smoke-check/official_clean_gate_report.json" \
	  --expected-split heldout \
	  --expected-questions-file "$(ENTERPRISE_RAG_BENCH_EXTRA_QUESTIONS)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GATE_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GATE_STATUS)"

.PHONY: enterprise-rag-bench-official-clean-oracle-audit
enterprise-rag-bench-official-clean-oracle-audit:
	python3 scripts/enterprise_rag_bench/oracle_usage_audit.py \
	  --report "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ORACLE_AUDIT)"

enterprise-rag-bench-official-clean-heldout-retrieval-quality-check: enterprise-rag-bench-official-repo
	python3 scripts/enterprise_rag_bench/retrieval_quality_gate.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_EXTRA_QUESTIONS)" \
	  --retrieval-file "$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/100/epic17-heldout-retrieval/retrieval.clean.jsonl" \
	  --report "$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate_report.json" \
	  --markdown "$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate_report.md" \
	  --top-k "$(ENTERPRISE_RAG_BENCH_TOPK)" \
	  --min-average-recall-pct 33 \
	  --min-hit-questions 33 \
	  --min-full-recall-questions 33 \
	  --max-average-invalid-extra-docs 9.67 \
	  --min-mrr 0.08 \
	  --min-ndcg 0.12 \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_GATE_PROGRESS_EVERY)" \
	  --log-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_GATE_LOG)" \
	  --status-file "$(ENTERPRISE_RAG_BENCH_RETRIEVAL_QUALITY_GATE_STATUS)"

enterprise-rag-bench-official-clean-status:
	@run_dir="$(ENTERPRISE_RAG_BENCH_ROOT)/official-clean/$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_SIZE)"; \
	if [ -n "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_RUN_LABEL)" ]; then \
	  run_dir="$$run_dir/$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_RUN_LABEL)"; \
	fi; \
	extra=""; \
	if [ "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_WATCH)" = "1" ]; then \
	  extra="--watch --interval-seconds $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_INTERVAL_SECONDS)"; \
	fi; \
	python3 scripts/enterprise_rag_bench/show_official_clean_status.py --run-dir "$$run_dir" --tail-lines "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STATUS_TAIL_LINES)" $$extra

enterprise-rag-bench-official-clean-50-gemma:
	$(MAKE) enterprise-rag-bench-official-clean-50 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemma ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemma

enterprise-rag-bench-official-clean-500-gemma:
	$(MAKE) enterprise-rag-bench-official-clean-500 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemma ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemma

enterprise-rag-bench-official-clean-heldout-gemma:
	$(MAKE) enterprise-rag-bench-official-clean-heldout ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemma ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemma

enterprise-rag-bench-official-clean-50-gemini:
	$(MAKE) enterprise-rag-bench-official-clean-50 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemini ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemini

enterprise-rag-bench-impact-gemini-50:
	$(MAKE) enterprise-rag-bench-official-clean-50-gemini \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RUN_LABEL="$(ENTERPRISE_RAG_BENCH_IMPACT_50_RUN_LABEL)" \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_REUSE_DB=1 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_DB_ROOT="$(ENTERPRISE_RAG_BENCH_IMPACT_50_DB_ROOT)" \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY=5 \
	  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RETRIEVAL_PROGRESS_EVERY=50

enterprise-rag-bench-official-clean-500-gemini:
	$(MAKE) enterprise-rag-bench-official-clean-500 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemini ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemini

enterprise-rag-bench-official-clean-heldout-gemini:
	$(MAKE) enterprise-rag-bench-official-clean-heldout ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=gemini ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=gemini

enterprise-rag-bench-official-clean-50-deepseek:
	$(MAKE) enterprise-rag-bench-official-clean-50 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=deepseek ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=deepseek

enterprise-rag-bench-official-clean-500-deepseek:
	$(MAKE) enterprise-rag-bench-official-clean-500 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=deepseek ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=deepseek

enterprise-rag-bench-official-clean-heldout-deepseek:
	$(MAKE) enterprise-rag-bench-official-clean-heldout ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_PROVIDER=deepseek ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_PROVIDER=deepseek

