locomo-official-data:
	python3 scripts/locomo/download.py \
	  --data-root "$(LOCOMO_DATA_ROOT)" \
	  --manifest "$(LOCOMO_DATA_MANIFEST)"

locomo-cortexdb-retrieval: locomo-official-data
	cargo build --release -p cortex-engine --bin locomo_retrieval
	@limit_args=""; \
	if [ -n "$(LOCOMO_MAX_QUESTIONS)" ]; then limit_args="--max-questions $(LOCOMO_MAX_QUESTIONS)"; fi; \
	./target/release/locomo_retrieval \
	  --data-file "$(LOCOMO_DATA_FILE)" \
	  --db-root "$(LOCOMO_DB_ROOT)" \
	  --output "$(LOCOMO_RETRIEVAL_OUTPUT)" \
	  --report "$(LOCOMO_RETRIEVAL_REPORT)" \
	  --top-k "$(LOCOMO_TOPK)" \
	  --reset-db \
	  $$limit_args

locomo-retrieval-adapter-check: locomo-cortexdb-retrieval
	python3 scripts/locomo/check_retrieval_adapter.py \
	  --data-manifest "$(LOCOMO_DATA_MANIFEST)" \
	  --retrieval-report "$(LOCOMO_RETRIEVAL_REPORT)" \
	  --retrieval-output "$(LOCOMO_RETRIEVAL_OUTPUT)" \
	  --output "$(LOCOMO_ADAPTER_REPORT)"

multihop-rag-official-repo:
	@if [ ! -d "$(MULTIHOP_RAG_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/yixuantt/MultiHop-RAG "$(MULTIHOP_RAG_OFFICIAL_REPO)"; \
	else \
	  git -C "$(MULTIHOP_RAG_OFFICIAL_REPO)" pull --ff-only; \
	fi

multihop-rag-official-data:
	python3 scripts/multihop_rag/download.py \
	  --data-root "$(MULTIHOP_RAG_DATA_ROOT)" \
	  --manifest "$(MULTIHOP_RAG_DATA_MANIFEST)"

multihop-rag-preflight: multihop-rag-official-data
	python3 scripts/multihop_rag/preflight.py \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --report "$(MULTIHOP_RAG_PREFLIGHT_REPORT)"

multihop-rag-balanced-50: multihop-rag-preflight
	python3 scripts/multihop_rag/build_balanced_subset.py \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --limit "$(MULTIHOP_RAG_SUBSET_LIMIT)" \
	  --output-root "$(MULTIHOP_RAG_SUBSET_ROOT)" \
	  --output-prefix "$(MULTIHOP_RAG_SUBSET_PREFIX)"

multihop-rag-local-50-check: multihop-rag-balanced-50
	@echo "MultiHop-RAG local 50-query subset ready under $(MULTIHOP_RAG_SUBSET_ROOT)/$(MULTIHOP_RAG_SUBSET_PREFIX)"

multihop-rag-cortexdb-retrieval-50: multihop-rag-local-50-check
	cargo build --release -p cortex-engine --bin multihop_rag_retrieval
	./target/release/multihop_rag_retrieval \
	  --queries "$(MULTIHOP_RAG_SUBSET_ROOT)/$(MULTIHOP_RAG_SUBSET_PREFIX)/$(MULTIHOP_RAG_SUBSET_PREFIX)_multihop.json" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --db-root "$(MULTIHOP_RAG_DB_50)" \
	  --output "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --report "$(MULTIHOP_RAG_RETRIEVAL_50_REPORT)" \
	  --top-k "$(MULTIHOP_RAG_TOPK)" \
	  --reset-db

multihop-rag-official-retrieval-metrics-50: multihop-rag-official-repo multihop-rag-cortexdb-retrieval-50
	python3 "$(MULTIHOP_RAG_OFFICIAL_REPO)/retrieval_evaluate.py" --file "$(MULTIHOP_RAG_RETRIEVAL_50)" | tee "$(MULTIHOP_RAG_RETRIEVAL_50_METRICS)"

multihop-rag-cortexdb-retrieval-full: multihop-rag-preflight
	cargo build --release -p cortex-engine --bin multihop_rag_retrieval
	./target/release/multihop_rag_retrieval \
	  --queries "$(MULTIHOP_RAG_QUERY_FILE)" \
	  --corpus "$(MULTIHOP_RAG_CORPUS_FILE)" \
	  --db-root "$(MULTIHOP_RAG_DB_FULL)" \
	  --output "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --report "$(MULTIHOP_RAG_RETRIEVAL_FULL_REPORT)" \
	  --top-k "$(MULTIHOP_RAG_TOPK)" \
	  --reset-db

multihop-rag-official-retrieval-metrics-full: multihop-rag-official-repo multihop-rag-cortexdb-retrieval-full
	python3 "$(MULTIHOP_RAG_OFFICIAL_REPO)/retrieval_evaluate.py" --file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" | tee "$(MULTIHOP_RAG_RETRIEVAL_FULL_METRICS)"

multihop-rag-retrieval-full-existing-check:
	@test -f "$(MULTIHOP_RAG_RETRIEVAL_FULL)" || (echo "missing $(MULTIHOP_RAG_RETRIEVAL_FULL); run make multihop-rag-official-retrieval-metrics-full first" && exit 1)

multihop-rag-qa-full-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-full first" && exit 1)

multihop-rag-qa-hybrid-full-retry-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-hybrid-full-retry first" && exit 1)

multihop-rag-qa-hybrid-full-retry-v4-existing-check:
	@test -f "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" || (echo "missing $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json; run make multihop-rag-official-qa-metrics-hybrid-full-retry-v4 first" && exit 1)

multihop-rag-deepseek-qa-50: multihop-rag-official-retrieval-metrics-50
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --output-root "$(MULTIHOP_RAG_QA_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-50-cache-metrics: multihop-rag-official-retrieval-metrics-50
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_50)" \
	  --output-root "$(MULTIHOP_RAG_QA_50_CACHE_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_50_CACHE_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_50_CACHE_METRICS))"

