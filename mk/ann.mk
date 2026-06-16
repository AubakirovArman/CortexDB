ann-fixture-check:
	cargo run --release -p cortex-engine --bin ann_fixture_gate -- --baseline $(ANN_FIXTURE_BASELINE)

ann-fixture-report:
	cargo run --release -p cortex-engine --bin ann_fixture_gate -- --baseline $(ANN_FIXTURE_BASELINE) --output $(ANN_FIXTURE_REPORT)

ann-drift-check:
	cargo run --release -p cortex-engine --bin ann_drift_check -- --baseline $(ANN_DRIFT_BASELINE)

ann-drift-report:
	cargo run --release -p cortex-engine --bin ann_drift_check -- --baseline $(ANN_DRIFT_BASELINE) --output $(ANN_DRIFT_REPORT)

ann-external-check:
	cargo run --release -p cortex-engine --bin ann_external_fixture_check -- --baseline $(ANN_EXTERNAL_BASELINE)

ann-external-report:
	cargo run --release -p cortex-engine --bin ann_external_fixture_check -- --baseline $(ANN_EXTERNAL_BASELINE) --output $(ANN_EXTERNAL_REPORT)

ann-metric-matrix-check:
	cargo run --release -p cortex-engine --bin ann_metric_matrix_check -- --baseline $(ANN_METRIC_MATRIX_BASELINE)

ann-metric-matrix-report:
	cargo run --release -p cortex-engine --bin ann_metric_matrix_check -- --baseline $(ANN_METRIC_MATRIX_BASELINE) --output $(ANN_METRIC_MATRIX_REPORT)

ann-reference-suite-check: ann-fixture-report ann-external-report ann-domain-corpus-report
	python3 scripts/ann/reference_suite_gate.py --suite "$(ANN_REFERENCE_SUITE)" --report "$(ANN_REFERENCE_SUITE_REPORT)"

ann-reference-suite-report: ann-reference-suite-check

ann-corpus-smoke-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH)

ann-corpus-smoke-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH) --output $(ANN_CORPUS_REPORT)

ann-domain-corpus-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535

ann-domain-corpus-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535 --output $(ANN_DOMAIN_REPORT)

ann-recall-probe-check:
	cargo build --release -p cortex-engine --bin ann_corpus_check
	python3 scripts/ann/recall_probe.py --runner ./target/release/ann_corpus_check --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --iterations $(ANN_RECALL_PROBE_ITERATIONS)

ann-recall-probe-report:
	cargo build --release -p cortex-engine --bin ann_corpus_check
	python3 scripts/ann/recall_probe.py --runner ./target/release/ann_corpus_check --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --iterations $(ANN_RECALL_PROBE_ITERATIONS) --output $(ANN_RECALL_PROBE_REPORT)

ann-production-slo-history-check:
	rm -rf "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)"
	cargo build --release -p cortex-engine --bin ann_corpus_check
	@i=1; while [ $$i -le "$(ANN_PRODUCTION_SLO_HISTORY_RUNS)" ]; do \
	  run_id=$$(printf "slo-%02d" $$i); \
	  scripts/ann/run_external_corpus.sh \
	    --vectors "$(ANN_DOMAIN_VECTORS)" \
	    --queries "$(ANN_DOMAIN_QUERIES)" \
	    --ground-truth "$(ANN_DOMAIN_GROUND_TRUTH)" \
	    --metric dot_product \
	    --output-root "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)" \
	    --run-id "$$run_id" \
	    --min-recall-q16 65535 \
	    --min-mean-recall-q16 65535; \
	  i=$$((i + 1)); \
	done
	python3 scripts/ann/history_gate.py \
	  --run-root "$(ANN_PRODUCTION_SLO_HISTORY_ROOT)" \
	  --output "$(ANN_PRODUCTION_SLO_HISTORY_REPORT)" \
	  --fail-on-regression \
	  --min-runs "$(ANN_PRODUCTION_SLO_HISTORY_RUNS)" \
	  --min-corpora 1 \
	  --max-p95-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_P95_TOLERANCE_NANOS)" \
	  --max-p99-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_P99_TOLERANCE_NANOS)" \
	  --max-max-regression-nanos "$(ANN_PRODUCTION_SLO_HISTORY_MAX_TOLERANCE_NANOS)"

