.PHONY: check test sdk-check sdk-release-contract-check openapi-check openapi-contract-check sdk-contract-check dashboard-smoke ann-fixture-check ann-fixture-report ann-drift-check ann-drift-report ann-external-check ann-external-report ann-metric-matrix-check ann-metric-matrix-report ann-corpus-smoke-check ann-corpus-smoke-report ann-domain-corpus-check ann-domain-corpus-report ann-demo-domain-corpus-build ann-demo-domain-corpus-run ann-demo-domain-publish-baseline ann-demo-domain-package-baseline ann-demo-domain-validate-baseline-package ann-embedded-domain-corpus-build ann-embedded-domain-corpus-run ann-embedding-domain-export ann-embedding-domain-corpus-run ann-real-embedding-preflight ann-real-embedding-benchmark ann-real-embedding-compare ann-real-embedding-benchmark-and-compare ann-real-embedding-history-report ann-real-embedding-history-regression-check ann-real-embedding-publish-baseline ann-real-embedding-package-baseline ann-real-embedding-validate-baseline-package ann-slo-profile ann-scripts-check ann-convert-public-smoke ann-public-corpus-smoke ann-public-corpus-run ann-corpus-compare ann-corpus-run-smoke ann-history-report ann-history-regression-check ann-publish-baseline ann-package-baseline ann-validate-baseline-package ann-compare-baseline-bundle smoke-test sdk-smoke-test alpha-check release-check demo

