binary-release-package:
	cargo build --release -p cortex-cli --bin cortexdb
	cargo build --release -p cortex-server --bin cortex-server
	python3 scripts/package_binaries.py --package-id "$(BINARY_RELEASE_ID)" --platform "$(BINARY_RELEASE_PLATFORM)" --version "$(BINARY_RELEASE_VERSION)" --archive "$(BINARY_RELEASE_ARCHIVE)"

binary-release-validate:
	python3 scripts/package_binaries.py --validate --archive "$(BINARY_RELEASE_ARCHIVE)"

binary-platform-matrix-check:
	python3 scripts/binary_platform_matrix_check.py --archive "$(BINARY_RELEASE_ARCHIVE)" --report "$(BINARY_PLATFORM_MATRIX_REPORT)"

install-script-check:
	python3 scripts/install_script_check.py

release-artifact-manifest-check:
	python3 scripts/release_artifact_manifest_check.py --version "$(BINARY_RELEASE_VERSION)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)" --manifest "$(RELEASE_ARTIFACT_MANIFEST)" --report "$(RELEASE_ARTIFACT_MANIFEST_REPORT)"

release-artifact-manifest-production-check:
	python3 scripts/release_artifact_manifest_check.py --version "$(BINARY_RELEASE_VERSION)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)" --evidence-bundle "$(RELEASE_EVIDENCE_BUNDLE_ARCHIVE)" --require-evidence-bundle --manifest "$(RELEASE_ARTIFACT_MANIFEST)" --report "$(RELEASE_ARTIFACT_MANIFEST_REPORT)"

release-evidence-bundle-check:
	python3 scripts/release_evidence_bundle.py --root "$(RELEASE_EVIDENCE_BUNDLE_ROOT)" --manifest "$(RELEASE_EVIDENCE_BUNDLE_MANIFEST)" --report "$(RELEASE_EVIDENCE_BUNDLE_REPORT)" --archive "$(RELEASE_EVIDENCE_BUNDLE_ARCHIVE)" --binary-archive "$(BINARY_RELEASE_ARCHIVE)"

release-notes-generate:
	python3 scripts/generate_release_notes.py --version "$(BINARY_RELEASE_VERSION)" --production-evidence-report "$(PRODUCTION_EVIDENCE_REPORT)" --evidence-bundle-report "$(RELEASE_EVIDENCE_BUNDLE_REPORT)" --release-manifest "$(RELEASE_ARTIFACT_MANIFEST)" --output "$(GENERATED_RELEASE_NOTES)"

evidence-artifact-retention-check:
	python3 scripts/evidence_artifact_retention_check.py --report "$(EVIDENCE_ARTIFACT_RETENTION_REPORT)"

versioning-policy-check:
	python3 scripts/versioning_policy_check.py --report "$(VERSIONING_POLICY_REPORT)"

binary-release-check:
	python3 scripts/package_binaries.py --self-test
	$(MAKE) install-script-check
	$(MAKE) binary-release-package
	$(MAKE) binary-release-validate
	$(MAKE) binary-platform-matrix-check

beta-delta-check:
	python3 scripts/check_beta_delta.py

beta-foundation-check:
	python3 scripts/beta_foundation_check.py --root "$(BETA_FOUNDATION_ROOT)" --report "$(BETA_FOUNDATION_REPORT)"

beta-rc-check:
	python3 scripts/beta_rc_check.py --root "$(BETA_RC_ROOT)" --report "$(BETA_RC_REPORT)"

beta-landing-check:
	python3 scripts/beta_landing_check.py --report "$(BETA_LANDING_REPORT)"

beta-release-check: beta-landing-check
	python3 scripts/beta_release_bundle.py --root "$(BETA_RELEASE_ROOT)" --report "$(BETA_RELEASE_REPORT)" --archive "$(BETA_RELEASE_ARCHIVE)"