ann-demo-domain-corpus-build:
	python3 scripts/ann/build_demo_domain_corpus.py --source-root $(ANN_DEMO_DOMAIN_SOURCE_ROOT) --source-root $(ANN_DEMO_DOMAIN_EXTRA_SOURCE_ROOT) --output-dir $(ANN_DEMO_DOMAIN_OUTPUT_DIR) --dimension $(ANN_DEMO_DOMAIN_DIMENSION) --limit $(ANN_DEMO_DOMAIN_LIMIT) --scale $(ANN_DEMO_DOMAIN_SCALE)

ann-demo-domain-corpus-run: ann-demo-domain-corpus-build
	scripts/ann/run_external_corpus.sh --vectors $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric dot_product --output-root $(ANN_DEMO_DOMAIN_RUN_ROOT) --run-id $(ANN_DEMO_DOMAIN_RUN_ID) --min-recall-q16 65535 --min-mean-recall-q16 65535 --max-neighbors $(ANN_DEMO_DOMAIN_MAX_NEIGHBORS) --ef-search $(ANN_DEMO_DOMAIN_EF_SEARCH) --ef-construction $(ANN_DEMO_DOMAIN_EF_CONSTRUCTION) --layer-count $(ANN_DEMO_DOMAIN_LAYER_COUNT)

ann-demo-domain-publish-baseline: ann-demo-domain-corpus-run
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_DEMO_DOMAIN_RUN_ROOT) --run-id $(ANN_DEMO_DOMAIN_RUN_ID) --baseline-id $(ANN_DEMO_DOMAIN_BASELINE_ID) --output-root $(ANN_DEMO_DOMAIN_BASELINE_ROOT)

ann-demo-domain-package-baseline: ann-demo-domain-publish-baseline
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_DEMO_DOMAIN_BASELINE_BUNDLE) --package-id $(ANN_DEMO_DOMAIN_BASELINE_ID) --output $(ANN_DEMO_DOMAIN_BASELINE_ARCHIVE)

ann-demo-domain-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_DEMO_DOMAIN_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-embedded-domain-corpus-build:
	@if [ -z "$(ANN_EMBEDDED_DOMAIN_SOURCE_ROOT)" ]; then echo "Set ANN_EMBEDDED_DOMAIN_SOURCE_ROOT to a JSONL payload directory with embedded vectors" >&2; exit 2; fi
	@if [ -z "$(ANN_EMBEDDED_DOMAIN_QUERIES)" ]; then echo "Set ANN_EMBEDDED_DOMAIN_QUERIES to a JSONL query file with embedded vectors" >&2; exit 2; fi
	python3 scripts/ann/build_embedded_domain_corpus.py --source-root $(ANN_EMBEDDED_DOMAIN_SOURCE_ROOT) --queries $(ANN_EMBEDDED_DOMAIN_QUERIES) --output-dir $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR) --limit $(ANN_EMBEDDED_DOMAIN_LIMIT)

ann-embedded-domain-corpus-run: ann-embedded-domain-corpus-build
	@slo_args="$$(python3 scripts/ann/slo_profile.py --profile $(ANN_EMBEDDED_DOMAIN_SLO_PROFILE) --format run-external-args)"; \
	custom_args=""; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS)" ]; then custom_args="$$custom_args --max-neighbors $(ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_EF_SEARCH)" ]; then custom_args="$$custom_args --ef-search $(ANN_EMBEDDED_DOMAIN_EF_SEARCH)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION)" ]; then custom_args="$$custom_args --ef-construction $(ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION)"; fi; \
	if [ -n "$(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)" ]; then custom_args="$$custom_args --layer-count $(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)"; fi; \
	scripts/ann/run_external_corpus.sh --vectors $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric $(ANN_EMBEDDED_DOMAIN_METRIC) --output-root $(ANN_EMBEDDED_DOMAIN_RUN_ROOT) --run-id $(ANN_EMBEDDED_DOMAIN_RUN_ID) $$slo_args $$custom_args