ANN_FIXTURE_BASELINE ?= crates/cortex-engine/fixtures/ann_fixture_baseline_v1.json
ANN_FIXTURE_REPORT ?= target/ann/ann_fixture_report.json
ANN_DRIFT_BASELINE ?= crates/cortex-engine/fixtures/ann_drift_baseline_v1.json
ANN_DRIFT_REPORT ?= target/ann/ann_drift_report.json
ANN_EXTERNAL_BASELINE ?= crates/cortex-engine/fixtures/ann_external_baseline_v1.json
ANN_EXTERNAL_REPORT ?= target/ann/ann_external_fixture_report.json
ANN_METRIC_MATRIX_BASELINE ?= crates/cortex-engine/fixtures/ann_metric_matrix_baseline_v1.json
ANN_METRIC_MATRIX_REPORT ?= target/ann/ann_metric_matrix_report.json
ANN_CORPUS_VECTORS ?= crates/cortex-engine/fixtures/ann_corpus_vectors_v1.jsonl
ANN_CORPUS_QUERIES ?= crates/cortex-engine/fixtures/ann_corpus_queries_v1.jsonl
ANN_CORPUS_GROUND_TRUTH ?= crates/cortex-engine/fixtures/ann_corpus_ground_truth_v1.jsonl
ANN_CORPUS_REPORT ?= target/ann/ann_corpus_report.json
ANN_CORPUS_GENERATED_GROUND_TRUTH ?= target/ann/generated_ground_truth.jsonl
ANN_DOMAIN_VECTORS ?= crates/cortex-engine/fixtures/ann_domain_vectors_v1.jsonl
ANN_DOMAIN_QUERIES ?= crates/cortex-engine/fixtures/ann_domain_queries_v1.jsonl
ANN_DOMAIN_GROUND_TRUTH ?= crates/cortex-engine/fixtures/ann_domain_ground_truth_v1.jsonl
ANN_DOMAIN_REPORT ?= target/ann/ann_domain_corpus_report.json
ANN_DEMO_DOMAIN_SOURCE_ROOT ?= examples/datasets
ANN_DEMO_DOMAIN_EXTRA_SOURCE_ROOT ?= examples/rag_demo/data
ANN_DEMO_DOMAIN_OUTPUT_DIR ?= target/ann/demo-domain-corpus/converted
ANN_DEMO_DOMAIN_RUN_ROOT ?= target/ann/demo-domain-corpus/runs
ANN_DEMO_DOMAIN_RUN_ID ?= demo-domain
ANN_DEMO_DOMAIN_DIMENSION ?= 64
ANN_DEMO_DOMAIN_LIMIT ?= 5
ANN_DEMO_DOMAIN_SCALE ?= 1200
ANN_DEMO_DOMAIN_MAX_NEIGHBORS ?= 16
ANN_DEMO_DOMAIN_EF_SEARCH ?= 128
ANN_DEMO_DOMAIN_LAYER_COUNT ?= 4
ANN_DEMO_DOMAIN_BASELINE_ID ?= $(ANN_DEMO_DOMAIN_RUN_ID)
ANN_DEMO_DOMAIN_BASELINE_ROOT ?= target/ann/demo-domain-corpus/release-baselines
ANN_DEMO_DOMAIN_BASELINE_BUNDLE ?= $(ANN_DEMO_DOMAIN_BASELINE_ROOT)/$(ANN_DEMO_DOMAIN_BASELINE_ID)
ANN_DEMO_DOMAIN_BASELINE_ARCHIVE ?= $(ANN_DEMO_DOMAIN_BASELINE_ROOT)/$(ANN_DEMO_DOMAIN_BASELINE_ID).tar.gz
ANN_EMBEDDED_DOMAIN_SOURCE_ROOT ?=
ANN_EMBEDDED_DOMAIN_QUERIES ?=
ANN_EMBEDDED_DOMAIN_OUTPUT_DIR ?= target/ann/embedded-domain-corpus/converted
ANN_EMBEDDED_DOMAIN_RUN_ROOT ?= target/ann/embedded-domain-corpus/runs
ANN_EMBEDDED_DOMAIN_RUN_ID ?= embedded-domain
ANN_EMBEDDED_DOMAIN_METRIC ?= dot_product
ANN_EMBEDDED_DOMAIN_LIMIT ?= 10
ANN_EMBEDDED_DOMAIN_SLO_PROFILE ?= balanced
ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS ?=
ANN_EMBEDDED_DOMAIN_EF_SEARCH ?=
ANN_EMBEDDED_DOMAIN_LAYER_COUNT ?=
ANN_EMBEDDING_SOURCE_ROOT ?=
ANN_EMBEDDING_QUERIES ?=
ANN_EMBEDDING_OUTPUT_DIR ?= target/ann/embedding-domain-corpus/export
ANN_EMBEDDING_RUN_ROOT ?= target/ann/embedding-domain-corpus/runs
ANN_EMBEDDING_RUN_ID ?= embedding-domain
ANN_EMBEDDING_PROVIDER ?= command
ANN_EMBEDDING_COMMAND ?=
ANN_EMBEDDING_NORMALIZATION ?= unit
ANN_EMBEDDING_SCALE ?= 32767
ANN_EMBEDDING_METRIC ?= cosine
ANN_EMBEDDING_LIMIT ?= 10
ANN_EMBEDDING_SLO_PROFILE ?= balanced
ANN_EMBEDDING_MAX_NEIGHBORS ?=
ANN_EMBEDDING_EF_SEARCH ?=
ANN_EMBEDDING_LAYER_COUNT ?=
ANN_REAL_EMBEDDING_SOURCE_ROOT ?=
ANN_REAL_EMBEDDING_QUERIES ?=
ANN_REAL_EMBEDDING_COMMAND ?= python3 scripts/ann/embed_text_command.py --require-model
ANN_REAL_EMBEDDING_OUTPUT_DIR ?= target/ann/real-embedding/export
ANN_REAL_EMBEDDING_RUN_ROOT ?= target/ann/real-embedding/runs
ANN_REAL_EMBEDDING_RUN_ID ?= real-embedding
ANN_REAL_EMBEDDING_PREFLIGHT_REPORT ?= target/ann/real-embedding/preflight.json
ANN_REAL_EMBEDDING_REQUIRE_API_KEY ?= false
ANN_REAL_EMBEDDING_NORMALIZATION ?= unit
ANN_REAL_EMBEDDING_SCALE ?= 32767
ANN_REAL_EMBEDDING_METRIC ?= cosine
ANN_REAL_EMBEDDING_LIMIT ?= 10
ANN_REAL_EMBEDDING_SLO_PROFILE ?= balanced
ANN_REAL_EMBEDDING_MAX_NEIGHBORS ?=
ANN_REAL_EMBEDDING_EF_SEARCH ?=
ANN_REAL_EMBEDDING_LAYER_COUNT ?=
ANN_REAL_EMBEDDING_BASELINE_REPORT ?=
ANN_REAL_EMBEDDING_CANDIDATE_REPORT ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)/report.json
ANN_REAL_EMBEDDING_COMPARISON ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/$(ANN_REAL_EMBEDDING_RUN_ID)/baseline_comparison.json
ANN_REAL_EMBEDDING_HISTORY_REPORT ?= $(ANN_REAL_EMBEDDING_RUN_ROOT)/history.json
ANN_REAL_EMBEDDING_BASELINE_ID ?= $(ANN_REAL_EMBEDDING_RUN_ID)
ANN_REAL_EMBEDDING_BASELINE_ROOT ?= target/ann/real-embedding/release-baselines
ANN_REAL_EMBEDDING_BASELINE_BUNDLE ?= $(ANN_REAL_EMBEDDING_BASELINE_ROOT)/$(ANN_REAL_EMBEDDING_BASELINE_ID)
ANN_REAL_EMBEDDING_BASELINE_ARCHIVE ?= $(ANN_REAL_EMBEDDING_BASELINE_ROOT)/$(ANN_REAL_EMBEDDING_BASELINE_ID).tar.gz
ANN_BASELINE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_CANDIDATE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_REPORT_COMPARISON ?= target/ann/ann_report_comparison.json
ANN_CORPUS_RUN_ID ?= smoke
ANN_CORPUS_RUN_ROOT ?= target/ann/corpus-runs
ANN_HISTORY_ROOT ?= $(ANN_CORPUS_RUN_ROOT)
ANN_HISTORY_REPORT ?= $(ANN_HISTORY_ROOT)/history.json
ANN_BASELINE_RUN_ID ?= $(ANN_CORPUS_RUN_ID)
ANN_BASELINE_ID ?= $(ANN_BASELINE_RUN_ID)
ANN_BASELINE_ROOT ?= target/ann/release-baselines
ANN_CANDIDATE_RUN_ID ?= $(ANN_CORPUS_RUN_ID)
ANN_BASELINE_BUNDLE ?= $(ANN_BASELINE_ROOT)/$(ANN_BASELINE_ID)
ANN_BASELINE_BUNDLE_REPORT ?= $(ANN_BASELINE_BUNDLE)/report.json
ANN_BASELINE_BUNDLE_COMPARISON ?= $(ANN_HISTORY_ROOT)/$(ANN_CANDIDATE_RUN_ID)/baseline_comparison.json
ANN_BASELINE_ARCHIVE ?= $(ANN_BASELINE_ROOT)/$(ANN_BASELINE_ID).tar.gz
ANN_MAX_P95_REGRESSION_NANOS ?= 0
ANN_MAX_MAX_REGRESSION_NANOS ?= 0
ANN_PUBLIC_SOURCE ?=
ANN_PUBLIC_DATASET_ID ?= public-ann
ANN_PUBLIC_FORMAT ?= fvecs
ANN_PUBLIC_METRIC ?= cosine
ANN_PUBLIC_RUN_ID ?= $(ANN_PUBLIC_DATASET_ID)-$(ANN_CORPUS_RUN_ID)
ANN_PUBLIC_OUTPUT_ROOT ?= target/ann/public-corpora
ANN_PUBLIC_NORMALIZATION ?= unit
ANN_PUBLIC_SCALE ?= 32767
ANN_PUBLIC_LIMIT ?= 10
ANN_PUBLIC_MAX_NEIGHBORS ?= 8
ANN_PUBLIC_EF_SEARCH ?= 64
ANN_PUBLIC_LAYER_COUNT ?= 4

