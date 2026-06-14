single-node-performance-check:
	cargo run --release -p cortex-engine --bin single_node_performance_check -- --root "$(SINGLE_NODE_PERF_ROOT)" --report "$(SINGLE_NODE_PERF_REPORT)" --cells "$(SINGLE_NODE_PERF_CELLS)" --max-total-ms "$(SINGLE_NODE_PERF_MAX_TOTAL_MS)" --min-ingest-cells-per-sec "$(SINGLE_NODE_PERF_MIN_INGEST_CELLS_PER_SEC)" --max-rss-bytes "$(SINGLE_NODE_PERF_MAX_RSS_BYTES)"
.PHONY: verify-performance-check numeric-verify-index-check
verify-performance-check:
	cargo run --release -p cortex-engine --bin verify_performance_check -- --root "$(VERIFY_PERF_ROOT)" --report "$(VERIFY_PERF_REPORT)" --cells "$(VERIFY_PERF_CELLS)" --warmup-samples "$(VERIFY_PERF_WARMUP_SAMPLES)" --samples "$(VERIFY_PERF_SAMPLES)" --max-p95-ms "$(VERIFY_PERF_MAX_P95_MS)"
numeric-verify-index-check:
	cargo run --release -p cortex-engine --bin numeric_verify_index_check -- --root "$(NUMERIC_VERIFY_INDEX_ROOT)" --report "$(NUMERIC_VERIFY_INDEX_REPORT)" --cells "$(NUMERIC_VERIFY_INDEX_CELLS)" --samples "$(NUMERIC_VERIFY_INDEX_SAMPLES)" --batch-size "$(NUMERIC_VERIFY_INDEX_BATCH_SIZE)" --max-p95-ms "$(NUMERIC_VERIFY_INDEX_MAX_P95_MS)" $(NUMERIC_VERIFY_INDEX_SKIP_VALIDATION)

.PHONY: memory-profile memory-estimate-audit
memory-profile:
	cargo run --release -p cortex-engine --bin memory_profile_check -- --root "$(MEMORY_PROFILE_ROOT)" --report "$(MEMORY_PROFILE_REPORT)" --cells "$(MEMORY_PROFILE_CELLS)" --batch-size "$(MEMORY_PROFILE_BATCH_SIZE)" $(MEMORY_PROFILE_DIRECT_CHECKPOINT) --payload-bytes "$(MEMORY_PROFILE_PAYLOAD_BYTES)" $(MEMORY_PROFILE_REOPEN_ONLY) --read-samples "$(MEMORY_PROFILE_READ_SAMPLES)" --payload-residency "$(MEMORY_PROFILE_PAYLOAD_RESIDENCY)" --max-rss-to-estimated-total-ratio "$(MEMORY_PROFILE_MAX_RSS_TO_ESTIMATED_TOTAL_RATIO)"
memory-estimate-audit: ; python3 scripts/memory_estimate_audit.py
.PHONY: scale-bench-100k scale-bench-1m scale-bench-10m-lazy scale-bench-ci scale-bench-trends continuous-benchmark-hosted-gate
scale-bench-100k:
	cargo run --release -p cortex-engine --bin scale_benchmark_check -- --root "$(SCALE_BENCH_ROOT)/100k" --report "$(SCALE_BENCH_100K_REPORT)" --cells 100000 --samples "$(SCALE_BENCH_SAMPLES)" --search-samples "$(SCALE_BENCH_SEARCH_SAMPLES)" --context-samples "$(SCALE_BENCH_CONTEXT_SAMPLES)" --verify-samples "$(SCALE_BENCH_VERIFY_SAMPLES)" --batch-size "$(SCALE_BENCH_BATCH_SIZE)"

scale-bench-1m:
	cargo run --release -p cortex-engine --bin scale_benchmark_check -- --root "$(SCALE_BENCH_ROOT)/1m" --report "$(SCALE_BENCH_1M_REPORT)" --cells 1000000 --samples "$(SCALE_BENCH_SAMPLES)" --search-samples "$(SCALE_BENCH_SEARCH_SAMPLES)" --context-samples "$(SCALE_BENCH_CONTEXT_SAMPLES)" --verify-samples "$(SCALE_BENCH_VERIFY_SAMPLES)" --batch-size "$(SCALE_BENCH_BATCH_SIZE)"

scale-bench-10m-lazy:
	cargo run --release -p cortex-engine --bin scale_benchmark_check -- --root "$(SCALE_BENCH_ROOT)/10m-lazy" --report "$(SCALE_BENCH_10M_LAZY_REPORT)" --cells 10000000 --samples "$(SCALE_BENCH_10M_SAMPLES)" --search-samples 0 --context-samples 0 --verify-samples 0 --batch-size "$(SCALE_BENCH_10M_BATCH_SIZE)" --payload-bytes "$(SCALE_BENCH_10M_PAYLOAD_BYTES)" --direct-checkpoint --payload-residency lazy --skip-storage-estimates --skip-validation