ann-embedding-domain-export:
	@if [ -z "$(ANN_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_EMBEDDING_SOURCE_ROOT to a JSONL payload directory without vectors" >&2; exit 2; fi
	@if [ -z "$(ANN_EMBEDDING_QUERIES)" ]; then echo "Set ANN_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@if [ "$(ANN_EMBEDDING_PROVIDER)" = "command" ] && [ -z "$(ANN_EMBEDDING_COMMAND)" ]; then echo "Set ANN_EMBEDDING_COMMAND to a command that reads text on stdin and prints a JSON vector" >&2; exit 2; fi
	@if [ "$(ANN_EMBEDDING_PROVIDER)" = "file" ] && [ -z "$(ANN_EMBEDDING_FILE)" ]; then echo "Set ANN_EMBEDDING_FILE when ANN_EMBEDDING_PROVIDER=file" >&2; exit 2; fi
	python3 scripts/ann/export_embedding_domain_corpus.py --source-root $(ANN_EMBEDDING_SOURCE_ROOT) --queries $(ANN_EMBEDDING_QUERIES) --output-dir $(ANN_EMBEDDING_OUTPUT_DIR) --provider $(ANN_EMBEDDING_PROVIDER) --embedding-command "$(ANN_EMBEDDING_COMMAND)" --embedding-file "$(ANN_EMBEDDING_FILE)" --embedding-cache "$(ANN_EMBEDDING_CACHE)" --url "$(ANN_EMBEDDING_URL)" --model "$(ANN_EMBEDDING_MODEL)" --timeout-seconds $(ANN_EMBEDDING_TIMEOUT_SECONDS) --normalization $(ANN_EMBEDDING_NORMALIZATION) --scale $(ANN_EMBEDDING_SCALE) --limit $(ANN_EMBEDDING_LIMIT)

ann-embedding-domain-corpus-run: ann-embedding-domain-export
	$(MAKE) ann-embedded-domain-corpus-run ANN_EMBEDDED_DOMAIN_SOURCE_ROOT=$(ANN_EMBEDDING_OUTPUT_DIR)/payloads ANN_EMBEDDED_DOMAIN_QUERIES=$(ANN_EMBEDDING_OUTPUT_DIR)/queries.jsonl ANN_EMBEDDED_DOMAIN_OUTPUT_DIR=$(ANN_EMBEDDING_OUTPUT_DIR)/converted ANN_EMBEDDED_DOMAIN_RUN_ROOT=$(ANN_EMBEDDING_RUN_ROOT) ANN_EMBEDDED_DOMAIN_RUN_ID=$(ANN_EMBEDDING_RUN_ID) ANN_EMBEDDED_DOMAIN_METRIC=$(ANN_EMBEDDING_METRIC) ANN_EMBEDDED_DOMAIN_LIMIT=$(ANN_EMBEDDING_LIMIT) ANN_EMBEDDED_DOMAIN_SLO_PROFILE=$(ANN_EMBEDDING_SLO_PROFILE) ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS=$(ANN_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDED_DOMAIN_EF_SEARCH=$(ANN_EMBEDDING_EF_SEARCH) ANN_EMBEDDED_DOMAIN_EF_CONSTRUCTION=$(ANN_EMBEDDING_EF_CONSTRUCTION) ANN_EMBEDDED_DOMAIN_LAYER_COUNT=$(ANN_EMBEDDING_LAYER_COUNT)

ann-real-embedding-readiness:
	@source_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ROOT)" ]; then source_args="$$source_args --source-root $(ANN_REAL_EMBEDDING_SOURCE_ROOT)"; fi; \
	query_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_QUERIES)" ]; then query_args="--queries $(ANN_REAL_EMBEDDING_QUERIES)"; fi; \
	required_env_args=""; \
	if [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "command" ] || [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "openai-compatible" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_URL --require-env CORTEXDB_EMBEDDING_MODEL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "local" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_URL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_MODEL)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_MODEL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_API_KEY)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_API_KEY"; fi; \
	archive_args=""; \
	if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)" ]; then archive_args="$$archive_args --source-archive-manifest $(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_SOURCE_ARCHIVE)" = "true" ]; then archive_args="$$archive_args --require-source-archive"; fi; \
	python3 scripts/ann/real_embedding_readiness.py $$source_args $$query_args --provider $(ANN_REAL_EMBEDDING_PROVIDER) --embedding-command "$(ANN_REAL_EMBEDDING_COMMAND)" --embedding-file "$(ANN_REAL_EMBEDDING_FILE)" --embedding-cache "$(ANN_REAL_EMBEDDING_CACHE)" --url "$(ANN_REAL_EMBEDDING_URL)" --model "$(ANN_REAL_EMBEDDING_MODEL)" --timeout-seconds $(ANN_REAL_EMBEDDING_TIMEOUT_SECONDS) --metric $(ANN_REAL_EMBEDDING_METRIC) --normalization $(ANN_REAL_EMBEDDING_NORMALIZATION) --scale $(ANN_REAL_EMBEDDING_SCALE) --limit $(ANN_REAL_EMBEDDING_LIMIT) $$required_env_args $$archive_args --output $(ANN_REAL_EMBEDDING_READINESS_REPORT)

