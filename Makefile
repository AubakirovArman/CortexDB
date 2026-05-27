.PHONY: check test sdk-check openapi-check smoke-test sdk-smoke-test alpha-check demo

check:
	cargo check --workspace

test:
	cargo test --workspace

sdk-check:
	./sdk/publish/check.sh

openapi-check:
	python3 scripts/check_openapi_coverage.py

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
	cargo bench -p cortex-engine --bench core_baseline
	./examples/demo/investment_projects/run.sh

demo:
	./examples/demo/investment_projects/run.sh
