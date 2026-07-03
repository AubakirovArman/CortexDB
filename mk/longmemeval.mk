longmemeval-v1-official-repo:
	@if [ ! -d "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/.git" ]; then \
	  git clone --depth 1 https://github.com/xiaowu0162/LongMemEval "$(LONGMEMEVAL_V1_OFFICIAL_REPO)"; \
	else \
	  git -C "$(LONGMEMEVAL_V1_OFFICIAL_REPO)" pull --ff-only; \
	fi

longmemeval-v1-official-lite-env: longmemeval-v1-official-repo
	@if [ ! -x "$(LONGMEMEVAL_V1_PYTHON)" ]; then python3 -m venv "$(LONGMEMEVAL_V1_VENV)"; fi
	"$(LONGMEMEVAL_V1_PYTHON)" -m pip install -r "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/requirements-lite.txt"

longmemeval-v1-official-data:
	python3 scripts/longmemeval/download_v1.py --data-root "$(LONGMEMEVAL_V1_DATA_ROOT)" --split "$(LONGMEMEVAL_V1_DATA_SPLIT)" --manifest "$(LONGMEMEVAL_V1_DATA_MANIFEST)"

longmemeval-v1-cortexdb-retrieval: longmemeval-v1-official-data
	cargo build --release -p cortex-cli --bin cortexdb
	@limit_args=""; \
	if [ -n "$(LONGMEMEVAL_V1_LIMIT)" ]; then limit_args="--limit $(LONGMEMEVAL_V1_LIMIT)"; fi; \
	python3 scripts/longmemeval/v1_cortexdb_retrieval.py \
	  --data-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --cortexdb-bin ./target/release/cortexdb \
	  --output-dir "$(LONGMEMEVAL_V1_OUTPUT_ROOT)" \
	  --granularity "$(LONGMEMEVAL_V1_GRANULARITY)" \
	  --index-mode "$(LONGMEMEVAL_V1_INDEX_MODE)" \
	  --context-mode "$(LONGMEMEVAL_V1_CONTEXT_MODE)" \
	  --max-turn-chars "$(LONGMEMEVAL_V1_MAX_TURN_CHARS)" \
	  --max-session-chars "$(LONGMEMEVAL_V1_MAX_SESSION_CHARS)" \
	  --top-k "$(LONGMEMEVAL_V1_TOPK)" \
	  $$limit_args

longmemeval-v1-official-retrieval-metrics: longmemeval-v1-official-lite-env longmemeval-v1-cortexdb-retrieval
	"$(LONGMEMEVAL_V1_PYTHON)" "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/evaluation/print_retrieval_metrics.py" "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" > "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)"
	cat "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)"

longmemeval-v1-retrieval-adapter-check: longmemeval-v1-official-retrieval-metrics
	python3 scripts/longmemeval/check_v1_retrieval_adapter.py \
	  --data-manifest "$(LONGMEMEVAL_V1_DATA_MANIFEST)" \
	  --retrieval-report "$(LONGMEMEVAL_V1_OUTPUT_ROOT)/report.json" \
	  --retrieval-log "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --official-metrics "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)" \
	  --output "$(LONGMEMEVAL_V1_RETRIEVAL_ADAPTER_REPORT)"

longmemeval-v1-e2e-adapter-check:
	python3 scripts/longmemeval/check_v1_e2e_adapter.py \
	  --package-dir "$(LONGMEMEVAL_V1_E2E_PACKAGE_DIR)" \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-adapter-report "$(LONGMEMEVAL_V1_RETRIEVAL_ADAPTER_REPORT)" \
	  --output "$(LONGMEMEVAL_V1_E2E_ADAPTER_REPORT)"

longmemeval-evidence-page-check:
	python3 scripts/longmemeval_evidence_page_check.py

.PHONY: longmemeval-per-type-regression-check
longmemeval-per-type-regression-check:
	python3 scripts/longmemeval/per_type_regression_gate.py --self-test

.PHONY: longmemeval-per-type-report-check
longmemeval-per-type-report-check:
	python3 scripts/longmemeval/per_type_report.py --self-test