check:
	cargo check --workspace

test:
	cargo test --workspace

sdk-check:
	./sdk/publish/check.sh

sdk-release-contract-check:
	python3 scripts/check_sdk_release_contract.py

openapi-check:
	python3 scripts/check_openapi_coverage.py

openapi-contract-check:
	python3 scripts/check_openapi_contract.py

sdk-contract-check:
	python3 scripts/check_sdk_contract.py

dashboard-smoke:
	cargo build -p cortex-server
	npm ci
	@if [ -n "$$CI" ]; then npx playwright install --with-deps chromium; else npx playwright install chromium; fi
	npm run dashboard:smoke

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

ann-corpus-smoke-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH)

ann-corpus-smoke-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --ground-truth $(ANN_CORPUS_GROUND_TRUTH) --output $(ANN_CORPUS_REPORT)

ann-domain-corpus-check:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535

ann-domain-corpus-report:
	cargo run --release -p cortex-engine --bin ann_corpus_check -- --vectors $(ANN_DOMAIN_VECTORS) --queries $(ANN_DOMAIN_QUERIES) --ground-truth $(ANN_DOMAIN_GROUND_TRUTH) --metric dot_product --min-recall-q16 65535 --min-mean-recall-q16 65535 --output $(ANN_DOMAIN_REPORT)