scale-bench-ci:
	cargo run --release -p cortex-engine --bin scale_benchmark_check -- --root "$(SCALE_BENCH_ROOT)/ci-10k" --report "$(SCALE_BENCH_CI_10K_REPORT)" --cells 10000 --samples "$(SCALE_BENCH_CI_SAMPLES)" --search-samples 0 --context-samples 0 --verify-samples 0 --batch-size "$(SCALE_BENCH_CI_BATCH_SIZE)" --payload-bytes "$(SCALE_BENCH_CI_PAYLOAD_BYTES)" --direct-checkpoint
	cargo run --release -p cortex-engine --bin scale_benchmark_check -- --root "$(SCALE_BENCH_ROOT)/ci-100k" --report "$(SCALE_BENCH_CI_100K_REPORT)" --cells 100000 --samples "$(SCALE_BENCH_CI_SAMPLES)" --search-samples 0 --context-samples 0 --verify-samples 0 --batch-size "$(SCALE_BENCH_CI_BATCH_SIZE)" --payload-bytes "$(SCALE_BENCH_CI_PAYLOAD_BYTES)" --direct-checkpoint

scale-bench-trends: ; python3 scripts/scale_benchmark_trends.py --root "$(SCALE_BENCH_ROOT)" --report "$(SCALE_BENCH_ROOT)/trends.json" --markdown "$(SCALE_BENCH_ROOT)/trends.md"
.PHONY: core-property-check core-property-random-check
core-property-check:
	cargo test -p cortex-engine --test core_property_tests

core-property-random-check:
	CORTEXDB_CORE_PROPERTY_RANDOM_SEED="$(CORE_PROPERTY_RANDOM_SEED)" cargo test -p cortex-engine --test core_property_tests

performance-trend-check:
	python3 scripts/performance_trend_check.py --load-report "$(LOAD_SMOKE_REPORT)" --single-node-report "$(SINGLE_NODE_PERF_REPORT)" --history-root "$(PERFORMANCE_HISTORY_ROOT)" --report "$(PERFORMANCE_TREND_REPORT)"
continuous-benchmark-gate: ; $(MAKE) performance-trend-check && $(MAKE) scale-bench-trends && $(MAKE) memory-estimate-audit && python3 scripts/continuous_benchmark_gate.py --min-regression-delta-ms "$(CONTINUOUS_BENCHMARK_MIN_REGRESSION_DELTA_MS)"
continuous-benchmark-hosted-gate: ; $(MAKE) load-smoke-check && $(MAKE) single-node-performance-check && $(MAKE) scale-bench-ci && $(MAKE) performance-trend-check PERFORMANCE_HISTORY_ROOT="$(HOSTED_PERFORMANCE_HISTORY_ROOT)" && $(MAKE) scale-bench-trends && $(MAKE) memory-estimate-audit && python3 scripts/continuous_benchmark_gate.py --min-regression-delta-ms "$(HOSTED_BENCHMARK_MIN_REGRESSION_DELTA_MS)" && python3 scripts/continuous_benchmark_gate.py --self-test
release-regression-dashboard-check:
	python3 scripts/release_regression_dashboard.py --baseline "$(RELEASE_REGRESSION_BASELINE)" --backup-report "$(BACKUP_DRILL_REPORT)" --single-node-report "$(SINGLE_NODE_PERF_REPORT)" --retrieval-report "$(RETRIEVAL_QUALITY_REPORT)" --context-report "$(CONTEXT_PACK_QUALITY_REPORT)" --verify-report "$(VERIFICATION_QUALITY_REPORT)" --api-report "$(HTTP_CONTRACT_OPS_REPORT)" --sdk-report "$(SDK_E2E_RELEASE_REPORT)" --report "$(RELEASE_REGRESSION_DASHBOARD_REPORT)" --markdown "$(RELEASE_REGRESSION_DASHBOARD_MARKDOWN)"

tenant-recovery-check:
	cargo build -p cortex-server --bin cortex-server
	cargo build -p cortex-cli --bin cortexdb
	python3 scripts/tenant_recovery_check.py --server ./target/debug/cortex-server --cli ./target/debug/cortexdb --root "$(TENANT_RECOVERY_ROOT)" --report "$(TENANT_RECOVERY_REPORT)"

context-verify-quality-check:
	cargo test -p cortex-engine --test context_verify_quality

dashboard-build:
	python3 scripts/dashboard_build.py

dashboard-standalone-build: dashboard-build

dashboard-check:
	python3 scripts/dashboard_build.py --check

dashboard-standalone-check: dashboard-check

dashboard-standalone-smoke: dashboard-standalone-check
	python3 scripts/dashboard_dist_smoke.py

dashboard-package: dashboard-standalone-build
	python3 scripts/dashboard_release.py --package-id $(DASHBOARD_PACKAGE_ID) --archive $(DASHBOARD_PACKAGE_ARCHIVE)

dashboard-validate-package:
	python3 scripts/dashboard_release.py --validate --archive $(DASHBOARD_PACKAGE_ARCHIVE)

dashboard-release-check:
	python3 scripts/dashboard_release.py --self-test
	$(MAKE) dashboard-package
	$(MAKE) dashboard-validate-package