ann-real-embedding-preflight:
	@if [ -z "$(ANN_REAL_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_REAL_EMBEDDING_SOURCE_ROOT to a JSONL payload directory" >&2; exit 2; fi
	@if [ -z "$(ANN_REAL_EMBEDDING_QUERIES)" ]; then echo "Set ANN_REAL_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@required_env_args=""; \
	if [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "command" ] || [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "openai-compatible" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_URL --require-env CORTEXDB_EMBEDDING_MODEL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_PROVIDER)" = "local" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_URL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_MODEL)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_MODEL"; fi; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_API_KEY)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_API_KEY"; fi; \
	python3 scripts/ann/preflight_real_embedding_benchmark.py --source-root $(ANN_REAL_EMBEDDING_SOURCE_ROOT) --queries $(ANN_REAL_EMBEDDING_QUERIES) --provider $(ANN_REAL_EMBEDDING_PROVIDER) --embedding-command "$(ANN_REAL_EMBEDDING_COMMAND)" --embedding-file "$(ANN_REAL_EMBEDDING_FILE)" --embedding-cache "$(ANN_REAL_EMBEDDING_CACHE)" --url "$(ANN_REAL_EMBEDDING_URL)" --model "$(ANN_REAL_EMBEDDING_MODEL)" --timeout-seconds $(ANN_REAL_EMBEDDING_TIMEOUT_SECONDS) --metric $(ANN_REAL_EMBEDDING_METRIC) --normalization $(ANN_REAL_EMBEDDING_NORMALIZATION) --scale $(ANN_REAL_EMBEDDING_SCALE) --limit $(ANN_REAL_EMBEDDING_LIMIT) $$required_env_args --output $(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)