ann-demo-domain-corpus-build:
	python3 scripts/ann/build_demo_domain_corpus.py --source-root $(ANN_DEMO_DOMAIN_SOURCE_ROOT) --source-root $(ANN_DEMO_DOMAIN_EXTRA_SOURCE_ROOT) --output-dir $(ANN_DEMO_DOMAIN_OUTPUT_DIR) --dimension $(ANN_DEMO_DOMAIN_DIMENSION) --limit $(ANN_DEMO_DOMAIN_LIMIT) --scale $(ANN_DEMO_DOMAIN_SCALE)

ann-demo-domain-corpus-run: ann-demo-domain-corpus-build
	scripts/ann/run_external_corpus.sh --vectors $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_DEMO_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric dot_product --output-root $(ANN_DEMO_DOMAIN_RUN_ROOT) --run-id $(ANN_DEMO_DOMAIN_RUN_ID) --min-recall-q16 65535 --min-mean-recall-q16 65535 --max-neighbors $(ANN_DEMO_DOMAIN_MAX_NEIGHBORS) --ef-search $(ANN_DEMO_DOMAIN_EF_SEARCH) --layer-count $(ANN_DEMO_DOMAIN_LAYER_COUNT)

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
	if [ -n "$(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)" ]; then custom_args="$$custom_args --layer-count $(ANN_EMBEDDED_DOMAIN_LAYER_COUNT)"; fi; \
	scripts/ann/run_external_corpus.sh --vectors $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/vectors.jsonl --queries $(ANN_EMBEDDED_DOMAIN_OUTPUT_DIR)/queries.jsonl --metric $(ANN_EMBEDDED_DOMAIN_METRIC) --output-root $(ANN_EMBEDDED_DOMAIN_RUN_ROOT) --run-id $(ANN_EMBEDDED_DOMAIN_RUN_ID) $$slo_args $$custom_args

