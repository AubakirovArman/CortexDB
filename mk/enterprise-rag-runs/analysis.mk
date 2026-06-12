enterprise-rag-bench-calibration-50: enterprise-rag-bench-score-summary-routed-v16-50

enterprise-rag-bench-calibration-100-prep: enterprise-rag-bench-balanced-100

enterprise-rag-bench-routed-v7-selective-lexical-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl"
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/combine_routed_answer_outputs.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --default-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl" \
	  --default-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)" \
	  --routed-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl" \
	  --routed-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)" \
	  --output-answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_ROOT)/answers.jsonl" \
	  --output-metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_METRICS)" \
	  --output-report-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_REPORT)" \
	  --routed-question-types "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_ROUTE_TYPES)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-50: enterprise-rag-bench-official-answer-metrics-embedding-rerank-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-v2-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V2_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-v3-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V3_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v4-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V4_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v5-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V5_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-embedding-rerank-fused-v6-lexical-windowed-judge-50:
	test -f "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)"
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V6_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v7-selective-lexical-judge-50: enterprise-rag-bench-routed-v7-selective-lexical-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V7_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v8-selective-lexical-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v8-selective-lexical-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V8_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v9-type-aware-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v9-type-aware-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V9_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v10-project-chain-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v10-project-chain-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V10_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v11-evidence-audit-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v11-evidence-audit-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V11_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v12-type-aware-digest-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v12-type-aware-digest-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V12_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v13-source-truth-digest-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v13-source-truth-digest-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V13_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v14-completeness-source-truth-judge-50: enterprise-rag-bench-routed-v14-completeness-source-truth-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V14_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v15-coverage-ranked-windowed-judge-50: enterprise-rag-bench-official-answer-metrics-routed-v15-coverage-ranked-windowed-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V15_50_JUDGE_ANALYSIS)"

enterprise-rag-bench-answer-error-analysis-routed-v16-conflict-coverage-judge-50: enterprise-rag-bench-routed-v16-conflict-coverage-judge-50
	python3 scripts/enterprise_rag_bench/analyze_answer_errors.py \
	  --questions-file "$(ENTERPRISE_RAG_BENCH_SUBSET_QUESTIONS)" \
	  --answers-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_ROOT)/answers.jsonl" \
	  --metrics-file "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_METRICS)" \
	  --report "$(ENTERPRISE_RAG_BENCH_RERANK_ANSWER_V16_50_JUDGE_ANALYSIS)"

