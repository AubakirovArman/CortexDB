.PHONY: check test alpha-check demo

check:
	cargo check --workspace

test:
	cargo test --workspace

alpha-check:
	cargo check --workspace
	cargo test --workspace --all-features
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings

demo:
	./examples/demo/investment_projects/run.sh