ann-embedding-domain-export:
	@if [ -z "$(ANN_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_EMBEDDING_SOURCE_ROOT to a JSONL payload directory without vectors" >&2; exit 2; fi
	@if [ -z "$(ANN_EMBEDDING_QUERIES)" ]; then echo "Set ANN_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@if [ "$(ANN_EMBEDDING_PROVIDER)" = "command" ] && [ -z "$(ANN_EMBEDDING_COMMAND)" ]; then echo "Set ANN_EMBEDDING_COMMAND to a command that reads text on stdin and prints a JSON vector" >&2; exit 2; fi
	python3 scripts/ann/export_embedding_domain_corpus.py --source-root $(ANN_EMBEDDING_SOURCE_ROOT) --queries $(ANN_EMBEDDING_QUERIES) --output-dir $(ANN_EMBEDDING_OUTPUT_DIR) --provider $(ANN_EMBEDDING_PROVIDER) --embedding-command "$(ANN_EMBEDDING_COMMAND)" --normalization $(ANN_EMBEDDING_NORMALIZATION) --scale $(ANN_EMBEDDING_SCALE) --limit $(ANN_EMBEDDING_LIMIT)

ann-embedding-domain-corpus-run: ann-embedding-domain-export
	$(MAKE) ann-embedded-domain-corpus-run ANN_EMBEDDED_DOMAIN_SOURCE_ROOT=$(ANN_EMBEDDING_OUTPUT_DIR)/payloads ANN_EMBEDDED_DOMAIN_QUERIES=$(ANN_EMBEDDING_OUTPUT_DIR)/queries.jsonl ANN_EMBEDDED_DOMAIN_OUTPUT_DIR=$(ANN_EMBEDDING_OUTPUT_DIR)/converted ANN_EMBEDDED_DOMAIN_RUN_ROOT=$(ANN_EMBEDDING_RUN_ROOT) ANN_EMBEDDED_DOMAIN_RUN_ID=$(ANN_EMBEDDING_RUN_ID) ANN_EMBEDDED_DOMAIN_METRIC=$(ANN_EMBEDDING_METRIC) ANN_EMBEDDED_DOMAIN_LIMIT=$(ANN_EMBEDDING_LIMIT) ANN_EMBEDDED_DOMAIN_SLO_PROFILE=$(ANN_EMBEDDING_SLO_PROFILE) ANN_EMBEDDED_DOMAIN_MAX_NEIGHBORS=$(ANN_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDED_DOMAIN_EF_SEARCH=$(ANN_EMBEDDING_EF_SEARCH) ANN_EMBEDDED_DOMAIN_LAYER_COUNT=$(ANN_EMBEDDING_LAYER_COUNT)

ann-real-embedding-preflight:
	@if [ -z "$(ANN_REAL_EMBEDDING_SOURCE_ROOT)" ]; then echo "Set ANN_REAL_EMBEDDING_SOURCE_ROOT to a JSONL payload directory" >&2; exit 2; fi
	@if [ -z "$(ANN_REAL_EMBEDDING_QUERIES)" ]; then echo "Set ANN_REAL_EMBEDDING_QUERIES to a JSONL query text file" >&2; exit 2; fi
	@required_env_args="--require-env CORTEXDB_EMBEDDING_URL --require-env CORTEXDB_EMBEDDING_MODEL"; \
	if [ "$(ANN_REAL_EMBEDDING_REQUIRE_API_KEY)" = "true" ]; then required_env_args="$$required_env_args --require-env CORTEXDB_EMBEDDING_API_KEY"; fi; \
	python3 scripts/ann/preflight_real_embedding_benchmark.py --source-root $(ANN_REAL_EMBEDDING_SOURCE_ROOT) --queries $(ANN_REAL_EMBEDDING_QUERIES) --embedding-command "$(ANN_REAL_EMBEDDING_COMMAND)" --metric $(ANN_REAL_EMBEDDING_METRIC) --normalization $(ANN_REAL_EMBEDDING_NORMALIZATION) --scale $(ANN_REAL_EMBEDDING_SCALE) --limit $(ANN_REAL_EMBEDDING_LIMIT) $$required_env_args --output $(ANN_REAL_EMBEDDING_PREFLIGHT_REPORT)

ann-real-embedding-benchmark: ann-real-embedding-preflight
	$(MAKE) ann-embedding-domain-corpus-run ANN_EMBEDDING_SOURCE_ROOT=$(ANN_REAL_EMBEDDING_SOURCE_ROOT) ANN_EMBEDDING_QUERIES=$(ANN_REAL_EMBEDDING_QUERIES) ANN_EMBEDDING_OUTPUT_DIR=$(ANN_REAL_EMBEDDING_OUTPUT_DIR) ANN_EMBEDDING_RUN_ROOT=$(ANN_REAL_EMBEDDING_RUN_ROOT) ANN_EMBEDDING_RUN_ID=$(ANN_REAL_EMBEDDING_RUN_ID) ANN_EMBEDDING_PROVIDER=command ANN_EMBEDDING_COMMAND="$(ANN_REAL_EMBEDDING_COMMAND)" ANN_EMBEDDING_NORMALIZATION=$(ANN_REAL_EMBEDDING_NORMALIZATION) ANN_EMBEDDING_SCALE=$(ANN_REAL_EMBEDDING_SCALE) ANN_EMBEDDING_METRIC=$(ANN_REAL_EMBEDDING_METRIC) ANN_EMBEDDING_LIMIT=$(ANN_REAL_EMBEDDING_LIMIT) ANN_EMBEDDING_SLO_PROFILE=$(ANN_REAL_EMBEDDING_SLO_PROFILE) ANN_EMBEDDING_MAX_NEIGHBORS=$(ANN_REAL_EMBEDDING_MAX_NEIGHBORS) ANN_EMBEDDING_EF_SEARCH=$(ANN_REAL_EMBEDDING_EF_SEARCH) ANN_EMBEDDING_LAYER_COUNT=$(ANN_REAL_EMBEDDING_LAYER_COUNT)

ann-real-embedding-compare:
	@if [ -z "$(ANN_REAL_EMBEDDING_BASELINE_REPORT)" ]; then echo "Set ANN_REAL_EMBEDDING_BASELINE_REPORT to a real embedding baseline report.json" >&2; exit 2; fi
	python3 scripts/ann/compare_reports.py --baseline $(ANN_REAL_EMBEDDING_BASELINE_REPORT) --candidate $(ANN_REAL_EMBEDDING_CANDIDATE_REPORT) --output $(ANN_REAL_EMBEDDING_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

ann-real-embedding-benchmark-and-compare: ann-real-embedding-benchmark ann-real-embedding-compare

ann-real-embedding-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT)

ann-real-embedding-history-regression-check:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --output $(ANN_REAL_EMBEDDING_HISTORY_REPORT) --fail-on-regression

ann-real-embedding-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_REAL_EMBEDDING_RUN_ROOT) --run-id $(ANN_REAL_EMBEDDING_RUN_ID) --baseline-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output-root $(ANN_REAL_EMBEDDING_BASELINE_ROOT)

ann-real-embedding-package-baseline: ann-real-embedding-publish-baseline
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_REAL_EMBEDDING_BASELINE_BUNDLE) --package-id $(ANN_REAL_EMBEDDING_BASELINE_ID) --output $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE)