# A6.2 (fast, offline): prove the type-aware generation harness — the prompt
# branching (ported verbatim from the DeepSeek diagnostic), the evaluate_qa.py-
# compatible JSONL schema, and byte-determinism — over the committed 5-branch
# fixture, with NO endpoint. The metered GPT-4o run plugs into the same generator
# via longmemeval-v1-typeaware-generate.
.PHONY: longmemeval-v1-typeaware-check
longmemeval-v1-typeaware-check:
	python3 scripts/longmemeval/run_official_generation.py --self-test

# A6.2 (real, metered): type-aware official generation over the retrieval log via
# an OpenAI-compatible endpoint (GPT-4o official). Emits hypotheses for the
# untouched official evaluate_qa.py (score with longmemeval-v1-official-qa-score
# LONGMEMEVAL_V1_HYPOTHESIS_FILE=<the emitted jsonl>).
longmemeval-v1-typeaware-generate: longmemeval-v1-cortexdb-retrieval
	@if [ -z "$(LONGMEMEVAL_V1_READER_API_KEY)" ]; then echo "Set LONGMEMEVAL_V1_READER_API_KEY for type-aware generation" >&2; exit 2; fi
	mkdir -p "$(LONGMEMEVAL_V1_GENERATION_ROOT)"
	python3 scripts/longmemeval/run_official_generation.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --output "$(LONGMEMEVAL_V1_GENERATION_ROOT)/typeaware_hypotheses.jsonl" \
	  --model "$(LONGMEMEVAL_V1_READER_MODEL_NAME)" \
	  $(if $(LONGMEMEVAL_V1_READER_BASE_URL),--base-url "$(LONGMEMEVAL_V1_READER_BASE_URL)",) \
	  --api-key "$(LONGMEMEVAL_V1_READER_API_KEY)"

longmemeval-v1-official-generate: longmemeval-v1-official-repo longmemeval-v1-cortexdb-retrieval
	@if [ -z "$(LONGMEMEVAL_V1_READER_API_KEY)" ]; then echo "Set LONGMEMEVAL_V1_READER_API_KEY or DEEPSEEK_API_KEY for generation" >&2; exit 2; fi
	mkdir -p "$(LONGMEMEVAL_V1_GENERATION_ROOT)"
	@base_url_args=""; \
	if [ -n "$(LONGMEMEVAL_V1_READER_BASE_URL)" ]; then base_url_args="--openai_base_url $(LONGMEMEVAL_V1_READER_BASE_URL)"; fi; \
	python3 "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/generation/run_generation.py" \
	  --in_file "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --out_dir "$(LONGMEMEVAL_V1_GENERATION_ROOT)" \
	  --model_name "$(LONGMEMEVAL_V1_READER_MODEL_NAME)" \
	  --model_alias "$(LONGMEMEVAL_V1_READER_MODEL_ALIAS)" \
	  $$base_url_args \
	  --openai_key "$(LONGMEMEVAL_V1_READER_API_KEY)" \
	  --retriever_type flat-session \
	  --topk_context "$(LONGMEMEVAL_V1_GENERATION_TOPK)" \
	  --history_format json \
	  --useronly false \
	  --cot false \
	  --con false

longmemeval-v1-official-qa-score: longmemeval-v1-official-lite-env longmemeval-v1-official-data
	@if [ -z "$(LONGMEMEVAL_V1_EVAL_API_KEY)" ]; then echo "Set LONGMEMEVAL_V1_EVAL_API_KEY or DEEPSEEK_API_KEY for LongMemEval evaluator" >&2; exit 2; fi
	@if [ -z "$(LONGMEMEVAL_V1_HYPOTHESIS_FILE)" ]; then echo "Set LONGMEMEVAL_V1_HYPOTHESIS_FILE to the official generation output jsonl" >&2; exit 2; fi
	cd "$(LONGMEMEVAL_V1_OFFICIAL_REPO)/src/evaluation" && \
	  OPENAI_API_KEY="$(LONGMEMEVAL_V1_EVAL_API_KEY)" "$(abspath $(LONGMEMEVAL_V1_PYTHON))" evaluate_qa.py "$(LONGMEMEVAL_V1_EVAL_MODEL)" "$(abspath $(LONGMEMEVAL_V1_HYPOTHESIS_FILE))" "$(abspath $(LONGMEMEVAL_V1_DATA_FILE))"