multihop-rag-official-qa-metrics-50: multihop-rag-official-repo multihop-rag-deepseek-qa-50
	$(MAKE) multihop-rag-official-qa-metrics-existing-50

multihop-rag-official-qa-metrics-existing-50: multihop-rag-official-repo
	test -f "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_50_METRICS))"

multihop-rag-qa-error-analysis-50:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_50_ANALYSIS)" \
	  --output-md "$(MULTIHOP_RAG_QA_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-full: multihop-rag-official-retrieval-metrics-full
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_FULL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style "$(MULTIHOP_RAG_QA_PROMPT_STYLE)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-temporal-50-v3: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-official-qa-metrics-temporal-50-v3: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v3
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_METRICS))"

multihop-rag-qa-error-analysis-temporal-50-v3:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-50-v3-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-abstention-retry

multihop-rag-official-qa-metrics-temporal-50-v3-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v3-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_METRICS))"

multihop-rag-qa-error-analysis-temporal-50-v3-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-50-v4-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-decomposition-retry

multihop-rag-official-qa-metrics-temporal-50-v4-decompose-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-50-v4-decompose-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-temporal-50-v4-decompose-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-chronology-50-v1: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --temporal-subtype chronology \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_CHRONOLOGY_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-chronology-retry

multihop-rag-official-qa-metrics-temporal-chronology-50-v1: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-chronology-50-v1
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-temporal-chronology-50-v1:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-chronology-yes-no-50-v1: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --temporal-subtype chronology \
	  --temporal-answer-form yes_no \
	  --max-queries "$(MULTIHOP_RAG_TEMPORAL_CHRONOLOGY_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-chronology-retry

multihop-rag-official-qa-metrics-temporal-chronology-yes-no-50-v1: multihop-rag-official-repo multihop-rag-deepseek-qa-temporal-chronology-yes-no-50-v1
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-temporal-chronology-yes-no-50-v1:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-comparison-50-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --max-queries "$(MULTIHOP_RAG_COMPARISON_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry

multihop-rag-official-qa-metrics-comparison-50-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-comparison-50-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-comparison-50-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_COMPARISON_50_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-comparison-50-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --max-queries "$(MULTIHOP_RAG_COMPARISON_GATE_LIMIT)" \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry \
	  --comparison-retry-style decompose

multihop-rag-official-qa-metrics-comparison-50-decompose-retry: multihop-rag-official-repo multihop-rag-deepseek-qa-comparison-50-decompose-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/official_qa_metrics.txt)"

multihop-rag-qa-error-analysis-comparison-50-decompose-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_COMPARISON_50_DECOMPOSE_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-deepseek-qa-temporal-v3: multihop-rag-official-retrieval-metrics-full
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)"

multihop-rag-deepseek-qa-temporal-v3-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_TEMPORAL_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v3 \
	  --question-type temporal_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --temporal-abstention-retry

multihop-rag-deepseek-qa-comparison-v2-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry

