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