ann-real-embedding-benchmark: ann-real-embedding-preflight
	$(MAKE) ann-embedding-domain-corpus-run ANN_EMBEDDING_SOURCE_ROOT=$(ANN_REAL_EMBEDDING_SOURCE_ROOT) ANN_EMBEDDING_QUERIES=$(ANN_REAL_EMBEDDING_QUERIES) ANN_EMBEDDING_OUTPUT_DIR=$(ANN_REAL_EMBEDDING_OUTPUT_DIR) ANN_EMBEDDING_RUN_ROOT=$(ANN_REAL_EMBEDDING_RUN_ROOT) ANN_EMBEDDING_RUN_ID=$(ANN_REAL_EMBEDDING_RUN_ID) ANN_EMBEDDING_PROVIDER=$(ANN_REAL_EMBEDDING_PROVIDER) ANN_EMBEDDING_COMMAND="$(ANN_REAL_EMBEDDING_COMMAND)" ANN_EMBEDDING_FILE="$(ANN_REAL_EMBEDDING_FILE)" ANN_EMBEDDING_CACHE="$(ANN_REAL_EMBEDDING_CACHE)" ANN_EMBEDDING_URL="$(ANN_REAL_EMBEDDING_URL)" ANN_EMBEDDING_MODEL="$(ANN_REAL_EMBEDDING_MODEL)" ANN_EMBEDDING_TIMEOUT_SECONDS=$(ANN_REAL_EMBEDDING_TIMEOUT_SECONDS) ANN_EMBEDDING_NORMALIZATION=$(ANN_REAL_EMBEDDING_NORMALIZATION) ANN_EMBEDDING_SCALE=$(ANN_REAL_EMBEDDING_SCALE) ANN_EMBEDDING_METRIC=$(ANN_REAL_EMBEDDING_METRIC) ANN_EMBEDDING_LIMIT=$(ANN_REAL_EMBEDDING_LIMIT) ANN_EMBEDDING_SLO_PROFILE=$(ANN_REAL_EMBEDDING_SLO_PROFILE) ANN_EMBEDDING_MAX_NEIGHBORS=$(ANN_REAL_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDING_EF_SEARCH=$(ANN_REAL_EMBEDDING_EF_SEARCH) ANN_EMBEDDING_EF_CONSTRUCTION=$(ANN_REAL_EMBEDDING_EF_CONSTRUCTION) ANN_EMBEDDING_LAYER_COUNT=$(ANN_REAL_EMBEDDING_LAYER_COUNT)
	@if [ -n "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)" ]; then \
	  python3 scripts/ann/attach_real_embedding_metadata.py \
	    --run-dir "$(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)" \
	    --preflight "$(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)" \
	    --export-manifest "$(ANN_REAL_EMBEDDING_OUTPUT_DIR)/manifest.json" \
	    --source-archive-manifest "$(ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST)"; \
	else \
	  python3 scripts/ann/attach_real_embedding_metadata.py \
	    --run-dir "$(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)" \
	    --preflight "$(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)" \
	    --export-manifest "$(ANN_REAL_EMBEDDING_OUTPUT_DIR)/manifest.json"; \
	fi

ann-real-embedding-compare:
	@if [ -z "$(ANN_REAL_EMBEDDING_BASELINE_REPORT)" ]; then echo "Set ANN_REAL_EMBEDDING_BASELINE_REPORT to a real embedding baseline report.json" >&2; exit 2; fi
	python3 scripts/ann/compare_reports.py --baseline $(ANN_REAL_EMBEDDING_BASELINE_REPORT) --candidate $(ANN_REAL_EMBEDDING_CANDIDATE_REPORT) --output $(ANN_REAL_EMBEDDING_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-benchmark-and-compare: ann-real-embedding-benchmark ann-real-embedding-compare

ann-real-embedding-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT) --max-p95-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-history-regression-check:
	python3 scripts/ann/bootstrap_history_fixture.py --source $(ANN_HISTORY_CLEAN_FIXTURE) --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --min-runs $(ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS)
	python3 scripts/ann/history_gate.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT) --fail-on-regression --min-runs $(ANN_REAL_EMBEDDING_MIN_HISTORY_RUNS) --min-corpora 1 --max-p95-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_REAL_EMBEDDING_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_REAL_EMBEDDING_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --run-id $(ANN_REAL_EMBEDDING_RUN_ID) --baseline-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output-root $(ANN_REAL_EMBEDDING_BASELINE_ROOT)

ann-real-embedding-package-baseline: ann-real-embedding-publish-baseline
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_REAL_EMBEDDING_BASELINE_BUNDLE) --package-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE)

ann-real-embedding-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth --require-real-embedding-metadata

ann-real-embedding-release-check: ann-real-embedding-benchmark
	@if [ -n "$(ANN_REAL_EMBEDDING_BASELINE_REPORT)" ]; then \
	  $(MAKE) ann-real-embedding-compare; \
	else \
	  echo "Skipping real embedding baseline comparison: ANN_REAL_EMBEDDING_BASELINE_REPORT is not set"; \
	fi
	$(MAKE) ann-real-embedding-history-regression-check
	$(MAKE) ann-real-embedding-package-baseline
	$(MAKE) ann-real-embedding-validate-baseline-package

ann-bge-m3-cache-corpus-build:
	python3 scripts/ann/build_bge_m3_cached_corpus.py --corpus-vectors $(ANN_BGE_M3_CORPUS_VECTORS) --query-cache $(ANN_BGE_M3_QUERY_CACHE) --output-dir $(ANN_BGE_M3_OUTPUT_DIR) --model $(ANN_BGE_M3_MODEL) --max-documents $(ANN_BGE_M3_MAX_DOCUMENTS) --max-queries $(ANN_BGE_M3_MAX_QUERIES) --limit $(ANN_BGE_M3_LIMIT) --scale $(ANN_BGE_M3_SCALE) --normalization $(ANN_BGE_M3_NORMALIZATION)

