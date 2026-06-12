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

