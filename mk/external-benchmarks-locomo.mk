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