ann-real-embedding-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_REAL_EMBEDDING_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-slo-profile:
	python3 scripts/ann/slo_profile.py --profile $(ANN_REAL_EMBEDDING_SLO_PROFILE) --format json

ann-scripts-check:
	python3 scripts/ann/build_demo_domain_corpus.py --self-test
	python3 scripts/ann/build_embedded_domain_corpus.py --self-test
	python3 scripts/ann/export_embedding_domain_corpus.py --self-test
	python3 scripts/ann/embed_text_command.py --self-test
	python3 scripts/ann/preflight_real_embedding_benchmark.py --self-test
	python3 scripts/ann/slo_profile.py --self-test
	python3 scripts/ann/convert_public_corpus.py --self-test
	python3 scripts/ann/run_public_corpus.py --self-test
	python3 scripts/ann/exact_ground_truth.py --self-test
	python3 scripts/ann/compare_reports.py --self-test
	python3 scripts/ann/summarize_history.py --self-test
	python3 scripts/ann/publish_baseline.py --self-test
	python3 scripts/ann/package_baseline.py --self-test
	python3 scripts/ann/validate_baseline_package.py --self-test
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
	  --output-root "$(ANN_PUBLIC_OUTPUT_ROOT)" \
	  --run-root "$(ANN_CORPUS_RUN_ROOT)" \
	  --run-id "$(ANN_PUBLIC_RUN_ID)"

