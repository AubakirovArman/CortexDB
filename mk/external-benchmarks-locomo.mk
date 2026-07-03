locomo-official-data:
	python3 scripts/locomo/download.py \
	  --data-root "$(LOCOMO_DATA_ROOT)" \
	  --manifest "$(LOCOMO_DATA_MANIFEST)"

locomo-cortexdb-retrieval: locomo-official-data
	cargo build --release -p cortex-bench --bin locomo_retrieval
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

.PHONY: locomo-retrieval-regression-check
locomo-retrieval-regression-check:
	python3 scripts/locomo/retrieval_regression_gate.py --self-test

# F3.4 (QA reader half, fast/offline): prove the category-aware LoCoMo answer
# generator — the multi-hop / temporal / open-domain / adversarial prompt
# branching (adversarial carries exact-abstention), the scorer-field JSONL schema,
# and byte-determinism — over the committed 4-category fixture with NO endpoint.
# The metered reader run plugs into the same generator via locomo-qa / locomo-qa-50.
.PHONY: locomo-qa-reader-check
locomo-qa-reader-check:
	python3 scripts/locomo/run_qa.py --self-test

# F3.4 (QA reader half, real/metered): generate answers from the retrieved-with-text
# log via an OpenAI-compatible reader (DeepSeek default; key via DEEPSEEK_KEY_FILE).
# Repo rule: run the 50-question subset first, then the full set.
LOCOMO_QA_INPUT_LOG ?= $(LOCOMO_QA_INPUT_LOG_DEFAULT)
LOCOMO_QA_OUTPUT ?= target/locomo/qa/hypotheses.jsonl
locomo-qa-50:
	python3 scripts/locomo/run_qa.py \
	  --input-log "$(LOCOMO_QA_INPUT_LOG)" \
	  --output "$(LOCOMO_QA_OUTPUT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LOCOMO_QA_READER_MODEL)" \
	  --limit 50
locomo-qa:
	python3 scripts/locomo/run_qa.py \
	  --input-log "$(LOCOMO_QA_INPUT_LOG)" \
	  --output "$(LOCOMO_QA_OUTPUT)" \
	  --api-key-file "$(DEEPSEEK_KEY_FILE)" \
	  --model "$(LOCOMO_QA_READER_MODEL)"
