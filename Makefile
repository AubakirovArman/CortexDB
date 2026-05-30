.PHONY: check test sdk-check openapi-check openapi-contract-check sdk-contract-check ann-fixture-check ann-fixture-report ann-drift-check ann-drift-report ann-external-check ann-external-report ann-metric-matrix-check ann-metric-matrix-report ann-corpus-smoke-check ann-corpus-smoke-report ann-scripts-check ann-corpus-compare smoke-test sdk-smoke-test alpha-check release-check demo

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
ANN_BASELINE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_CANDIDATE_REPORT ?= $(ANN_CORPUS_REPORT)
ANN_REPORT_COMPARISON ?= target/ann/ann_report_comparison.json

check:
	cargo check --workspace

test:
	cargo test --workspace

sdk-check:
	./sdk/publish/check.sh

openapi-check:
	python3 scripts/check_openapi_coverage.py

openapi-contract-check:
	python3 scripts/check_openapi_contract.py

sdk-contract-check:
	python3 scripts/check_sdk_contract.py

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

ann-scripts-check:
	python3 scripts/ann/exact_ground_truth.py --self-test
	python3 scripts/ann/compare_reports.py --self-test
	mkdir -p target/ann
	python3 scripts/ann/exact_ground_truth.py --vectors $(ANN_CORPUS_VECTORS) --queries $(ANN_CORPUS_QUERIES) --output $(ANN_CORPUS_GENERATED_GROUND_TRUTH)
	diff -u $(ANN_CORPUS_GROUND_TRUTH) $(ANN_CORPUS_GENERATED_GROUND_TRUTH)

ann-corpus-compare:
	python3 scripts/ann/compare_reports.py --baseline $(ANN_BASELINE_REPORT) --candidate $(ANN_CANDIDATE_REPORT) --output $(ANN_REPORT_COMPARISON)

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
	$(MAKE) ann-fixture-check
	$(MAKE) ann-drift-check
	$(MAKE) ann-external-check
	$(MAKE) ann-metric-matrix-check
	$(MAKE) ann-corpus-smoke-check
	$(MAKE) ann-scripts-check
	cargo bench -p cortex-engine --bench core_baseline
	./examples/demo/investment_projects/run.sh

release-check: alpha-check
	$(MAKE) smoke-test
	$(MAKE) sdk-smoke-test
	@echo "=== Release check passed ==="

demo:
	./examples/demo/investment_projects/run.sh