dashboard-product-check: dashboard-standalone-smoke
	$(MAKE) dashboard-release-check
	python3 scripts/dashboard_product_check.py --report "$(DASHBOARD_PRODUCT_REPORT)"

single-node-slo-dashboard-check: dashboard-standalone-smoke
	python3 scripts/single_node_slo_dashboard_check.py --report "$(SINGLE_NODE_SLO_DASHBOARD_REPORT)"

dashboard-operational-status-check: dashboard-standalone-smoke
	python3 scripts/dashboard_operational_status_check.py --report "$(DASHBOARD_OPERATIONAL_STATUS_REPORT)"

context-pack-explorer-check: dashboard-standalone-smoke
	python3 scripts/context_pack_explorer_check.py --report "$(CONTEXT_PACK_EXPLORER_REPORT)"

verification-explorer-check: dashboard-standalone-smoke
	python3 scripts/verification_explorer_check.py --report "$(VERIFICATION_EXPLORER_REPORT)"

retrieval-quality-explorer-check:
	python3 scripts/retrieval_quality_dashboard_self_test.py
	python3 scripts/retrieval_quality_explorer_check.py --report "$(RETRIEVAL_QUALITY_EXPLORER_REPORT)"

permissions-view-check: dashboard-standalone-smoke
	python3 scripts/permissions_view_check.py --report "$(PERMISSIONS_VIEW_REPORT)"

audit-viewer-v2-check: dashboard-standalone-smoke
	python3 scripts/audit_viewer_v2_check.py --report "$(AUDIT_VIEWER_V2_REPORT)"

backup-restore-view-check: dashboard-standalone-smoke
	python3 scripts/backup_restore_view_check.py --report "$(BACKUP_RESTORE_VIEW_REPORT)"

incident-view-check: dashboard-standalone-smoke
	python3 scripts/incident_view_check.py --report "$(INCIDENT_VIEW_REPORT)"

dashboard-role-ui-check: dashboard-standalone-smoke
	python3 scripts/dashboard_role_ui_check.py --report "$(DASHBOARD_ROLE_UI_REPORT)"

ingestion-job-dashboard-check: dashboard-standalone-smoke
	cargo test -p cortex-server dashboard_
	python3 scripts/dashboard_product_check.py --report "$(INGESTION_JOB_DASHBOARD_REPORT)"

ingestion-validation-report-check:
	cargo test -p cortex-engine --test ingestion_validation_report
	cargo test -p cortex-server response_snapshot_tests
	$(MAKE) openapi-contract-check

ingestion-backpressure-check:
	cargo test -p cortex-engine --test ingestion_backpressure
	cargo test -p cortex-server ingestion_backpressure_
	$(MAKE) openapi-contract-check

ingestion-deduplication-check:
	cargo test -p cortex-engine --test ingestion_adapters text_ingestion_writes_content_and_source_hash_metadata
	cargo test -p cortex-engine --test ingestion_adapters text_ingestion_skip_existing_policy_skips_duplicate_chunks
	cargo test -p cortex-engine --test ingestion_adapters json_ingestion_writes_fact_cells

structured-source-ref-check: dashboard-standalone-smoke
	cargo test -p cortex-engine --test ingestion_tests cell_metadata_parses_source_ref_aliases_and_url
	cargo test -p cortex-engine --test ingestion_adapters
	cargo test -p cortex-engine --test ingestion_validation_report
	cargo test -p cortex-server response_snapshot_tests
	python3 scripts/structured_source_ref_check.py --report "$(STRUCTURED_SOURCE_REF_REPORT)"

deterministic-chunking-check:
	cargo test -p cortex-engine --test ingestion_chunking_policy
	cargo test -p cortex-engine --test ingestion_adapters text_chunk_policy_produces_stable_ids_and_long_paragraph_chunks
	cargo test -p cortex-engine --test ingestion_adapters csv_ingestion_writes_one_cell_per_row
	python3 scripts/deterministic_chunking_check.py --report "$(DETERMINISTIC_CHUNKING_REPORT)"

chunking-quality-benchmark-check: deterministic-chunking-check
	python3 scripts/chunking_quality_benchmark.py --settings "$(CHUNKING_QUALITY_SETTINGS)" --report "$(CHUNKING_QUALITY_REPORT)"

pdf-digital-adapter-check:
	cargo test -p cortex-engine --test ingestion_pdf_contracts
	cargo test -p cortex-engine --test ingestion_pdf_digital

ocr-adapter-trait-check:
	cargo test -p cortex-engine --test ingestion_pdf_contracts

dashboard-smoke: dashboard-check
	cargo build -p cortex-server
	npm ci
	@if [ -n "$$CI" ]; then npx playwright install --with-deps chromium; else npx playwright install chromium; fi
	npm run dashboard:smoke

dashboard-screenshots: dashboard-check
	cargo build -p cortex-server
	npm ci
	@if [ -n "$$CI" ]; then npx playwright install --with-deps chromium; else npx playwright install chromium; fi
	npm run dashboard:screenshots
