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