ann-bge-m3-cache-recall-report: ann-bge-m3-cache-corpus-build
	scripts/ann/run_external_corpus.sh --vectors $(ANN_BGE_M3_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_BGE_M3_OUTPUT_DIR)/queries.jsonl --metric $(ANN_BGE_M3_METRIC) --output-root $(ANN_BGE_M3_RUN_ROOT) --run-id $(ANN_BGE_M3_RUN_ID) --min-recall-q16 65535 --min-mean-recall-q16 65535 --max-neighbors $(ANN_DEMO_DOMAIN_MAX_NEIGHBORS) --ef-search $(ANN_DEMO_DOMAIN_EF_SEARCH) --ef-construction $(ANN_DEMO_DOMAIN_EF_CONSTRUCTION) --layer-count $(ANN_DEMO_DOMAIN_LAYER_COUNT)

ann-slo-profile:
	python3 scripts/ann/slo_profile.py --profile $(ANN_REAL_EMBEDDING_SLO_PROFILE) --format json

ann-scripts-check:
	python3 scripts/ann/build_bge_m3_cached_corpus.py --self-test
	python3 scripts/ann/build_demo_domain_corpus.py --self-test
	python3 scripts/ann/build_embedded_domain_corpus.py --self-test
	python3 scripts/ann/embedding_provider_selftest.py
	python3 scripts/ann/export_embedding_domain_corpus.py --self-test
	python3 scripts/ann/embed_text_command.py --self-test
	python3 scripts/ann/preflight_real_embedding_benchmark.py --self-test
	python3 scripts/ann/attach_real_embedding_metadata.py --self-test
	python3 scripts/ann/bootstrap_history_fixture.py --self-test
	python3 scripts/ann/real_embedding_readiness.py --self-test
	python3 scripts/ann/slo_profile.py --self-test
	python3 scripts/ann/convert_public_corpus.py --self-test
	python3 scripts/ann/run_public_corpus.py --self-test
	python3 scripts/ann/exact_ground_truth.py --self-test
	python3 scripts/ann/recall_probe.py --self-test
	python3 scripts/ann/report_contract.py --self-test
	python3 scripts/ann/compare_reports.py --self-test
	python3 scripts/ann/summarize_history.py --self-test
	python3 scripts/ann/history_contract.py --self-test
	python3 scripts/ann/history_fixture_check.py --self-test
	python3 scripts/ann/history_gate.py --self-test
	python3 scripts/ann/publish_baseline.py --self-test
	python3 scripts/ann/package_baseline.py --self-test
	python3 scripts/ann/validate_baseline_package.py --self-test
	$(MAKE) ann-history-fixture-check
	mkdir -p target/ann
	python3 scripts/ann/exact_ground_truth.py --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output $(ANN_CORPUS_GENERATED_GROUND_TRUTH)
	diff -u $(ANN_CORPUS_GROUND_TRUTH) $(ANN_CORPUS_GENERATED_GROUND_TRUTH)

ann-convert-public-smoke:
	python3 scripts/ann/convert_public_corpus.py --self-test

ann-public-corpus-smoke:
	python3 scripts/ann/run_public_corpus.py --self-test

ann-public-corpus-run:
	@if [ -z "$(ANN_PUBLIC_SOURCE)" ]; then echo "Set ANN_PUBLIC_SOURCE to a URL, archive path, or extracted corpus directory" >&2; exit 2; fi
	@if [ -d "$(ANN_PUBLIC_SOURCE)" ]; then source_arg="--source-dir"; elif [ -f "$(ANN_PUBLIC_SOURCE)" ]; then source_arg="--source-archive"; else source_arg="--source-url"; fi; \
	python3 scripts/ann/run_public_corpus.py \
	  "$$source_arg" "$(ANN_PUBLIC_SOURCE)" \
	  --dataset-id "$(ANN_PUBLIC_DATASET_ID)" \
	  --format "$(ANN_PUBLIC_FORMAT)" \
	  --metric "$(ANN_PUBLIC_METRIC)" \
	  --normalization "$(ANN_PUBLIC_NORMALIZATION)" \
	  --scale "$(ANN_PUBLIC_SCALE)" \
	  --limit "$(ANN_PUBLIC_LIMIT)" \
	  --max-neighbors "$(ANN_PUBLIC_MAX_NEIGHBORS)" \
	  --ef-search "$(ANN_PUBLIC_EF_SEARCH)" \
	  --layer-count "$(ANN_PUBLIC_LAYER_COUNT)" \
	  --max-p99-latency-nanos "$(ANN_PUBLIC_MAX_P99_LATENCY_NANOS)" \
	  --output-root "$(ANN_PUBLIC_OUTPUT_ROOT)" \
	  --run-root "$(ANN_CORPUS_RUN_ROOT)" \
	  --run-id "$(ANN_PUBLIC_RUN_ID)"