use-case-pack-check:
	python3 scripts/use_case_pack_check.py --report "$(USE_CASE_PACK_REPORT)"

contributor-onboarding-check:
	python3 scripts/contributor_onboarding_check.py --report "$(CONTRIBUTOR_ONBOARDING_REPORT)"

community-roadmap-check:
	python3 scripts/community_roadmap_check.py --report "$(COMMUNITY_ROADMAP_REPORT)"

public-retrieval-benchmark-page-check:
	python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output "$(RETRIEVAL_BETA_REPORT)" --min-domains 4 --repeat-runs 5
	$(MAKE) retrieval-quality-history-check
	python3 scripts/public_retrieval_benchmark_check.py --page docs/PUBLIC_RETRIEVAL_BENCHMARKS.md --beta-report "$(RETRIEVAL_BETA_REPORT)" --history-report "$(RETRIEVAL_QUALITY_HISTORY_REPORT)" --report "$(PUBLIC_RETRIEVAL_BENCHMARKS_REPORT)"

public-benchmarks-check: public-retrieval-benchmark-page-check
	python3 scripts/public_benchmarks_check.py --report "$(PUBLIC_BENCHMARKS_REPORT)"

comparison-docs-check:
	python3 scripts/comparison_docs_check.py --report "$(COMPARISON_DOCS_REPORT)"

docs-link-check:
	python3 scripts/docs_link_check.py

getting-started-check:
	python3 scripts/getting_started_check.py

agent-memory-demo-check:
	python3 scripts/agent_memory_demo_check.py --report "$(AGENT_MEMORY_DEMO_REPORT)"

agent-session-check:
	cargo test -p cortex-engine --test agent_session_tests

feedback-learning-check:
	cargo test -p cortex-engine --test feedback_tests --test context_pack

memory-quality-benchmark-check:
	python3 scripts/memory_quality_benchmark_check.py --report "$(MEMORY_QUALITY_BENCHMARK_REPORT)"

tool-registry-check:
	python3 scripts/tool_registry_check.py --report "$(TOOL_REGISTRY_REPORT)"

context-pack-tool-recommendation-check:
	python3 scripts/context_pack_tool_recommendation_check.py --report "$(CONTEXT_PACK_TOOL_RECOMMENDATION_REPORT)"

knowledge-graph-check:
	python3 scripts/knowledge_graph_check.py --report "$(KNOWLEDGE_GRAPH_REPORT)"

production-hardening-check:
	python3 scripts/production_hardening_check.py --root "$(PRODUCTION_HARDENING_ROOT)" --report "$(PRODUCTION_HARDENING_REPORT)"

production-candidate-check:
	python3 scripts/production_candidate_check.py --root "$(PRODUCTION_CANDIDATE_ROOT)" --report "$(PRODUCTION_CANDIDATE_REPORT)"

production-v1-check:
	python3 scripts/production_v1_check.py --root "$(PRODUCTION_V1_ROOT)" --report "$(PRODUCTION_V1_REPORT)"

public-claims-check:
	python3 scripts/check_public_claims.py --report "$(PUBLIC_CLAIMS_REPORT)"

load-smoke-check:
	cargo build -p cortex-server --bin cortex-server
	python3 scripts/load_smoke_check.py --server ./target/debug/cortex-server --root "$(LOAD_SMOKE_ROOT)" --report "$(LOAD_SMOKE_REPORT)" --cells "$(LOAD_SMOKE_CELLS)" --reads "$(LOAD_SMOKE_READS)" --searches "$(LOAD_SMOKE_SEARCHES)" --contexts "$(LOAD_SMOKE_CONTEXTS)" --verifies "$(LOAD_SMOKE_VERIFIES)" --workers "$(LOAD_SMOKE_WORKERS)"

load-suite-check:
	cargo build -p cortex-server --bin cortex-server
	python3 scripts/load_suite_check.py --server ./target/debug/cortex-server --root "target/load-suite" --report "target/load-suite/report.json"
