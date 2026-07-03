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
	  --context-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CONTEXT_MODE)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --prompt-style "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROMPT_STYLE)" \
	  --gemini-thinking-budget "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GEMINI_THINKING_BUDGET)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_TABLE)),--include-evidence-table,) \
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
	  --context-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CONTEXT_MODE)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --prompt-style "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROMPT_STYLE)" \
	  --gemini-thinking-budget "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GEMINI_THINKING_BUDGET)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_TABLE)),--include-evidence-table,) \
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
	  --context-mode "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CONTEXT_MODE)" \
	  --max-tokens "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_MAX_TOKENS)" \
	  --prompt-style "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROMPT_STYLE)" \
	  --gemini-thinking-budget "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GEMINI_THINKING_BUDGET)" \
	  --unsupported-claim-guard "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET)),--enable-text-intent-budget,) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR)),--self-consistency-repair --self-consistency-retries "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_RETRIES)",) \
	  --answer-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ANSWER_WORKERS)" \
	  --judge-workers "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_WORKERS)" \
	  --judge-timeout-seconds "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_JUDGE_TIMEOUT_SECONDS)" \
	  --progress-every "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROGRESS_EVERY)" \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN)),--include-evidence-plan,) \
	  $(if $(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE),--evidence-plan-file "$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE)",) \
	  $(if $(filter 1 true yes,$(ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_TABLE)),--include-evidence-table,) \
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

.PHONY: erb-official-rejudge-ready-check
erb-official-rejudge-ready-check:
	python3 scripts/enterprise_rag_bench/check_official_rejudge.py --self-test
	python3 scripts/enterprise_rag_bench/check_official_rejudge.py --report "target/erb-official-rejudge/readiness.json"