ann-corpus-compare:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_REPORT) --candidate $(ANN_CANDIDATE_REPORT) --output $(ANN_REPORT_COMPARISON)

ann-corpus-run-smoke:
	scripts/ann/run_external_corpus.sh --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output-root $(ANN_CORPUS_RUN_ROOT) --run-id $(ANN_CORPUS_RUN_ID)

ann-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-history-regression-check:
	python3 scripts/ann/history_gate.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT) --fail-on-regression --min-runs 1 --min-corpora 1 --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-history-fixture-check:
	python3 scripts/ann/history_fixture_check.py \
	  --clean $(ANN_HISTORY_CLEAN_FIXTURE) \
	  --recall-regression $(ANN_HISTORY_RECALL_REGRESSION_FIXTURE) \
	  --latency-regression $(ANN_HISTORY_LATENCY_REGRESSION_FIXTURE)

ann-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_HISTORY_ROOT) --run-id $(ANN_BASELINE_RUN_ID) --baseline-id $(ANN_BASELINE_ID) --output-root $(ANN_BASELINE_ROOT)

ann-package-baseline:
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_BASELINE_BUNDLE) --package-id $(ANN_BASELINE_ID) --output $(ANN_BASELINE_ARCHIVE)

ann-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-compare-baseline-bundle:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_BUNDLE_REPORT) --candidate $(ANN_HISTORY_ROOT)/$(ANN_CANDIDATE_RUN_ID)/report.json --output $(ANN_BASELINE_BUNDLE_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-p99-regression-nanos $(ANN_MAX_P99_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-nightly-regression-report:
	$(MAKE) ann-fixture-report
	$(MAKE) ann-drift-report
	$(MAKE) ann-external-report
	$(MAKE) ann-metric-matrix-report
	$(MAKE) ann-corpus-smoke-report
	$(MAKE) ann-domain-corpus-report
	$(MAKE) ann-corpus-run-smoke
	$(MAKE) ann-demo-domain-corpus-run
	$(MAKE) ann-demo-domain-package-baseline
	$(MAKE) ann-demo-domain-validate-baseline-package
	$(MAKE) ann-publish-baseline
	$(MAKE) ann-package-baseline
	$(MAKE) ann-validate-baseline-package
	$(MAKE) ann-compare-baseline-bundle

ann-release-evidence-check:
	rm -rf $(ANN_RELEASE_EVIDENCE_ROOT)
	$(MAKE) ann-corpus-run-smoke ANN_CORPUS_RUN_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_CORPUS_RUN_ID=$(ANN_RELEASE_EVIDENCE_RUN_ID)
	$(MAKE) ann-history-regression-check ANN_HISTORY_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_HISTORY_REPORT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT)/history.json
	$(MAKE) ann-publish-baseline ANN_HISTORY_ROOT=$(ANN_RELEASE_EVIDENCE_RUN_ROOT) ANN_BASELINE_RUN_ID=$(ANN_RELEASE_EVIDENCE_RUN_ID) ANN_BASELINE_ID=$(ANN_RELEASE_EVIDENCE_BASELINE_ID) ANN_BASELINE_ROOT=$(ANN_RELEASE_EVIDENCE_BASELINE_ROOT)
	$(MAKE) ann-package-baseline ANN_BASELINE_BUNDLE=$(ANN_RELEASE_EVIDENCE_BASELINE_BUNDLE) ANN_BASELINE_ID=$(ANN_RELEASE_EVIDENCE_BASELINE_ID) ANN_BASELINE_ARCHIVE=$(ANN_RELEASE_EVIDENCE_BASELINE_ARCHIVE)
	$(MAKE) ann-validate-baseline-package ANN_BASELINE_ARCHIVE=$(ANN_RELEASE_EVIDENCE_BASELINE_ARCHIVE)
	$(MAKE) ann-demo-domain-package-baseline
	$(MAKE) ann-demo-domain-validate-baseline-package
	@echo "=== ANN release evidence check passed ==="
