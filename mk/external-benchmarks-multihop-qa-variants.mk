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