multihop-rag-deepseek-qa-comparison-v3-decompose-retry: multihop-rag-retrieval-full-existing-check
	python3 scripts/multihop_rag/run_deepseek_qa.py \
	  --retrieval-file "$(MULTIHOP_RAG_RETRIEVAL_FULL)" \
	  --output-root "$(MULTIHOP_RAG_QA_COMPARISON_DECOMPOSE_RETRY_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --base-url "$(MULTIHOP_RAG_QA_BASE_URL)" \
	  --model "$(MULTIHOP_RAG_QA_MODEL)" \
	  --top-k-context "$(MULTIHOP_RAG_COMPARISON_QA_TOPK_CONTEXT)" \
	  --max-chars-per-doc "$(MULTIHOP_RAG_QA_MAX_CHARS_PER_DOC)" \
	  --prompt-style multihop-v2 \
	  --question-type comparison_query \
	  --workers "$(MULTIHOP_RAG_QA_WORKERS)" \
	  --comparison-retry \
	  --comparison-retry-style decompose

multihop-rag-combine-qa-full-hybrid: multihop-rag-deepseek-qa-full multihop-rag-deepseek-qa-temporal-v3
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_TEMPORAL_ROOT)/deepseek_qa.json" \
	  --question-type temporal_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa_report.json"

multihop-rag-combine-qa-full-hybrid-retry: multihop-rag-qa-full-existing-check multihop-rag-deepseek-qa-temporal-v3-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_TEMPORAL_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type temporal_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa_report.json"

multihop-rag-combine-qa-full-hybrid-retry-v4: multihop-rag-qa-hybrid-full-retry-existing-check multihop-rag-deepseek-qa-comparison-v2-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_COMPARISON_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type comparison_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa_report.json"

multihop-rag-postprocess-hybrid-full-retry-v5: multihop-rag-qa-hybrid-full-retry-v4-existing-check
	python3 scripts/multihop_rag/postprocess_qa_answers.py \
	  --input "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa_report.json" \
	  --temporal-answer-normalize

multihop-rag-combine-qa-full-hybrid-retry-v6: multihop-rag-postprocess-hybrid-full-retry-v5 multihop-rag-deepseek-qa-comparison-v3-decompose-retry
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_COMPARISON_DECOMPOSE_RETRY_ROOT)/deepseek_qa.json" \
	  --question-type comparison_query \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa_report.json"

multihop-rag-combine-qa-full-hybrid-retry-v7: multihop-rag-combine-qa-full-hybrid-retry-v6 multihop-rag-deepseek-qa-temporal-chronology-yes-no-50-v1
	python3 scripts/multihop_rag/combine_qa_by_type.py \
	  --base-qa "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --replacement-qa "$(MULTIHOP_RAG_QA_TEMPORAL_CHRONOLOGY_YES_NO_50_ROOT)/deepseek_qa.json" \
	  --question-type temporal_query \
	  --temporal-subtype chronology \
	  --temporal-answer-form yes_no \
	  --output "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/deepseek_qa.json" \
	  --report "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/deepseek_qa_report.json"

multihop-rag-official-qa-metrics-hybrid-full: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v4: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry-v4
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v5: multihop-rag-official-repo multihop-rag-postprocess-hybrid-full-retry-v5
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v6: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry-v6
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_METRICS))"

multihop-rag-official-qa-metrics-hybrid-full-retry-v7: multihop-rag-official-repo multihop-rag-combine-qa-full-hybrid-retry-v7
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/official_qa_metrics.txt)"

multihop-rag-official-qa-metrics-full: multihop-rag-official-repo multihop-rag-deepseek-qa-full
	$(MAKE) multihop-rag-official-qa-metrics-existing-full

multihop-rag-official-qa-metrics-existing-full: multihop-rag-official-repo
	test -f "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json"
	mkdir -p "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output"
	cp "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" "$(MULTIHOP_RAG_OFFICIAL_REPO)/qa_output/llama.json"
	cd "$(MULTIHOP_RAG_OFFICIAL_REPO)" && PYTHONPATH="$(abspath scripts/multihop_rag)" python3 qa_evaluate.py | tee "$(abspath $(MULTIHOP_RAG_QA_FULL_METRICS))"

multihop-rag-qa-error-analysis-full:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_FULL_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_FULL_ANALYSIS)" \
	  --output-md "$(MULTIHOP_RAG_QA_FULL_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v4:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V4_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v5:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V5_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v6:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/qa_error_analysis.md"

multihop-rag-qa-error-analysis-hybrid-full-retry-v7:
	python3 scripts/multihop_rag/analyze_qa_errors.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/qa_error_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V7_ROOT)/qa_error_analysis.md"

multihop-rag-temporal-subtype-analysis-v6:
	python3 scripts/multihop_rag/analyze_temporal_subtypes.py \
	  --qa-file "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/deepseek_qa.json" \
	  --output-json "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/temporal_subtype_analysis.json" \
	  --output-md "$(MULTIHOP_RAG_QA_HYBRID_FULL_RETRY_V6_ROOT)/temporal_subtype_analysis.md"