ann-corpus-compare:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_REPORT) --candidate $(ANN_CANDIDATE_REPORT) --output $(ANN_REPORT_COMPARISON)

ann-corpus-run-smoke:
	scripts/ann/run_external_corpus.sh --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output-root $(ANN_CORPUS_RUN_ROOT) --run-id $(ANN_CORPUS_RUN_ID)

ann-history-report:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT)

ann-history-regression-check:
	python3 scripts/ann/summarize_history.py --run-root $(ANN_HISTORY_ROOT) --output $(ANN_HISTORY_REPORT) --fail-on-regression

ann-publish-baseline:
	python3 scripts/ann/publish_baseline.py --run-root $(ANN_HISTORY_ROOT) --run-id $(ANN_BASELINE_RUN_ID) --baseline-id $(ANN_BASELINE_ID) --output-root $(ANN_BASELINE_ROOT)

ann-package-baseline:
	python3 scripts/ann/package_baseline.py --baseline-bundle $(ANN_BASELINE_BUNDLE) --package-id $(ANN_BASELINE_ID) --output $(ANN_BASELINE_ARCHIVE)

ann-validate-baseline-package:
	python3 scripts/ann/validate_baseline_package.py --archive $(ANN_BASELINE_ARCHIVE) --require-production-safe --require-history --require-ground-truth

ann-compare-baseline-bundle:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_BUNDLE_REPORT) --candidate $(ANN_HISTORY_ROOT)/$(ANN_CANDIDATE_RUN_ID)/report.json --output $(ANN_BASELINE_BUNDLE_COMPARISON) --max-p95-regression-nanos $(ANN_MAX_P95_REGRESSION_NANOS) --max-max-regression-nanos $(ANN_MAX_MAX_REGRESSION_NANOS)

smoke-test:
	scripts/smoke_test.sh

sdk-smoke-test:
	python3 scripts/sdk_smoke_test.py

alpha-check:
	RUSTFLAGS="-D warnings" cargo check --workspace
	RUSTFLAGS="-D warnings" cargo test --workspace --all-features
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	$(MAKE) sdk-check
	$(MAKE) openapi-check
	$(MAKE) openapi-contract-check
	$(MAKE) sdk-contract-check
	$(MAKE) dashboard-smoke
	$(MAKE) ann-fixture-check
	$(MAKE) ann-drift-check
	$(MAKE) ann-external-check
	$(MAKE) ann-metric-matrix-check
	$(MAKE) ann-corpus-smoke-check
	$(MAKE) ann-domain-corpus-check
	$(MAKE) ann-scripts-check
	$(MAKE) ann-corpus-run-smoke
	$(MAKE) ann-demo-domain-corpus-run
	cargo bench -p cortex-engine --bench core_baseline
	./examples/demo/investment_projects/run.sh

release-check: alpha-check
	$(MAKE) ann-publish-baseline
	$(MAKE) ann-package-baseline
	$(MAKE) ann-demo-domain-package-baseline
	$(MAKE) smoke-test
	$(MAKE) sdk-smoke-test
	@echo "=== Release check passed ==="

demo:
	./examples/demo/investment_projects/run.sh
