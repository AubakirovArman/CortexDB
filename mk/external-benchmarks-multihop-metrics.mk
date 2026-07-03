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

.PHONY: multihop-retrieval-regression-check
multihop-retrieval-regression-check:
	python3 scripts/multihop_rag/retrieval_regression_gate.py --self-test
