smoke-test:
	scripts/smoke_test.sh

sdk-smoke-test:
	python3 scripts/sdk_smoke_test.py

rag-demo-smoke:
	python3 examples/rag_demo/smoke.py

alpha-check:
	RUSTFLAGS="-D warnings" cargo check --workspace
	RUSTFLAGS="-D warnings" cargo test --workspace --all-features
	cargo fmt --check
	cargo clippy --workspace --all-targets -- -D warnings
	$(MAKE) sdk-check
	$(MAKE) openapi-check
	$(MAKE) openapi-contract-check
	$(MAKE) sdk-contract-check
	$(MAKE) migration-policy-check
	$(MAKE) storage-format-change-note-check
	$(MAKE) migration-compatibility-check
	$(MAKE) beta-delta-check
	$(MAKE) correctness-prerequisites-check
	$(MAKE) accountability-receipt-check
	$(MAKE) public-claims-check
	$(MAKE) load-smoke-check
	$(MAKE) single-node-performance-check
	$(MAKE) performance-trend-check
	$(MAKE) tenant-recovery-check
	$(MAKE) context-verify-quality-check
	$(MAKE) dashboard-check
	$(MAKE) dashboard-smoke
	$(MAKE) dashboard-screenshots
	$(MAKE) dashboard-release-check
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
	$(MAKE) rag-demo-smoke

release-check: alpha-check
	$(MAKE) binary-release-check
	$(MAKE) production-evidence-sweep
	$(MAKE) backup-offsite-check
	$(MAKE) upgrade-rollback-cli-flow-check
	$(MAKE) crash-fault-check
	$(MAKE) chaos-restart-check
	$(MAKE) replication-lifecycle-check
	$(MAKE) consensus-release-lane-check
	$(MAKE) receipt-production-readiness-check
	$(MAKE) retrieval-quality-check
	$(MAKE) release-regression-dashboard-check
	$(MAKE) smoke-test
	$(MAKE) sdk-smoke-test
	$(MAKE) versioning-policy-check
	$(MAKE) release-evidence-bundle-check
	$(MAKE) release-artifact-manifest-production-check
	$(MAKE) release-notes-generate
	$(MAKE) evidence-artifact-retention-check
	@echo "=== Release check passed ==="

demo:
	./examples/demo/investment_projects/run.sh

render-flagship-demo-gif:
	./scripts/render_flagship_demo_gif.sh

flagship-demo-check:
	python3 scripts/flagship_demo_check.py