longmemeval-v1-official-score: longmemeval-v1-official-generate
	@echo "Generation completed. Re-run make longmemeval-v1-official-qa-score LONGMEMEVAL_V1_HYPOTHESIS_FILE=<generated-file>"

longmemeval-v1-package-submission:
	python3 scripts/longmemeval/package_v1_submission.py \
	  --package-name "$(LONGMEMEVAL_V1_PACKAGE_NAME)" \
	  --output-root "$(LONGMEMEVAL_V1_SUBMISSION_ROOT)" \
	  --force

longmemeval-v1-error-analysis:
	python3 scripts/longmemeval/analyze_v1_results.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_RETRIEVAL_LOG)" \
	  --official-metrics "$(LONGMEMEVAL_V1_OFFICIAL_METRICS_REPORT)" \
	  --generation-dir "$(LONGMEMEVAL_V1_GENERATION_ROOT)" \
	  --output-root "$(LONGMEMEVAL_V1_ANALYSIS_ROOT)"

longmemeval-v1-deepseek-flash-falsecase-check:
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/compact_context_false_cases_retrieval.jsonl" \
	  --reference-file "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/false_cases_reference.json" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking "$(LONGMEMEVAL_V1_DEEPSEEK_GENERATION_THINKING)" \
	  --judge-thinking "$(LONGMEMEVAL_V1_DEEPSEEK_JUDGE_THINKING)"

longmemeval-v1-deepseek-flash-diff:
	python3 scripts/longmemeval/compare_deepseek_flash_runs.py \
	  --old-root "$(LONGMEMEVAL_V1_DEEPSEEK_FLASH_IMPLICIT_ROOT)" \
	  --new-root "$(LONGMEMEVAL_V1_DEEPSEEK_ROOT)" \
	  --reference-file "$(LONGMEMEVAL_V1_FALSECASE_ROOT)/false_cases_reference.json" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_FLASH_DIFF_ROOT)"

longmemeval-v1-deepseek-flash-compact-50-check:
	python3 scripts/longmemeval/build_balanced_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --limit "$(LONGMEMEVAL_V1_COMPACT_50_LIMIT)" \
	  --output-root "$(LONGMEMEVAL_V1_COMPACT_50_INPUT_ROOT)" \
	  --output-prefix compact_50
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_50_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_COMPACT_50_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_COMPACT_50_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-compact-500-check:
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --output-root "$(LONGMEMEVAL_V1_DEEPSEEK_COMPACT_500_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-preference-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type single-session-preference \
	  --output-root "$(LONGMEMEVAL_V1_PREFERENCE_INPUT_ROOT)" \
	  --output-prefix preference
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_PREFERENCE_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_PREFERENCE_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_PREFERENCE_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-single-session-user-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type single-session-user \
	  --output-root "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_INPUT_ROOT)" \
	  --output-prefix single_session_user
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_SINGLE_SESSION_USER_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-multi-session-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type multi-session \
	  --output-root "$(LONGMEMEVAL_V1_MULTI_SESSION_INPUT_ROOT)" \
	  --output-prefix multi_session
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_MULTI_SESSION_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_MULTI_SESSION_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_MULTI_SESSION_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

longmemeval-v1-deepseek-flash-temporal-check:
	python3 scripts/longmemeval/build_question_type_subset.py \
	  --reference-file "$(LONGMEMEVAL_V1_DATA_FILE)" \
	  --retrieval-log "$(LONGMEMEVAL_V1_COMPACT_RETRIEVAL_LOG)" \
	  --question-type temporal-reasoning \
	  --output-root "$(LONGMEMEVAL_V1_TEMPORAL_INPUT_ROOT)" \
	  --output-prefix temporal
	python3 scripts/longmemeval/run_deepseek_flash_subset.py \
	  --retrieval-log "$(LONGMEMEVAL_V1_TEMPORAL_RETRIEVAL)" \
	  --reference-file "$(LONGMEMEVAL_V1_TEMPORAL_REFERENCE)" \
	  --output-root "$(LONGMEMEVAL_V1_TEMPORAL_ROOT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LONGMEMEVAL_V1_DEEPSEEK_MODEL)" \
	  --generation-thinking disabled \
	  --judge-thinking disabled

