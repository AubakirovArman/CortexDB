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
	cargo build --release -p cortex-bench --bin multihop_rag_retrieval
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
	cargo build --release -p cortex-bench --bin multihop_rag_retrieval
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
