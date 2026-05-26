.PHONY: check test alpha-check demo

check:
	cargo check --workspace

test:
	cargo test --workspace

alpha-check:
	RUSTFLAGS="-D warnings" cargo check --workspace
	RUSTFLAGS="-D warnings" cargo test --workspace --all-features
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	./sdk/publish/check.sh
	cargo bench -p cortex-engine --bench core_baseline
	./examples/demo/investment_projects/run.sh

demo:
	./examples/demo/investment_projects/run.sh
