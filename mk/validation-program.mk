# F5.3 — benchmark validation program aggregate.
#
# `make validation-nightly` runs the benchmark-validation lane's regression /
# packaging gates in one sequential chain, matching the machine-readable lane map
# (fixtures/benchmarks/lanes.v1.json) and the nightly.yml benchmark-validation
# job. It is the single entry point for the nightly benchmark validation run
# (120-minute budget, stable toolchain). Every gate in the chain is deterministic
# and self-contained (self-tested fast lanes + real dataset adapters); none needs
# an LLM. Artifact upload (target/benchmark-registry, target/aab, target/locomo,
# …) is wired `if: always()` in the workflow job.

.PHONY: validation-nightly
validation-nightly:
	$(MAKE) benchmark-registry-check
	$(MAKE) longmemeval-per-type-regression-check
	$(MAKE) longmemeval-per-type-report-check
	$(MAKE) longmemeval-v1-typeaware-check
	$(MAKE) longmemeval-v1-hybrid-retrieval-check
	$(MAKE) erb-category-regression-check
	$(MAKE) multihop-retrieval-regression-check
	$(MAKE) locomo-retrieval-adapter-check
	$(MAKE) locomo-retrieval-regression-check
	$(MAKE) locomo-qa-reader-check
	$(MAKE) locomo-qa-input-check
	$(MAKE) locomo-qa-evidence-check
	$(MAKE) interim-gemma-qa-score-check
	$(MAKE) erb-official-rejudge-ready-check
	$(MAKE) aab-mini-score-check
	$(MAKE) aab-snapshot-matrix-check
	$(MAKE) quick-retrieval-eval-check
	$(MAKE) benchmark-trend-check
	@echo "validation-nightly: benchmark validation program complete"
