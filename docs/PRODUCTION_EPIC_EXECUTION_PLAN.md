# CortexDB Production Epic Execution Plan

Source: `/mnt/hf_model_weights/arman/3bit/sites/pl.md`

Status model:

- `todo` means not started in this execution plan.
- `partial` means some repo evidence exists, but the epic is not closed by the acceptance gate.
- `done` means the epic has direct evidence and repeatable checks.
- `blocked` means external conditions are required.

Execution rule: close epics in order inside a block unless a later epic is a direct prerequisite for the current one.

## Current Boundary

CortexDB is currently evidence-backed for Beta Foundation and local single-node production boundary evidence. This plan tracks the next layer: production-grade single-node maturity, stronger security, benchmark history, packaging, operational UX, and future agent-native product layers.

## Current Progress Snapshot

- Done: 119 / 150
- Partial: 1 / 150
- Todo: 30 / 150
- Current closed epic: Epic 119, Incident View
- Long-running partial: Epic 12, 72h storage soak evidence accumulation

## Recommended Order

1. Release governance and public claims.
2. Core engine stability and API compatibility.
3. Storage durability and backup/restore safety.
4. Observability and operations.
5. Packaging, install, and deployment.
6. Security and access control.
7. Retrieval, ContextPack, and Verification quality.
8. Ingestion and provenance.
9. Dashboard and product demos.
10. Advanced agent-native layers.

## A. Release Governance And Production Claims

Acceptance: public release cannot be published without a passing claims gate, evidence bundle, release manifest, and version compatibility notes.

### Epic 1. Production Claims Governance v2

Status: done

Evidence:

- `docs/PUBLIC_CLAIMS_POLICY.md`
- `docs/PUBLIC_CLAIMS_FREEZE.md`
- `scripts/check_public_claims.py`
- `make public-claims-check`

Tasks:

- Strengthen `PUBLIC_CLAIMS_POLICY`.
- Forbid overclaims in README, docs, dashboard, and release notes.
- Separate Alpha, Beta, Production, and Distributed Experimental language.
- Add release-note claim checks.

### Epic 2. Public Beta Release Definition

Status: done

Evidence:

- `docs/BETA_RELEASE.md`
- `docs/BETA_DELTA.md`
- `scripts/beta_release_bundle.py`
- `make beta-release-check`

Tasks:

- Create or update `docs/BETA_RELEASE.md`.
- Define `v0.2.0-beta.1`.
- Mark stable, experimental, and out-of-scope surfaces.
- Synchronize README wording.

### Epic 3. Production v1 Boundary Hardening

Status: done

Evidence:

- `docs/PRODUCTION_V1.md`
- `docs/PRODUCTION_V1_EVIDENCE.md`
- `scripts/production_v1_check.py`
- `make production-v1-check`

Tasks:

- Strengthen `docs/PRODUCTION_V1.md`.
- Add explicit local-only wording.
- Add examples of supported and unsupported use.

### Epic 4. Release Evidence Bundle v2

Status: done

Evidence:

- `scripts/release_evidence_bundle.py`
- `make release-evidence-bundle-check`
- `target/release-evidence-bundle/manifest.json`
- `target/release-evidence-bundle/report.json`
- `target/release-evidence-bundle/release-evidence.tar.gz`
- `target/release-evidence-bundle/release-evidence.tar.gz.sha256`
- Latest local bundle report: `status=passed`, `artifact_count=23`

Tasks:

- Create one `release-evidence.tar.gz`.
- Include JSON reports, checksums, benchmark outputs, SDK reports, and security reports.

### Epic 5. Release Manifest Verifier

Status: done

Evidence:

- `docs/RELEASE_ARTIFACT_MANIFEST.md`
- `scripts/release_artifact_manifest_check.py`
- `make release-artifact-manifest-check`
- `make release-artifact-manifest-production-check`
- `target/release-artifact-manifest/manifest.json`
- `target/release-artifact-manifest/report.json`
- Latest production manifest report: `status=passed`, `artifact_count=15`
- Manifest includes release evidence bundle checksum, SDK versions, and 7 storage format versions.

Tasks:

- Create `release-manifest.json`.
- Include binary checksums, evidence bundle checksums, commit hash, OpenAPI version, SDK versions, and storage format versions.
- Add a verifier command or script.

### Epic 6. Release Notes Automation

Status: done

Evidence:

- `docs/CHANGELOG_RULES.md`
- `docs/API_CHANGELOG.md`
- `docs/RELEASE_NOTES_v0.2.0-beta.1.md`
- `scripts/generate_release_notes.py`
- `make release-notes-generate`
- `target/release-notes/generated.md`
- Generated notes include evidence gates, evidence bundle summary, release manifest summary, known limitations, explicit non-goals, and migration notes.

Tasks:

- Generate release notes from evidence.
- Include gates passed and failed.
- Include known limitations.
- Include migration notes.

### Epic 7. Public Claims CI Gate

Status: done

Evidence:

- `scripts/check_public_claims.py`
- `make public-claims-check`
- `.github/workflows/release.yml`

Tasks:

- Add a checker for README, docs, and blog snippets.
- Fail on forbidden claims such as distributed production, cloud-ready, and enterprise-ready.

### Epic 8. Evidence Artifact Retention Policy

Status: done

Evidence:

- `docs/EVIDENCE_ARTIFACT_RETENTION_POLICY.md`
- `docs/EVIDENCE_ARTIFACT_RETENTION_POLICY.json`
- `scripts/evidence_artifact_retention_check.py`
- `make evidence-artifact-retention-check`
- The policy classifies GitHub Release assets, release evidence bundle
  artifacts, release manifest artifacts, and local-only patterns.

Tasks:

- Define which `target/*/report.json` artifacts are published.
- Define which artifacts go into GitHub Release.
- Define which artifacts stay ignored.

### Epic 9. Release Regression Dashboard

Status: done

Evidence:

- `make performance-trend-check`
- `docs/PERFORMANCE_TREND_HISTORY.md`
- Retrieval and ANN regression/history gates already exist for narrower domains.
- `fixtures/release_regression/history/v0.1.0-core-alpha.5/report.json`
- `scripts/release_regression_dashboard.py`
- `make release-regression-dashboard-check`
- `target/release-regression-dashboard/report.json`
- `target/release-regression-dashboard/dashboard.md`
- `docs/RELEASE_REGRESSION_DASHBOARD.md`
- The release regression dashboard compares storage, search, ContextPack,
  Verify, API, and SDK metrics against the previous release fixture.

Tasks:

- Compare release N vs N-1.
- Track storage, search, context, verify, API, and SDK metrics.

### Epic 10. Versioning Policy v1

Status: done

Evidence:

- `docs/API_VERSIONING.md`
- `docs/API_CHANGELOG.md`
- `docs/AQL_CHANGELOG.md`
- `docs/SDK_RELEASE.md`
- `docs/STORAGE_COMPATIBILITY_EVIDENCE.md`
- `docs/VERSIONING_POLICY.md`
- `docs/VERSIONING_POLICY.json`
- `scripts/versioning_policy_check.py`
- `make versioning-policy-check`
- The unified policy covers HTTP API, SDKs, storage formats, and AQL grammar
  with one breaking-change process.

Tasks:

- Define semver policy for API, SDK, storage formats, and AQL grammar.
- Document breaking-change process.

## B. Storage Durability And Data Safety

Acceptance: database has repeated storage evidence, restore fixtures, backup/restore compatibility, corruption behavior, and RPO/RTO docs.

### Epic 11. Storage Soak History v1

Status: done

Evidence:

- `make storage-soak-check`
- `make storage-soak-history-check`
- `make storage-soak-24h-campaign`
- `make storage-soak-24h-evidence-check`
- `target/storage-soak/report.json`
- `target/storage-soak-history/report.json`
- Current retained local history reports `twenty_four_hour_evidence.met=true`,
  `run_count=981`, `total_cycles=19584`, `total_cells_written=979016`, and
  `total_duration_seconds=86476`.

Tasks:

- Run a 24h soak.
- Include write, checkpoint, compact, backup, and restore loops.
- Add kill/restart injections.
- Save a machine-readable report.

### Epic 12. Storage Soak History v2

Status: partial

Evidence:

- `scripts/storage_soak_v2_gate.py`
- `make storage-soak-72h-start`
- `make storage-soak-72h-campaign`
- `make storage-soak-72h-status`
- `make storage-soak-72h-status-report`
- `make storage-soak-72h-watchdog`
- `make storage-soak-campaign-status-check`
- `make storage-soak-72h-evidence-check`
- V2 uses a separate `target/storage-soak-history-v2/` history root and a
  heavier default workload: 50 cycles per run and 100 cells per cycle.

Remaining:

- Run and retain the full 72-hour v2 campaign.
- Verify `target/storage-soak-history-v2/report.json` and
  `target/storage-soak-history-v2/v2-gate.json` with status `passed`.

Tasks:

- Run a 72h soak.
- Use higher cell count and mixed workload.
- Capture metrics trend.
- Add regression threshold.

### Epic 13. Previous Release Restore Fixtures

Status: done

Evidence:

- `fixtures/migration/historical/v0.1.0-core-alpha.5/fixture.json`
- `fixtures/migration/historical/v0.1.0-core-alpha.5/backup/`
- `fixtures/migration/compatibility_matrix_v1.json`
- `make migration-compatibility-check`
- `target/migration-historical-restore/report.json`
- `target/migration-upgrade-matrix-v2/report.json`

Tasks:

- Save a fixture DB from `v0.1.0`.
- Open it with the current binary.
- Validate, read back, and compare results.

### Epic 14. Backup Archive Compatibility

Status: done

Evidence:

- `make storage-compat-check`
- `target/storage-compat/report.json`
- `target/backup-drill/report.json`
- `target/migration-historical-restore/report.json`
- `target/migration-upgrade-matrix-v2/report.json`
- `docs/STORAGE_COMPATIBILITY.md`
- `docs/BACKUP_RESTORE.md`

Tasks:

- Create backup from previous version.
- Restore with current version.
- Validate restored DB.
- Document incompatible boundaries.

### Epic 15. WAL Corruption Matrix v2

Status: done

Evidence:

- `cargo test -p cortex-storage --test wal_tests`
- `cargo test -p cortex-engine --test recovery_modes`
- `cargo test -p cortex-engine --test alpha_matrix`
- `crates/cortex-storage/tests/wal_tests.rs`
- `crates/cortex-engine/tests/recovery_modes.rs`
- `crates/cortex-engine/tests/alpha_matrix.rs`
- `docs/WAL_REPLAY.md`

Tasks:

- Test corrupt header.
- Test corrupt tail.
- Test checksum mismatch.
- Test corrupt section.
- Test partial record.
- Verify strict and best-effort behavior.

### Epic 16. Segment Corruption Matrix v2

Status: done

Evidence:

- `cargo test -p cortex-engine --test corruption_matrix`
- `crates/cortex-engine/tests/corruption_matrix.rs`
- `docs/STORAGE_COMPATIBILITY.md`
- `docs/CRASH_SIMULATION.md`

Tasks:

- Corrupt `.acs`.
- Corrupt `.acb`.
- Corrupt `.aci`.
- Corrupt `.acv`.
- Corrupt `.ach`.
- Document expected validation and repair behavior.

### Epic 17. Manifest Rollback/Fallback Tests

Status: done

Evidence:

- `cargo test -p cortex-engine --test crash_matrix`
- `cargo test -p cortex-storage --test segment_index_tests manifest`
- `crates/cortex-engine/tests/crash_matrix.rs`
- `crates/cortex-storage/tests/segment_index_tests.rs`
- `crates/cortex-engine/src/checkpoint/manifest_safety.rs`
- `docs/CRASH_SIMULATION.md`

Tasks:

- Test old manifest.
- Test missing manifest.
- Test partial manifest.
- Test corrupted manifest.
- Verify fail-safe behavior.

### Epic 18. Compaction Fault Injection v2

Status: done

Evidence:

- `cargo test -p cortex-engine --test crash_matrix`
- `cargo test -p cortex-engine --test bundle_gc`
- `crates/cortex-engine/tests/crash_matrix.rs`
- `crates/cortex-engine/tests/bundle_gc.rs`
- `docs/CRASH_SIMULATION.md`

Tasks:

- Kill before segment write.
- Kill after segment write.
- Kill before manifest update.
- Kill after manifest update.
- Verify retired segment handling.

### Epic 19. Backup Prune Safety

Status: done

Evidence:

- `make backup-drill-check`
- `target/backup-drill/report.json`
- `cargo test -p cortex-engine --test backup_restore backup_retention`
- `cargo test -p cortex-cli backup_prune`
- `crates/cortex-engine/src/backup/retention.rs`
- `scripts/backup_drill_check.sh`
- `docs/BACKUP_RESTORE.md`

Tasks:

- Implement or verify prune latest N.
- Never delete the only valid backup.
- Add dry-run mode.
- Save prune report.

### Epic 20. Restore Dry-run Mode

Status: done

Evidence:

- `cargo test -p cortex-engine --test backup_restore restore_dry_run`
- `cargo test -p cortex-cli cli_restore_dry_run`
- `make backup-drill-check`
- `target/backup-drill/report.json`
- `crates/cortex-engine/src/backup/dry_run.rs`
- `crates/cortex-cli/src/cli.rs`
- `crates/cortex-cli/src/cli_ops.rs`
- `scripts/backup_drill_check.sh`
- `docs/BACKUP_RESTORE.md`
- `docs/CLI.md`

Tasks:

- Inspect archive without writing. Done: `restore_from_backup_dry_run` reads
  backup files and leaves the target path absent.
- Verify checksums. Done: dry-run reads manifest, segment, bitmap, lexical,
  vector, HNSW, and WAL files through storage readers.
- Verify version compatibility. Done: storage readers reject incompatible or
  corrupt formats before restore.
- Estimate restore path. Done: CLI report includes `restore_path=...` and
  refuses existing targets like real restore.

### Epic 21. Backup/Restore RPO/RTO Profiles

Status: done

Evidence:

- `make backup-rpo-rto-profile-check`
- `target/backup-rpo-rto/report.json`
- `make backup-restore-production-pack-check`
- `target/backup-restore-production-pack/report.json`
- `scripts/backup_rpo_rto_profiles.py`
- `scripts/backup_restore_production_pack.py`
- `docs/RPO_RTO.md`
- `docs/BACKUP_RESTORE.md`

Tasks:

- Define small, medium, and large profiles. Done: local profiles are 10, 100,
  and 500 base cells plus a WAL-tail probe.
- Measure backup time. Done: profile report records `backup_duration_ms`.
- Measure restore time. Done: profile report records `restore_duration_ms` and
  `restore_dry_run_duration_ms`.
- Define data-loss boundary. Done: profile gate writes after backup and proves
  the restored copy excludes that post-backup cell.

### Epic 22. Offsite Backup Adapter v1

Status: done

Evidence:

- `cargo test -p cortex-engine --test backup_restore offsite`
- `cargo test -p cortex-cli backup_offsite_stage_command_validates_and_publishes_copy`
- `make backup-offsite-check`
- `target/backup-offsite/report.json`
- `python3 scripts/backup_restore_production_pack.py --backup-drill-report target/backup-drill/report.json --backup-offsite-report target/backup-offsite/report.json --rpo-rto-profile-report target/backup-rpo-rto/report.json --output target/backup-restore-production-pack/report.json`
- `crates/cortex-engine/src/backup/offsite.rs`
- `scripts/backup_offsite_check.sh`
- `docs/BACKUP_RESTORE.md`

Tasks:

- Add local filesystem adapter. Done: `LocalFilesystemOffsiteAdapter`.
- Add adapter trait. Done: `OffsiteBackupAdapter`.
- Validate checksums. Done: staged copy validation reads persisted files through
  storage readers before publish and production-pack requires this evidence.
- Simulate staged upload. Done: offsite gate reports
  `staged_upload_simulated=true` and `published=true` for the local filesystem
  adapter.

### Epic 23. Encrypted Backup MVP

Status: done

Tasks:

- Add passphrase encryption. Done: `backup-encrypted`,
  `restore-encrypted`, and `Database::encrypted_backup_path` create and restore
  CortexDB-local passphrase archives.
- Add key derivation. Done for MVP: archive metadata records
  `cortexdb.fnv64-passphrase.v1` as the local deterministic KDF boundary.
- Create encrypted archive. Done: `make encrypted-backup-check` writes
  `target/encrypted-backup/backup.cdbenc`.
- Validate restore. Done: the evidence gate restores checkpointed and WAL-tail
  data and validates the restored database.
- Fail safely on wrong key. Done: wrong-passphrase and corrupt-ciphertext
  restores fail without creating the target database.

Evidence:

- `make encrypted-backup-check`
- `target/encrypted-backup/report.json`
- `make backup-restore-production-pack-check`
- `target/backup-restore-production-pack/report.json`
- `scripts/encrypted_backup_check.py`
- `scripts/backup_restore_production_pack.py`
- `docs/ENCRYPTED_BACKUPS_DESIGN.md`
- `docs/BACKUP_RESTORE.md`

Boundary:

- This is an encrypted backup MVP for local release evidence. It is not
  KMS-backed, externally audited, or compliance-certified encryption.

### Epic 24. Encrypted Backup Rotation

Status: done

Tasks:

- Define key rotation policy. Done: current MVP uses archive-scoped
  passphrase rotation. New backups are created with the new passphrase; old
  backups remain decryptable only with the old passphrase until retention
  expiry.
- Verify old backup decrypt. Done: `make encrypted-backup-rotation-check`
  restores the old archive with the old passphrase.
- Verify new backup encrypt. Done: the same gate creates a new archive with the
  new passphrase and restores current data from it.
- Document rotation flow. Done:
  `docs/ENCRYPTED_BACKUPS_DESIGN.md` and `docs/BACKUP_RESTORE.md`.

Evidence:

- `make encrypted-backup-rotation-check`
- `target/encrypted-backup-rotation/report.json`
- `make backup-restore-production-pack-check`
- `target/backup-restore-production-pack/report.json`
- `scripts/encrypted_backup_rotation_check.py`
- `scripts/backup_restore_pack_validators.py`
- `scripts/backup_restore_production_pack.py`
- `docs/ENCRYPTED_BACKUPS_DESIGN.md`
- `docs/BACKUP_RESTORE.md`

Boundary:

- This closes passphrase-rotation evidence for the encrypted-backup MVP. It is
  not KMS-backed key rotation or compliance-grade custody.

### Epic 25. Storage Format Freeze v1

Status: done

Tasks:

- Freeze ACLOG v1. Done: `storage-format-freeze-v1` freezes the current
  `ACLOGv0\0` marker and `WAL_FORMAT_VERSION = 0` as the first compatibility
  contract for WAL.
- Freeze ACS v1. Done: `.acs` is frozen at `ACS1`.
- Freeze ACB v1. Done: `.acb` is frozen at `ACB0`.
- Freeze ACI v1. Done: `.aci` is frozen at `ACI2` with read-only compatibility
  for `ACI0` and `ACI1`.
- Freeze ACV v1. Done: `.acv` is frozen at `ACV0`.
- Freeze ACH v1. Done: `.ach` is frozen at `ACH0`.
- Freeze manifest v1. Done: `.acm` is frozen at `ACM0`.
- Add compatibility docs. Done: `STORAGE_FORMATS.md`,
  `STORAGE_COMPATIBILITY.md`, and `UPGRADE_MIGRATION.md` describe the
  freeze-v1 contract and evidence gate.

Evidence:

- `fixtures/storage/storage_format_freeze_v1.json`
- `make storage-format-freeze-check`
- `target/storage-format-freeze/report.json`
- `make storage-compat-check`
- `target/storage-compat/report.json`
- `scripts/storage_format_freeze_check.py`
- `scripts/storage_compat_check.py`
- `docs/STORAGE_FORMATS.md`
- `docs/STORAGE_COMPATIBILITY.md`
- `docs/UPGRADE_MIGRATION.md`

Boundary:

- Freeze v1 is a compatibility contract over the current markers. It does not
  renumber every existing magic to `v1`, and it does not prove online rolling
  upgrade or in-place downgrade.

## C. Core Engine Stability

Acceptance: embedded users get stable API; server, SDK, and CLI share consistent errors; no public core path panics.

### Epic 26. Engine Public API Freeze

Status: done

Tasks:

- Document stable public APIs. Done: `ENGINE_API.md` now lists the frozen
  crate-root facade symbols and points to the machine-readable freeze contract.
- Hide internal APIs. Done: the freeze gate checks that `cleanup`,
  `database_files`, `lock`, and `options` remain private helper modules.
- Ensure examples compile. Done: `public_api.rs` covers the stable root facade,
  and `engine-api-check` runs the compile test.
- Add rustdoc examples. Done: crate-level `cortex-engine` docs and existing
  `Database` docs compile through engine doctests.

Evidence:

- `fixtures/engine/public_api_freeze_v1.json`
- `make engine-public-api-freeze-check`
- `make engine-api-check`
- `target/engine-api/report.json`
- `crates/cortex-engine/tests/public_api.rs`
- `crates/cortex-engine/src/lib.rs`
- `docs/ENGINE_API.md`
- `docs/ENGINE_API_EVIDENCE.md`
- `docs/MODULE_OWNERSHIP.md`

Boundary:

- This freezes the embedded Rust crate-root facade for current local users. It
  does not freeze C ABI, HTTP API, SDK package versions, or every experimental
  implementation module.

### Epic 27. Engine API Compatibility Tests

Status: done

Tasks:

- Compile an external sample crate. Done: `examples/engine_api_compat` opts out
  of the workspace and compiles through `make engine-api-compat-check`.
- Test `Database::open`. Done: the sample opens a local database and later opens
  a restored database with explicit options.
- Test put/get/search/context/verify/backup. Done: the sample runs `put_cell`,
  `get_latest_cell`, `search_keyword`, `context_pack_from_aql`,
  `verify_fact_aql`, `checkpoint`, `Database::backup_path`, and
  `Database::restore_from_backup`.

Evidence:

- `examples/engine_api_compat/Cargo.toml`
- `examples/engine_api_compat/src/main.rs`
- `scripts/engine_api_compat_check.py`
- `make engine-api-compat-check`
- `make engine-api-check`
- `target/engine-api-compat/report.json`
- `docs/ENGINE_API_COMPATIBILITY.md`
- `docs/ENGINE_API.md`

Boundary:

- This proves source-level compatibility for an external local path dependency.
  Published crate SemVer, remote registry packaging, HTTP API compatibility, and
  SDK package lifecycle remain separate epics.

### Epic 28. Engine Error Model v1

Status: done

Tasks:

- Stabilize public error enum. Done: `EngineError` now exposes stable
  `EngineErrorCode`, `EngineErrorCategory`, `http_status`, `safe_message`, and
  `cli_hint` metadata.
- Map errors to HTTP, CLI, and SDK. Done: server routing maps through
  `EngineError::code()`, CLI hints use `EngineError::cli_hint()`, and
  `engine-error-model-check` verifies SDK-visible code coverage.
- Remove ad-hoc public error strings. Done: CLI engine errors no longer match
  individual variants for hints; the engine owns the user-facing hint policy.

Evidence:

- `crates/cortex-engine/src/error.rs`
- `crates/cortex-engine/tests/error_model.rs`
- `fixtures/engine/error_model_v1.json`
- `scripts/engine_error_model_check.py`
- `make engine-error-model-check`
- `make engine-api-check`
- `make openapi-contract-check`
- `target/engine-error-model/report.json`
- `docs/ENGINE_ERROR_MODEL.md`
- `docs/API_ERROR_TAXONOMY.md`

Boundary:

- The model freezes engine-level classification and adapter mapping. It does
  not freeze every free-form human-readable message forever; messages may
  become clearer while stable codes/categories remain compatible.

### Epic 29. Engine Feature Flags

Status: done

Tasks:

- Separate experimental HNSW, replication, and dashboard features. Done:
  `EngineFeatureFlags` exposes explicit `experimental_hnsw`,
  `experimental_replication`, and `dashboard` flags. HNSW graph persistence and
  database-level replication snapshot/install require explicit opt-in, while
  dashboard routes require `CORTEXDB_DASHBOARD=true` on the server surface.
- Keep production-safe defaults. Done: `DatabaseOptions::default()` uses
  `EngineFeatureFlags::production_safe()`, so new databases skip `.ach` graph
  creation and vector search uses exact persisted vector fallback unless HNSW
  is enabled.

Evidence:

- `crates/cortex-engine/tests/feature_flags.rs`
- `crates/cortex-server/src/dashboard_tests.rs`
- `docs/ENGINE_FEATURE_FLAGS.md`
- `make engine-feature-flags-check`

Boundary:

- Consensus primitives remain available for local tests and design work. The
  gated surface is the database-level replication snapshot/install path.

### Epic 30. Engine Module Ownership

Status: done

Tasks:

- Maintain `MODULE_OWNERSHIP.md`. Done: the document now defines stable
  facades, required ownership areas, top-level `cortex-engine` module owners,
  cross-crate boundaries, and review checklists.
- Define owners for storage, search, context, verify, ingestion, and server.
  Done: the required ownership matrix covers storage, search, context, verify,
  ingestion, server, CLI, and SDK owners with required gates.

Evidence:

- `docs/MODULE_OWNERSHIP.md`
- `scripts/module_ownership_check.py`
- `make module-ownership-check`
- `make engine-api-check`

Boundary:

- This epic documents and gates module ownership. It does not yet enforce Rust
  visibility or dependency boundaries; that is Epic 31.

### Epic 31. Engine Internal Boundary Audit

Status: done

Evidence:

- `docs/ENGINE_INTERNAL_BOUNDARIES.md`
- `scripts/engine_internal_boundary_check.py`
- `make engine-internal-boundary-check`
- `make engine-api-check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Mark internal modules. Done: `ENGINE_INTERNAL_BOUNDARIES.md` defines the
  crate-root facade rule and points to the existing module ownership map.
- Prevent SDK/server from depending on internal details. Done: server and CLI
  imports now use root facade re-exports, and the boundary gate rejects
  `cortex_engine::<known_module>::...` references in server/SDK paths.

Boundary:

- This gate enforces external import discipline by scanning server/SDK code.
  It does not make every engine module private yet; that larger compatibility
  migration remains a future API-design task.

### Epic 32. Engine Determinism Audit

Status: done

Evidence:

- `docs/ENGINE_DETERMINISM.md`
- `scripts/engine_determinism_check.py`
- `make engine-determinism-check`
- `make engine-api-check`
- `cargo test -p cortex-engine --test determinism`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Verify deterministic ordering for search, context, and verify. Done:
  `tests/determinism.rs` repeats public calls before and after checkpoint and
  asserts stable search, ContextPack, and VerificationReport output order.
- Add snapshot tests. Done: the determinism test suite uses canonical string
  snapshots for search result order, ContextPack cell/anomaly order, evidence
  order, guard order, and numeric conflict order.
- Avoid nondeterministic maps in public output. Done:
  `engine_determinism_check.py` fails if public search/context/verify response
  paths introduce `HashMap` or `HashSet`.

### Epic 33. Engine Memory Accounting

Status: done

Evidence:

- `crates/cortex-engine/src/memory_accounting.rs`
- `crates/cortex-engine/tests/storage_stats.rs`
- `crates/cortex-server/src/responses.rs`
- `docs/openapi.yaml`
- `docs/METRICS.md`
- `cargo test -p cortex-engine --test storage_stats`
- `cargo test -p cortex-cli`
- `make openapi-contract-check`
- `make sdk-contract-check`
- `make observability-check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Estimate MemTable memory. Done: `storage_stats()` now reports raw
  MemTable payload bytes and estimated MemTable structure+payload bytes.
- Estimate index memory. Done: `memory_accounting.rs` estimates current AQL
  bitmap, lexical, candidate, and frequency map memory.
- Estimate ContextPack memory. Done: stats include a ContextPack working-set
  estimate for visible cells.
- Expose stats. Done: Rust `StorageStats`, CLI JSON/plain stats, HTTP
  `/v1/stats`, `/v1/metrics`, Prometheus metrics, SDK `StatsResponse`,
  dashboard cards, OpenAPI, API docs, and observability docs include the new
  memory accounting fields.

### Epic 34. Engine Config Model

Status: done

Evidence:

- `crates/cortex-engine/src/config.rs`
- `docs/ENGINE_CONFIG.md`
- `crates/cortex-engine/tests/public_api.rs`
- `cargo test -p cortex-engine config`
- `cargo test -p cortex-engine --test public_api`
- `cargo test -p cortex-cli`
- `cargo test -p cortex-server`
- `make engine-api-check`
- `make engine-feature-flags-check`
- `make module-ownership-check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Add formal config struct. Done: `EngineConfig` and `EngineConfigError`
  are exported from the `cortex-engine` crate-root facade and covered by the
  public API freeze fixture.
- Add env loading. Done: `EngineConfig::from_env()` and
  `EngineConfig::from_env_vars(...)` parse durability, recovery, stale lock,
  HNSW profile, experimental HNSW, experimental replication, and dashboard
  values with strict invalid-value errors.
- Align CLI/server config. Done: CLI database opens and server actor opens use
  the same `EngineConfig`/`DatabaseOptions` path, with command flags allowed to
  opt into HNSW without disabling env-enabled features.
- Document config. Done: `ENGINE_CONFIG.md`, `ENGINE_API.md`,
  `ENGINE_FEATURE_FLAGS.md`, module ownership docs, and engine API fixtures now
  describe the shared config boundary.

### Epic 35. Engine Panic Audit

Status: done

Evidence:

- `docs/ENGINE_PANIC_AUDIT.md`
- `scripts/engine_panic_audit_check.py`
- `crates/cortex-storage/src/segment.rs`
- `crates/cortex-storage/src/indexes.rs`
- `crates/cortex-storage/src/vectors.rs`
- `crates/cortex-storage/src/manifest.rs`
- `crates/cortex-storage/src/wal/codec.rs`
- `crates/cortex-storage/src/wal/writer.rs`
- `crates/cortex-engine/src/graph.rs`
- `crates/cortex-engine/tests/graph_tests.rs`
- `make engine-panic-audit-check`
- `make engine-api-check`
- `cargo test -p cortex-storage`
- `cargo test -p cortex-engine --test graph_tests`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Search for `unwrap`, `expect`, and `panic`. Done:
  `engine_panic_audit_check.py` scans production `cortex-core`,
  `cortex-engine`, and `cortex-storage` sources while excluding docs, tests,
  and `src/bin` tooling.
- Replace in core paths. Done: storage fixed-width binary readers, WAL codec
  helpers, WAL writer append paths, and tool-cell name extraction no longer use
  `unwrap`/`expect` in production paths.
- Add regression tests. Done: graph tool-cell tests cover missing tool names,
  and existing storage corruption/roundtrip tests cover the fallible binary
  readers and WAL decode paths.

## D. AQL / Query Layer

Acceptance: AQL becomes stable contract, explainable, safe, and tested across SDK/API.

### Epic 36. AQL v0.4 Compatibility Pack

Status: done

Evidence:

- `docs/AQL_V0_4.md`
- `docs/AQL_COMPATIBILITY.md`
- `docs/AQL_CHANGELOG.md`
- `scripts/aql_compat_check.py`
- `crates/cortex-aql/tests/aql_v0_4_golden_tests.rs`
- `crates/cortex-aql/tests/parser_tests.rs`
- `crates/cortex-aql/tests/binder_hardening_tests.rs`
- `crates/cortex-aql/tests/aql_stabilization_tests.rs`
- `make aql-compat-check`
- `target/aql-compat/report.json`
- `cargo test -p cortex-aql`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Add golden parser tests. Done: `aql_v0_4_golden_tests.rs` freezes
  `RETRIEVE CONTEXT`, `EXPLAIN RETRIEVE CONTEXT`, `VERIFY FACT`, `REMEMBER`,
  requirements, list literals, and parsed AST shape.
- Add golden binder tests. Done: the same golden pack freezes bound retrieve
  plan fields, policy clamps, quality thresholds, and bitmap bytecode.
- Add malformed query tests. Done: `parser_tests.rs` and the compatibility
  gate cover invalid modes, malformed syntax, huge integers, multiline
  diagnostics, and safe parse messages.
- Add permission denial tests. Done: golden and binder hardening tests cover
  unreadable scopes, safe bind messages, and HTTP `permission_denied` mapping.
- Add unknown field tests. Done: golden tests cover `FieldNotFilterable`, and
  `AQL_COMPATIBILITY.md` documents unknown-field client behavior.

### Epic 37. AQL Explain

Status: done

Evidence:

- `crates/cortex-engine/src/query/explain.rs`
- `crates/cortex-server/src/aql.rs`
- `crates/cortex-cli/src/cli_aql.rs`
- `crates/cortex-cli/src/cli_aql_json.rs`
- `crates/cortex-sdk/src/types.rs`
- `docs/openapi.yaml`
- `docs/API_JSON_SCHEMAS.md`
- `docs/AQL_COMPATIBILITY.md`
- `docs/AQL_COMPATIBILITY_EVIDENCE.md`
- `docs/AQL_V0_4.md`
- `scripts/check_openapi_contract.py`
- `cargo test -p cortex-engine --test query_search explain_retrieve_aql_reports_plan_filters_counts_and_mode`
- `cargo test -p cortex-server aql_explain`
- `cargo test -p cortex-cli aql_command_explain_reports_plan_filters_counts_and_mode`
- `cargo test -p cortex-sdk typed_aql_explain_response_decodes_contract`
- `make aql-compat-check`
- `make openapi-contract-check`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

Tasks:

- Implement `EXPLAIN RETRIEVE`. Done: `Database::explain_retrieve_aql`
  executes `EXPLAIN RETRIEVE CONTEXT` through the same parse/bind/index path
  as retrieval and returns a typed explain report.
- Output bitmap plan. Done: explain returns `bitmap_plan` and `bitmap_ops`
  from the bound bitmap program.
- Output filters. Done: explain returns policy, liveness, and rendered `WHERE`
  filters.
- Output candidate counts. Done: explain returns universe, agent-allowed,
  live, after-bitmap, after-quality, and returned-limit counts.
- Output selected retrieval mode. Done: engine, HTTP, CLI JSON, and SDK expose
  `selected_mode`.

### Epic 38. AQL Error Taxonomy

Status: done

Evidence:

- `crates/cortex-engine/src/error.rs`
- `crates/cortex-server/src/responses.rs`
- `crates/cortex-sdk/src/types.rs`
- `crates/cortex-engine/tests/error_model.rs`
- `crates/cortex-server/src/tests/error_taxonomy_tests.rs`
- `crates/cortex-server/src/tests/snapshot_api_tests.rs`
- `crates/cortex-server/src/tests/error_response_snapshot_tests.rs`
- `crates/cortex-sdk/src/tests.rs`
- `scripts/aql_compat_check.py`
- `scripts/check_error_taxonomy_contract.py`
- `docs/API_ERROR_TAXONOMY.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/API.md`
- `docs/ENGINE_ERROR_MODEL.md`
- `docs/openapi.yaml`
- `cargo test -p cortex-engine --test error_model`
- `cargo test -p cortex-server error_taxonomy`
- `cargo test -p cortex-server snapshot_all_sdk_visible_error_responses`
- `cargo test -p cortex-sdk error_code_decodes_full_core_alpha_taxonomy`
- `make aql-compat-check`
- `make openapi-contract-check`

Tasks:

- Add `invalid_aql`. Done: existing parser/generic bind class remains stable
  through engine, HTTP, OpenAPI, and SDK.
- Add `permission_denied`. Done: policy-denied AQL bind failures remain
  `403 permission_denied`.
- Add `unknown_field`. Done: `BindError::FieldNotFilterable` now maps to
  `400 unknown_field`.
- Add `unsupported_operator`. Done: `BindError::UnsupportedComparator` now maps
  to `400 unsupported_operator`.
- Map errors to HTTP and SDK. Done: engine codes, router errors, OpenAPI enum,
  server snapshots, Rust SDK enum/tests, and compatibility gates are aligned.

### Epic 39. AQL Compatibility Changelog

Status: done

Evidence:

- `fixtures/aql/grammar_change_registry_v1.json`
- `docs/AQL_CHANGELOG.md`
- `scripts/check_aql_changelog_policy.py`
- `make aql-changelog-policy-check`
- `make aql-compat-check`

Tasks:

- Require changelog for every grammar change. Done: the AQL grammar-change
  registry requires a stable `change_id`, changelog anchor, SQL example, and
  test reference for each grammar or binder compatibility change.
- Add examples for grammar changes. Done: `docs/AQL_CHANGELOG.md` now lists
  runnable SQL examples for the current AQL v0.4 grammar and diagnostic
  compatibility entries, and the checker verifies them.

### Epic 40. AQL Query Cache

Status: done

Evidence:

- `crates/cortex-engine/src/query/cache.rs`
- `Database::aql_query_cache_stats`
- `crates/cortex-engine/tests/aql_query_cache.rs`
- `crates/cortex-engine/tests/public_api.rs`
- `fixtures/engine/public_api_freeze_v1.json`
- `cargo test -p cortex-engine --test aql_query_cache`
- `cargo test -p cortex-engine --test public_api`
- `make engine-api-check`

Tasks:

- Add parse/bind cache. Done: `Database::bind_aql_cached` caches owned
  `BoundPlan` values after parse+bind misses and reuses them for repeated
  `RETRIEVE`, `EXPLAIN RETRIEVE`, `VERIFY FACT`, and `REMEMBER` calls.
- Invalidate by AgentView/catalog version. Done: cache keys include an
  `AgentView` fingerprint and the cache clears entries when the catalog
  fingerprint changes through commit sequence or manifest/live-segment changes.

### Epic 41. AQL Require Semantics v1

Status: done

Evidence:

- `docs/AQL_REQUIRE_SEMANTICS.md`
- `docs/AQL_V0_4.md`
- `crates/cortex-engine/src/context/mod.rs`
- `crates/cortex-engine/src/context/pack.rs`
- `crates/cortex-engine/tests/aql_require_semantics.rs`
- `cargo test -p cortex-engine --test aql_require_semantics`

Tasks:

- Formalize `REQUIRE citations`. Done: AQL-bound citation policy now reaches
  `ContextPack.citations_required` and missing-citation anomalies.
- Formalize confidence. Done: confidence requirements are documented as hard
  candidate filters using SourceRef confidence, source-trust fallback, or `0`.
- Formalize source trust. Done: source-trust requirements are documented and
  tested as hard filters over `source_trust_q16`.
- Formalize freshness. Done: freshness requirements are documented and tested
  as query-time age filters over `created_unix_seconds`.

### Epic 42. AQL Limit/Budget Semantics

Status: done

Evidence:

- `docs/AQL_LIMIT_BUDGET_SEMANTICS.md`
- `docs/AQL_V0_4.md`
- `crates/cortex-engine/src/context/pack.rs`
- `crates/cortex-engine/tests/aql_limit_budget_semantics.rs`
- `cargo test -p cortex-engine --test aql_limit_budget_semantics`

Tasks:

- Clarify candidate limit. Done: effective `LIMIT ... CANDIDATES` is a
  policy-clamped hard upper bound for `retrieve_aql` and ContextPack input.
- Clarify ContextPack cell limit. Done: v1 has no separate AQL final cell
  limit; packed cells are bounded by candidate limit, token budget, and
  optional redundancy reduction.
- Clarify token budget. Done: `BUDGET ... TOKENS` is the default ContextPack
  budget for AQL calls, while explicit `ContextPackOptions.token_budget_tokens`
  can deliberately override it and remains AgentView-clamped.

### Epic 43. AQL Security Fuzzing

Status: done

Evidence:

- `docs/AQL_SECURITY_FUZZING.md`
- `docs/AQL_V0_4.md`
- `crates/cortex-engine/tests/aql_security_fuzzing.rs`
- `cargo test -p cortex-engine --test aql_security_fuzzing`

Tasks:

- Generate random WHERE/NOT/AND/OR queries. Done: the test builds a
  fixed-seed deterministic corpus of nested predicate, `NOT`, `AND`, and `OR`
  expressions plus hand-written edge cases.
- Verify no scope bypass. Done: each generated query either returns only cells
  from the readable scope or fails closed with `permission_denied`; the same
  corpus runs before and after checkpoint/reopen.

### Epic 44. AQL SDK Helpers

Status: done

Evidence:

- Rust SDK AQL helpers: `crates/cortex-sdk/src/aql.rs`,
  `crates/cortex-sdk/src/aql_support.rs`.
- Python SDK AQL helpers: `sdk/python/cortexdb_client.py`.
- TypeScript SDK AQL helpers:
  `sdk/typescript/cortexdb-client.ts`,
  `sdk/typescript/cortexdb-client.js`,
  `sdk/typescript/cortexdb-client.cjs`,
  `sdk/typescript/cortexdb-client.d.ts`.
- Usage docs/examples:
  `docs/SDK_QUICKSTART.md`,
  `sdk/README.md`,
  `sdk/python/README.md`,
  `sdk/typescript/README.md`,
  `sdk/python/examples/basic.py`,
  `sdk/typescript/examples/basic.mjs`.
- Checks:
  `cargo test -p cortex-sdk`,
  `python3 sdk/python/test_cortexdb_client.py`,
  `node sdk/typescript/test.js`,
  `make sdk-contract-check`,
  `make sdk-check`,
  `cargo fmt --check`,
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `git diff --check`.

Tasks:

- Add SDK builder methods for retrieve. Done: Rust, Python, and TypeScript
  expose builder helpers for `RETRIEVE CONTEXT`.
- Add SDK builder methods for verify. Done: Rust, Python, and TypeScript
  expose builder helpers for `VERIFY FACT`.
- Add SDK builder methods for remember. Done: Rust, Python, and TypeScript
  expose builder helpers for `REMEMBER`.
- Reduce string-only query usage. Done: SDK quickstarts, README snippets, live
  contract example, and Python/TypeScript basic examples use helpers for common
  AQL statements.

### Epic 45. AQL Query Examples Pack

Status: done

Evidence:

- Domain example index: `examples/aql/README.md`.
- Investment project examples: `examples/aql/investment_projects/*.aql`.
- Legal policy examples: `examples/aql/legal_policies/*.aql`.
- Support ticket examples: `examples/aql/support_tickets/*.aql`.
- Technical docs examples: `examples/aql/technical_docs/*.aql`.
- Parser regression gate:
  `crates/cortex-aql/tests/aql_examples_pack.rs`.
- Checks:
  `cargo test -p cortex-aql --test aql_examples_pack`.

Tasks:

- Add investment examples. Done: retrieve, explain, verify, and remember
  examples for project finance and budget-risk workflows.
- Add legal examples. Done: retrieve, explain, verify, and remember examples
  for policy lookup and human-review workflows.
- Add support examples. Done: retrieve, explain, verify, and remember examples
  for incident triage and support remediation workflows.
- Add technical docs examples. Done: retrieve, explain, verify, and remember
  examples for SDK contracts and storage runbooks.

## E. Retrieval, Search, ANN

Acceptance: retrieval is measurable, explainable, multi-domain, and has official benchmark adapters.

### Epic 46. Multi-domain Retrieval Corpus v2

Status: done

Evidence:

- Investment baseline corpus:
  `examples/real_domains/investment_projects/corpus/documents.jsonl`,
  `examples/real_domains/investment_projects/corpus/chunks.jsonl`,
  `examples/real_domains/investment_projects/queries/queries.jsonl`,
  `examples/real_domains/investment_projects/queries/ground_truth.jsonl`.
- Legal policies corpus:
  `examples/real_domains/legal_policies/corpus/documents.jsonl`,
  `examples/real_domains/legal_policies/corpus/chunks.jsonl`,
  `examples/real_domains/legal_policies/queries/queries.jsonl`,
  `examples/real_domains/legal_policies/queries/ground_truth.jsonl`.
- Support tickets corpus:
  `examples/real_domains/support_tickets/corpus/documents.jsonl`,
  `examples/real_domains/support_tickets/corpus/chunks.jsonl`,
  `examples/real_domains/support_tickets/queries/queries.jsonl`,
  `examples/real_domains/support_tickets/queries/ground_truth.jsonl`.
- Technical docs corpus:
  `examples/real_domains/technical_docs/corpus/documents.jsonl`,
  `examples/real_domains/technical_docs/corpus/chunks.jsonl`,
  `examples/real_domains/technical_docs/queries/queries.jsonl`,
  `examples/real_domains/technical_docs/queries/ground_truth.jsonl`.
- Validation/report gate:
  `python3 scripts/retrieval_beta_report.py --domain-root examples/real_domains --output target/retrieval/retrieval_beta_report_epic46.json --min-domains 4 --repeat-runs 5`.
- Result: 4 domains, 76 documents, 205 chunks, 70 queries, 70 ground-truth
  rows, `production_safe=true`.

Tasks:

- Add `legal_policies`. Done: documents, chunks, queries, ground truth, source
  registry, README, and validators exist and pass.
- Add `support_tickets`. Done: documents, chunks, queries, ground truth, source
  registry, README, and validators exist and pass.
- Add `technical_docs`. Done: documents, chunks, queries, ground truth, source
  registry, README, and validators exist and pass.
- Add corpus, chunks, queries, and ground truth. Done: four retrieval domains
  are discoverable by `scripts/retrieval_beta_report.py` and the repeated local
  lexical probe reports no regressions.

### Epic 47. Retrieval Quality History

Status: done

Evidence:

- `scripts/retrieval_quality_history.py`
- `scripts/retrieval_quality_history_self_test.py`
- `make retrieval-quality-history-check`
- `target/retrieval-quality/history.json`
- Latest local history report: `status=passed`, `production_safe=true`,
  `domain_count=4`, `history_runs_per_domain=5`, `run_count=20`, and
  `regression_count=0`.

Tasks:

- Run repeated evaluations per domain. Done: the history gate evaluates four
  checked-in retrieval domains five times each.
- Track recall. Done: each domain summary records `latest_mean_recall_q16`.
- Track MRR. Done: each domain summary records `latest_mean_mrr_q16`.
- Track nDCG. Done: each domain summary records `latest_mean_ndcg_q16`.
- Track p95/p99 latency. Done: each domain summary records
  `latest_p95_latency_nanos` and `latest_p99_latency_nanos`.
- Add no-regression report. Done: adjacent runs fail the gate on quality,
  exact-parity, or latency regressions beyond configured local tolerances.

### Epic 48. Public Retrieval Benchmark Page

Status: done

Evidence:

- `docs/PUBLIC_RETRIEVAL_BENCHMARKS.md`
- `scripts/public_retrieval_benchmark_check.py`
- `make public-retrieval-benchmark-page-check`
- `make public-benchmarks-check`
- `target/public-retrieval-benchmarks/report.json`
- Latest local page report: `status=passed`, `domain_count=4`,
  `run_count=20`, `regression_count=0`, and totals of 76 documents, 205
  chunks, 70 queries, and 70 ground-truth rows.

Tasks:

- Publish metrics table. Done: the page publishes per-domain recall, MRR,
  nDCG, run count, and latency-field coverage.
- Explain dataset size. Done: the page lists documents, chunks, queries, and
  ground-truth rows per domain plus totals.
- Explain exact vs ANN. Done: the page separates deterministic lexical fixture,
  exact vector fallback, guarded ANN/HNSW, and endpoint-backed embedding
  evidence.
- Document limitations. Done: the page explicitly excludes production SLA,
  hosted embedding CI, private customer corpus quality, fallback-free production
  HNSW, leaderboard placement, and legal/financial correctness claims.

### Epic 49. LongMemEval Retrieval Adapter

Status: done

Evidence:

- `scripts/longmemeval/check_v1_retrieval_adapter.py`
- `make longmemeval-v1-retrieval-adapter-check`
- `target/longmemeval-v1/data/manifest.json`
- `target/longmemeval-v1/cortexdb/longmemeval_s_cleaned_cortexdb_session_retrieval.jsonl`
- `target/longmemeval-v1/cortexdb/official_retrieval_metrics.txt`
- `target/longmemeval-v1/cortexdb/report.json`
- `target/longmemeval-v1/retrieval-adapter/report.json`
- Latest local adapter report: `status=passed`, `retrieval_log_rows=500`,
  official `session recall_all@10=0.9021`, official
  `session ndcg_any@10=0.7873`.

Tasks:

- Ingest LongMemEval-S. Done: `make longmemeval-v1-official-data`
  downloads the official `xiaowu0162/longmemeval-cleaned` small split and
  writes a manifest with 500 rows and dataset checksum.
- Produce official retrieval log. Done: `make longmemeval-v1-cortexdb-retrieval`
  builds the CLI, loads per-question CortexDB fixtures, and writes the
  session-level retrieval JSONL.
- Store official metrics. Done: `make longmemeval-v1-official-retrieval-metrics`
  runs the official LongMemEval `print_retrieval_metrics.py`, and
  `make longmemeval-v1-retrieval-adapter-check` stores the validated adapter
  report.

### Epic 50. LongMemEval End-to-End Adapter

Status: done

Evidence:

- `scripts/longmemeval/check_v1_e2e_adapter.py`
- `make longmemeval-v1-e2e-adapter-check`
- `target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o/manifest.json`
- `target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o/hypotheses.jsonl`
- `target/longmemeval-v1/submission/cortexdb-longmemeval-v1-official-gpt4o/eval-results-gpt-4o.jsonl`
- `target/longmemeval-v1/e2e-adapter/report.json`
- Latest local E2E adapter report: `status=passed`, `hypotheses_rows=500`,
  `eval_rows=500`, QA `accuracy=0.7660`, and official retrieval
  `session recall_all@10=0.9021`.

Tasks:

- Wire ContextPack to reader LLM. Done: the packaged historical official local
  run validates CortexDB retrieval output through the official reader
  hypotheses file.
- Run official QA eval. Done: the package validates 500 official QA eval rows
  and recomputes the `383 / 500` score from labels.
- Separate retrieval claims from QA claims. Done: the E2E adapter report records
  retrieval metrics and QA accuracy as separate claims and states that this is
  not a published LongMemEval leaderboard entry.

### Epic 51. LoCoMo Adapter

Status: done

Evidence:

- `scripts/locomo/download.py`
- `scripts/locomo/check_retrieval_adapter.py`
- `crates/cortex-engine/src/bin/locomo_retrieval/main.rs`
- `crates/cortex-engine/src/bin/locomo_retrieval/args.rs`
- `crates/cortex-engine/src/bin/locomo_retrieval/model.rs`
- `crates/cortex-engine/src/bin/locomo_retrieval/view.rs`
- `make locomo-retrieval-adapter-check`
- `target/locomo/data/manifest.json`
- `target/locomo/retrieval/cortexdb_locomo_retrieval.jsonl`
- `target/locomo/retrieval/cortexdb_locomo_report.json`
- `target/locomo/retrieval-adapter/report.json`
- Latest local retrieval-only report: `status=passed`, `samples=10`,
  `turns_indexed=5,882`, `questions=1,986`, `rows_with_evidence=1,982`,
  `hit@1=0.3199`, and `hit@10=0.6312`.

Tasks:

- Ingest conversational memory. Done: `locomo_retrieval` writes LoCoMo
  conversation turns into CortexDB as durable `document_block` cells with
  sample-specific scopes.
- Start with retrieval-only evaluation. Done: the adapter searches each
  conversation scope and scores retrieved `dia_id` values against LoCoMo QA
  evidence IDs.
- Add optional end-to-end evaluation. Done as a documented boundary: the report
  records `optional_e2e=not_run`; E2E answer scoring remains a separate future
  gate because this epic closes the retrieval adapter without API keys.

### Epic 52. Search Explain API

Status: done

Evidence:

- `crates/cortex-server/src/search.rs`
- `crates/cortex-server/src/responses.rs`
- `crates/cortex-server/src/tests/search_api_tests.rs`
- `crates/cortex-server/src/tests/snapshot_completion_tests.rs`
- `crates/cortex-sdk/src/types.rs`
- `crates/cortex-sdk/src/tests.rs`
- `docs/API.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/openapi.yaml`
- `make openapi-contract-check`
- `cargo test -p cortex-server search_explain`
- `cargo test -p cortex-sdk`

Tasks:

- Explain term scores. Done: `/v1/search/explain` returns per-term
  contribution rows with weighted term frequencies.
- Explain vector score. Done: each result returns `vector_score` and
  `vector_contribution_q16`.
- Explain fusion score. Done: hybrid results return `fusion_rank_score` and a
  deterministic contribution summary.
- Explain selected strategy. Done: explain responses include the same
  `routing.selected_strategy` contract as `/v1/search`.
- Explain matched fields. Done: each result reports matched ranking fields:
  `title`, `body_text`, and/or `vector`.

### Epic 53. Query Routing Engine

Status: done

Evidence:

- `crates/cortex-engine/src/search/routing.rs`
- `crates/cortex-server/src/search.rs`
- `crates/cortex-server/src/tests/search_api_tests.rs`
- `crates/cortex-cli/src/cli_ops.rs`
- `crates/cortex-cli/src/tests.rs`
- `crates/cortex-sdk/src/types.rs`
- `crates/cortex-sdk/src/tests.rs`
- `docs/API.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/openapi.yaml`
- `cargo test -p cortex-engine routing`
- `cargo test -p cortex-server v1_search_auto_reports_selected_routing_strategy`
- `cargo test -p cortex-cli search_command_auto_mode_reports_routing_json`
- `cargo test -p cortex-sdk path_encodes_auto_search_routing_contract`

Tasks:

- Route lexical queries. Done: explicit/default keyword requests route to
  `SearchRouteStrategy::Keyword`.
- Route vector queries. Done: `mode=vector&algorithm=ann|exact` routes to
  `vector_ann` or `vector_exact`.
- Route hybrid queries. Done: text plus vector routes to `hybrid`, and explicit
  hybrid mode fails closed when no vector is provided.
- Explain strategy. Done: server, CLI, SDK, and API docs expose
  `routing.selected_strategy` and `routing.reason`.
- Define fallback behavior. Done: invalid mode/algorithm values fail closed, and
  `auto` routes based on text/vector availability with explicit ANN/exact
  selection.

### Epic 54. HNSW SLO History

Status: done

Evidence:

- `make ann-production-slo-history-check`
- `target/ann/production-slo-history/runs/history.json`
- `crates/cortex-engine/src/search/ann_corpus.rs`
- `scripts/ann/summarize_history.py`
- `scripts/ann/history_gate.py`
- `scripts/ann/history_contract.py`
- `docs/ANN_PRODUCTION_TUNING.md`

Latest local evidence:

- `run_count=10`
- `corpus_count=1`
- `regression_count=0`
- `latest_min_observed_recall_q16=65535`
- `latest_mean_recall_q16=65535`
- `latest_p95_latency_nanos=9600`
- `latest_p99_latency_nanos=9600`
- `latest_fallback_count=0`
- `latest_fallback_rate_q16=0`
- `latest_graph_freshness_q16=65535`
- `latest_stale_vector_count=0`
- `latest_production_safe=true`

Tasks:

- Run 10+ HNSW runs. Done: `ann-production-slo-history-check` creates ten
  release-mode local domain ANN/HNSW runs.
- Track latency. Done: history tracks p50, p95, p99, and max latency and gates
  adjacent regressions with configured tolerances.
- Track recall. Done: history tracks min and mean recall and fails on recall
  regressions.
- Track fallback rate. Done: ANN corpus reports `fallback_count` and
  `fallback_rate_q16`; the history gate requires latest fallback to be zero.
- Track graph freshness. Done: ANN corpus reports `graph_freshness_q16` and
  `stale_vector_count`; the history gate requires full freshness and zero stale
  vectors.

### Epic 55. HNSW Failure Simulation

Status: done

Evidence:

- `crates/cortex-engine/src/search/ann.rs`
- `crates/cortex-engine/src/search/ann/tests.rs`
- `crates/cortex-engine/src/search/database.rs`
- `crates/cortex-engine/tests/hnsw_failure_simulation.rs`
- `docs/API_JSON_SCHEMAS.md`
- `docs/SEARCH.md`
- `docs/openapi.yaml`
- `cargo test -p cortex-engine --test hnsw_failure_simulation`

Tasks:

- Simulate corrupt graph. Done: corrupt `.ach` checksum now fails validation
  but vector search returns exact fallback with `invalid_graph`.
- Simulate missing trailer. Done: truncated `.ach` falls back to exact vector
  search instead of failing the query.
- Simulate stale vector. Done: a persisted vector candidate missing from the
  HNSW graph returns exact fallback with `stale_graph`.
- Verify fallback to exact. Done: integration tests assert the expected
  `CellId` is returned from exact `.acv` search for all failure modes.

### Epic 56. Vector Index Rebuild Tool

Status: done

Evidence:

- `Database::rebuild_vector_indexes` rebuilds live-segment `.acv` files and
  `.ach` files when HNSW is enabled or already expected by the manifest.
- `cortexdb vector rebuild <path> [--experimental-hnsw]` exposes the local
  repair path.
- `crates/cortex-engine/tests/vector_rebuild.rs`
- `crates/cortex-cli/src/tests.rs`

Tasks:

- Add `cortexdb vector rebuild`. Done.
- Validate ACV/ACH. Done: rebuild finishes with `validate_storage()`.
- Repair mismatch. Done: corrupt `.acv`, corrupt `.ach`, and stale `.ach`
  regression tests rebuild healthy persisted ANN bundles.

### Epic 57. Embedding Provider Abstraction

Status: done

Evidence:

- `scripts/ann/embedding_provider.py`
- `scripts/ann/embed_text_command.py`
- `scripts/ann/export_embedding_domain_corpus.py`
- `scripts/ann/preflight_real_embedding_benchmark.py`
- `scripts/ann/real_embedding_readiness.py`
- `make ann-scripts-check`

Tasks:

- Support OpenAI-compatible providers. Done: direct provider mode and wrapper
  mode both use env-only endpoint/model/key handling.
- Support local endpoints. Done: `local` provider accepts endpoint config
  without requiring an API key and can omit model unless explicitly required.
- Support file-based embeddings. Done: `file` provider reads JSONL vectors keyed
  by text or text SHA-256.
- Ensure no secrets are committed. Done: provider reports expose endpoint origin,
  env var name, and key-presence boolean, not the key value.

### Epic 58. Embedding Cache

Status: done

Evidence:

- `scripts/ann/embedding_cache.py`
- `scripts/ann/embedding_provider.py`
- `scripts/ann/embedding_provider_selftest.py`
- `Makefile`
- `docs/ANN_PRODUCTION_TUNING.md`
- `make ann-scripts-check`

Tasks:

- Cache text hash to embedding. Done: optional JSONL cache is keyed by input
  text SHA-256 plus provider identity.
- Invalidate on model change. Done: cache identity includes provider model.
- Invalidate on dimension change. Done: cache identity includes configured
  dimension and cached vectors are still dimension-validated on read.

### Epic 59. Retrieval Regression Dashboard

Status: done

Evidence:

- `scripts/retrieval_quality_dashboard.py`
- `scripts/retrieval_quality_dashboard_panels.py`
- `scripts/retrieval_quality_dashboard_self_test.py`
- `make retrieval-quality-check`
- `target/retrieval-quality/dashboard.html`

Tasks:

- Add dashboard panel for recall. Done.
- Add dashboard panel for MRR. Done.
- Add dashboard panel for nDCG. Done.
- Add dashboard panel for latency trends. Done: the dashboard reads
  `target/retrieval-quality/history.json` and compares latest p95 latency
  against the previous per-domain run.

### Epic 60. Search Quality Gate v2

Status: done

Evidence:

- `fixtures/search_quality_gate_v2_thresholds.json`
- `scripts/search_quality_gate_v2.py`
- `make search-quality-gate-v2-check`
- `make retrieval-quality-check`
- `target/search-quality-gate-v2/report.json`
- `release-check` invokes `retrieval-quality-check`, so release fails on search
  quality regression.

Tasks:

- Add per-domain thresholds. Done: checked-in thresholds cover
  `investment_projects`, `legal_policies`, `support_tickets`, and
  `technical_docs`.
- Add exact parity checks. Done: per-domain exact parity and ANN exact parity
  are thresholded.
- Add ANN safe mode. Done: the gate requires production-safe ANN history, zero
  fallback, full graph freshness, and zero stale vectors.
- Fail release on regression. Done: beta/history/ANN regression counts must be
  zero and `release-check` runs the retrieval/search gate.

## F. ContextPack Production Layer

Acceptance: ContextPack becomes trustworthy product output, not just internal response.

### Epic 61. ContextPack Quality v2

Status: done

Evidence:

- `examples/eval/context_pack_quality.jsonl`
- `scripts/context_pack_quality_check.py`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`
- `make context-pack-quality-check`
- `target/context-pack-quality/report.json`

Tasks:

- Add 25+ cases. Done: the fixture contains 25 ContextPack quality cases.
- Cover 4 domains. Done: the fixture covers 5 domains and the gate requires
  at least 4 domains.
- Measure evidence, citation, token, redundancy, and anomaly metrics. Done:
  the generated report includes evidence coverage, citation coverage, token
  reduction, redundancy reduction, anomaly coverage, deterministic ordering,
  and per-domain metrics.

### Epic 62. ContextPack Quality v3

Status: done

Evidence:

- `fixtures/context_pack_quality_v3_datasets.json`
- `fixtures/context_pack_quality_v3_thresholds.json`
- `scripts/context_pack_quality_v3_check.py`
- `make context-pack-quality-v3-check`
- `make context-pack-quality-check`
- `target/context-pack-quality/v3-report.json`

Tasks:

- Add 100+ cases. Done: the v3 gate expands real-domain seed cases into 105
  deterministic quality cases.
- Use external datasets. Done: the dataset descriptor binds the gate to 4
  checked-in real-domain external dataset fixtures.
- Add failure categories. Done: the report tracks evidence selection,
  citation pressure, token-budget pressure, redundancy pressure, and anomaly
  pressure categories.
- Add per-domain thresholds. Done: checked-in thresholds cover each v3 domain
  and are enforced by `context_pack_quality_v3_check.py`.

### Epic 63. ContextPack Explain v2

Status: done

Evidence:

- `crates/cortex-engine/tests/context_pack_explain_v2.rs`
- `scripts/context_pack_explain_v2_check.py`
- `make context-pack-explain-v2-check`
- `target/context-pack-quality/explain-v2-report.json`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Explain why selected. Done: selected cells expose `why_selected`, matched
  terms, and structured score components.
- Explain why excluded. Done: excluded candidates expose `why_excluded` in
  ContextPack anomalies.
- Explain source trust. Done: explain output includes `source_trust_q16`,
  `source_trust_category`, and `source_trust_bonus`.
- Explain redundancy penalty. Done: explain output includes
  `redundancy_penalty` and a negative score component reason.
- Explain token budget reason. Done: token-overload anomalies explain
  `estimated_tokens` vs `token_budget_tokens` pressure.

### Epic 64. ContextPack Prompt Export

Status: done

Evidence:

- `crates/cortex-engine/src/context/export.rs`
- `crates/cortex-engine/src/context/export/json_export.rs`
- `crates/cortex-engine/tests/context_pack_prompt_export.rs`
- `scripts/context_pack_prompt_export_check.py`
- `make context-pack-prompt-export-check`
- `target/context-pack-quality/prompt-export-report.json`
- `docs/openapi.yaml`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Export JSON format. Done: `ContextPackExportFormat::Json` emits
  `context_pack.v1` JSON from the engine and the HTTP default remains typed
  JSON.
- Export Markdown format. Done: `ContextPackExportFormat::Markdown` remains
  covered by export tests and docs.
- Export prompt format. Done: `ContextPackExportFormat::Prompt` remains
  covered by export tests and docs.
- Add citation instructions. Done: prompt export tells agents to preserve
  citations and cite `citation=` or `source_ref=` values for factual claims.
- Add conflict-handling prompt. Done: prompt export tells agents not to resolve
  conflicting evidence silently and to report conflicts with citations.

### Epic 65. ContextPack Answerability Score

Status: done

Evidence:

- `crates/cortex-engine/src/context/answerability.rs`
- `crates/cortex-engine/tests/context_pack_answerability.rs`
- `scripts/context_pack_answerability_check.py`
- `make context-pack-answerability-check`
- `target/context-pack-quality/answerability-report.json`
- `docs/openapi.yaml`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Estimate whether context is enough. Done: ContextPack now emits
  `answerability_q16` as a deterministic 0..65535 coverage score for explicit
  query terms selected into the pack.
- Emit `insufficient_context` anomaly. Done: packs with empty or partially
  covered context emit `ContextPackAnomalyCode::InsufficientContext` with
  covered/missing term details.

### Epic 66. ContextPack Conflict Visibility Metric

Status: done

Evidence:

- `crates/cortex-engine/src/context/conflicts.rs`
- `crates/cortex-engine/tests/context_pack_conflict_visibility.rs`
- `scripts/context_pack_conflict_visibility_check.py`
- `make context-pack-conflict-visibility-check`
- `target/context-pack-quality/conflict-visibility-report.json`
- `docs/openapi.yaml`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Measure whether conflicting evidence appears in pack. Done: ContextPack now
  emits `conflict_visibility_q16` and `visible_conflict_count` for selected
  `project` + `metric` groups that contain multiple `value=` variants.

### Epic 67. ContextPack Private Scope Leak Test

Status: done

Evidence:

- `crates/cortex-engine/tests/context_pack_private_scope.rs`
- `scripts/context_pack_private_scope_check.py`
- `make context-pack-private-scope-check`
- `target/context-pack-quality/private-scope-report.json`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Ensure forbidden scope never appears in ContextPack. Done: a broad
  `WHERE status = "ready"` ContextPack query excludes forbidden-scope ready
  cells before persistence, after checkpoint/restart, after compact/restart,
  and from JSON, prompt, and Markdown exports.

### Epic 68. ContextPack Token Estimator v2

Status: done

Evidence:

- `crates/cortex-engine/src/context/token_estimator.rs`
- `crates/cortex-engine/tests/context_pack_token_estimator.rs`
- `scripts/context_pack_token_estimator_check.py`
- `make context-pack-token-estimator-check`
- `target/context-pack-quality/token-estimator-report.json`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Improve token estimation. Done: ContextPack now uses deterministic
  profile-based token estimation instead of a single byte-count heuristic.
- Add model-specific profiles. Done: `ContextTokenProfile` covers Cortex
  default, GPT-4o-like, DeepSeek-chat-like, Gemma-it-like, and BGE-M3-like
  profiles plus model-name alias mapping.

### Epic 69. ContextPack Large Cell Policy

Status: done

Evidence:

- `crates/cortex-engine/src/context/large_cell.rs`
- `crates/cortex-engine/tests/context_pack_large_cell_policy.rs`
- `scripts/context_pack_large_cell_policy_check.py`
- `make context-pack-large-cell-policy-check`
- `target/context-pack-quality/large-cell-policy-report.json`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`
- `docs/CONTEXT_PACK_QUALITY_EVIDENCE.md`

Tasks:

- Define truncate policy. Done: `ContextLargeCellPolicy::Truncate` includes a
  UTF-8-safe prefix plus a deterministic truncation marker when it fits.
- Define exclude policy. Done: `ContextLargeCellPolicy::Exclude` omits
  oversized cells and emits `token_overload` evidence.
- Define summarize-placeholder policy. Done:
  `ContextLargeCellPolicy::SummarizePlaceholder` emits deterministic metadata
  and reference fields without calling an LLM.
- Define source-only reference policy. Done:
  `ContextLargeCellPolicy::SourceOnlyReference` keeps provenance-style metadata
  while omitting the oversized body.

### Epic 70. ContextPack SDK Types v1

Status: done

Evidence:

- `crates/cortex-sdk/src/context_pack.rs`
- `crates/cortex-sdk/src/context_pack_tests.rs`
- `crates/cortex-sdk/src/types.rs`
- `docs/SDK_QUICKSTART.md`

Tasks:

- Add typed SDK models for cells. Done: `ContextPackCellV1` aliases the
  stable typed response model and is covered by serde round-trip tests.
- Add typed SDK models for source refs. Done: `ContextPackSourceRefV1`
  covers source id, optional URL, document/page/range/json-path, and
  confidence fields.
- Add typed SDK models for explain. Done: `ContextPackExplainV1` and
  `ScoreComponentResponse` cover selection reasons, score components, source
  trust, and redundancy fields.
- Add typed SDK models for anomalies. Done: `ContextPackAnomalyV1` covers
  optional cell id, code, message, and `why_excluded`, with helper counting on
  `ContextPackV1`.

## G. Verification And Trust

Acceptance: Verify has measured quality, structured conflicts, and usable reports.

### Epic 71. Verification Dataset v2

Status: done

Evidence:

- `examples/eval/verification_cases.jsonl`
- `crates/cortex-engine/tests/verification_evaluation.rs`
- `scripts/verification_quality_check.py`
- `target/verification-quality/report.json`
- `docs/VERIFICATION_QUALITY_EVIDENCE.md`

Tasks:

- Add 50+ cases. Done: `examples/eval/verification_cases.jsonl` contains 50
  deterministic labelled cases and `make verification-quality-check` validates
  them.
- Cover 4 domains. Done: the report covers 5 domains:
  `investment_projects`, `support_tickets`, `legal_policies`,
  `technical_docs`, and `world_indicators`.
- Include supported, contradicted, mixed, and insufficient labels. Done:
  latest report records `supported=14`, `contradicted=20`, `mixed=8`, and
  `insufficient=8` with zero false positives and zero false negatives.

### Epic 72. Verification Dataset v3

Status: done

Evidence:

- `examples/eval/verification_cases.jsonl`
- `crates/cortex-engine/tests/verification_evaluation.rs`
- `scripts/verification_quality_check.py`
- `target/verification-quality/report.json`
- `docs/VERIFICATION_QUALITY_EVIDENCE.md`

Tasks:

- Add 200+ cases. Done: `examples/eval/verification_cases.jsonl`
  contains 200 deterministic labelled cases.
- Include temporal cases. Done: latest report records 70 temporal cases.
- Include numeric cases. Done: latest report records 114 numeric cases.
- Include currency cases. Done: latest report records 75 currency cases.
- Include source cases. Done: latest report records 25 source/citation cases.
- Include ambiguous cases. Done: latest report records 23 ambiguous cases.
- Include outdated evidence cases. Done: latest report records 24 outdated/current-old cases.

### Epic 73. Engine-native NumericValue

Status: done

Evidence:

- `crates/cortex-engine/src/verification/numeric/mod.rs`
- `crates/cortex-engine/src/verification/numeric/parse.rs`
- `crates/cortex-engine/src/verification/numeric/value.rs`
- `crates/cortex-engine/src/verification/numeric/tests.rs`
- `crates/cortex-engine/tests/verification_guards.rs`
- `docs/VERIFY_FACT.md`

Tasks:

- Add unit parser. Done: `parse_unit_code` normalizes units and aliases
  such as `%`, `percent`, `hrs`, and `sec`.
- Add currency parser. Done: `parse_currency_code` validates and normalizes
  supported currency codes to uppercase.
- Add magnitude parser. Done: `parse_magnitude_suffix` parses `B/M/K/%`
  and multilingual suffixes used by deterministic verification.
- Add normalized comparison. Done: `compare_numeric_values`,
  `normalized_numeric_equal`, and `NumericValue` helper methods distinguish
  equal, conflicting, and incomparable numeric contexts without floats.
- Add structured conflicts. Done: `VerificationNumericConflict` continues to
  carry metric, left/right display values, and typed `NumericValue` pairs, with
  regression tests in `verification_guards`.

### Epic 74. Date/Temporal Conflict Detection

Status: done

Evidence:

- `crates/cortex-engine/src/verification/temporal.rs`
- `crates/cortex-engine/src/verification/guards.rs`
- `crates/cortex-engine/tests/verification_guards.rs`
- `crates/cortex-engine/tests/verification_evaluation.rs`
- `examples/eval/verification_cases.jsonl`
- `docs/VERIFY_FACT.md`

Tasks:

- Add date parser. Done: `parse_temporal_date` and
  `extract_temporal_query_range` parse `YYYY-MM-DD`, `YYYY/MM/DD`, and
  year-only query ranges.
- Add `valid_from`. Done: verification reads `valid_from=` headers and treats
  future evidence as not yet valid for earlier facts.
- Add `valid_to`. Done: verification reads `valid_to=` headers and treats
  expired evidence as stale for later facts.
- Add stale fact detection. Done: stale evidence is excluded from support and
  contradiction paths, emits a `stale_fact` guard, and is covered by labelled
  fixture cases.

### Epic 75. Source Trust Model v1

Status: done

Evidence:

- `crates/cortex-engine/src/source_trust.rs`
- `crates/cortex-engine/src/context/pack.rs`
- `crates/cortex-engine/src/verification.rs`
- `crates/cortex-engine/tests/context_pack.rs`
- `crates/cortex-engine/tests/verification_tests.rs`
- `docs/SOURCE_TRUST_MODEL.md`
- `docs/VERIFY_FACT.md`

Tasks:

- Add source trust categories. Done: `SourceTrustCategory` maps absent
  metadata to `unknown` and Q16 ranges to `low`, `medium`, `high`, and
  `official`.
- Add trust score in ContextPack. Done: ContextPack explain blocks include
  `source_trust_q16`, `source_trust_category`, `source_trust_bonus`, and the
  `source_trust_bonus` score component.
- Add trust score in Verify. Done: `VERIFY FACT` reports source trust on
  supporting and contradicting evidence and sorts equal matches by higher
  trust first.

### Epic 76. Source Trust Calibration

Status: done

Evidence:

- `crates/cortex-engine/src/source_trust.rs`
- `crates/cortex-engine/src/query/metadata.rs`
- `crates/cortex-engine/src/query/metadata_validation.rs`
- `crates/cortex-engine/src/context/pack.rs`
- `crates/cortex-engine/src/verification.rs`
- `crates/cortex-engine/tests/context_pack.rs`
- `crates/cortex-engine/tests/verification_tests.rs`
- `docs/SOURCE_TRUST_MODEL.md`
- `docs/VERIFY_FACT.md`

Tasks:

- Define official/internal/extracted/inferred weights. Done:
  `source_trust_class` calibrates missing explicit Q16 metadata to stable
  weights: official `60000`, internal `52000`, extracted `40000`, inferred
  `20000`; explicit `source_trust_q16` remains the override.
- Explain trust contribution. Done: ContextPack score components now explain
  whether source trust came from explicit Q16 metadata, calibrated class
  metadata, or the default unknown score.

### Epic 77. Contradiction Index v1

Status: done

Evidence:

- `crates/cortex-engine/src/verification/conflict_index.rs`
- `crates/cortex-engine/src/verification.rs`
- `crates/cortex-engine/tests/verification_conflict_index.rs`
- `docs/VERIFY_FACT.md`
- `docs/API_CHANGELOG.md`

Tasks:

- Persist known conflicts. Done: durable Relation cells with
  `predicate=contradicts` remain the persisted conflict primitive and are read
  by `Database::conflict_index`.
- Query by entity. Done: `Database::conflicts_for_entity` filters inline and
  persisted conflicts using structured `entity=` / `project=` facets and fact
  text fallback.
- Query by metric. Done: `Database::conflicts_for_metric` filters inline and
  persisted conflicts using structured `metric=` facets and fact text fallback.
- Query by source. Done: `Database::conflicts_for_source` filters by readable
  evidence source metadata; persisted relation records inherit source facets
  only from source cells visible to the caller's `AgentView`.

### Epic 78. Verification Markdown Export

Status: done

Evidence:

- `crates/cortex-engine/src/verification/export.rs`
- `crates/cortex-engine/tests/verification_export.rs`
- `crates/cortex-engine/tests/verification_guards.rs`
- `docs/VERIFY_FACT.md`
- `docs/API_CHANGELOG.md`

Tasks:

- Export report table. Done: Markdown export includes a `## Report Table`
  section with fact, status, evidence counts, guard count, and numeric conflict
  count.
- Include supporting evidence. Done: `## Supporting Evidence` remains in the
  stable Markdown report and is covered by regression tests.
- Include contradicting evidence. Done: `## Contradicting Evidence` remains in
  the stable Markdown report and is covered by regression tests.
- Include guards. Done: `## Guards` lists rule guards with code, cell id, and
  message.
- Include limitations. Done: `## Limitations` explains AgentView visibility,
  missing-evidence semantics, rule-based numeric/temporal/contradiction checks,
  and source-trust boundaries.

### Epic 79. Verification SDK Helpers

Status: done

Tasks:

- Add typed verify request builders. Done: Rust SDK now exposes
  `VerifyRequest` with stable `/v1/verify` path construction, JSON/Markdown/Audit
  formats, and `VerifyRequest::fact(...)` for AQL-backed request generation.
- Add result enums. Done: `VerifyResult` maps current and legacy wire statuses
  including `supported`, `insufficient`, `contradicted`, `mixed`, and
  `mixed_evidence`.
- Add conflict types. Done: `VerifyConflict`, `VerifyEvidenceConflict`, and
  `VerifyNumericConflict` expose contradicting evidence and numeric conflicts
  from `VerificationReportResponse::conflicts()`.
- Evidence: `crates/cortex-sdk/src/verification.rs` and
  `crates/cortex-sdk/src/verification_tests.rs` cover request building, result
  mapping, and conflict extraction. `docs/SDK_QUICKSTART.md` documents the
  helper flow.

### Epic 80. Verification Quality Dashboard

Status: done

Tasks:

- Add confusion matrix. Done: `scripts/verification_quality_dashboard.py`
  converts the quality report into dashboard `confusion_rows` and a rendered
  Markdown confusion table.
- Track false positives. Done: dashboard JSON/Markdown carries
  `false_positive_count`, and the gate fails closed when the source report
  exposes non-zero false positives.
- Track false negatives. Done: dashboard JSON/Markdown carries
  `false_negative_count`, and the gate fails closed when the source report
  exposes non-zero false negatives.
- Track per-domain quality. Done: `per_domain_quality` summarizes domain-level
  case counts, status counts, and accuracy q16 for each verification domain.
- Evidence: `make verification-quality-check` now runs
  `scripts/verification_quality_dashboard_self_test.py` and writes
  `target/verification-quality/dashboard.json` plus
  `target/verification-quality/dashboard.md`.

## H. Ingestion And Data Pipeline

Acceptance: ingestion becomes operationally safe and provenance-rich.

### Epic 81. Ingestion Jobs v2

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/jobs.rs`
- `crates/cortex-engine/src/ingestion/progress.rs`
- `crates/cortex-engine/tests/ingestion_job_tests.rs`
- `crates/cortex-server/src/tests/ingest_tests.rs`
- `crates/cortex-cli/src/cli_ingest_tests.rs`
- `docs/INGESTION.md`
- `docs/API.md`
- `scripts/ingestion_jobs_v2_check.py`
- `make ingestion-jobs-v2-check`

Tasks:

- Add durable jobs. Done: jobs are persisted atomically under
  `ingestion_jobs/<job_id>.json`.
- Add retry. Done: failed jobs can be requeued with retry counters and max retry
  guards through engine, HTTP, and CLI surfaces.
- Add cancel. Done: queued/running jobs can be cancelled through engine, HTTP,
  and CLI surfaces.
- Add progress. Done: records include total/completed/failed item counters and
  the last emitted cell id.
- Add failure reasons. Done: failed records persist `message` and failed-item
  counters.
- Resume after restart. Done: stale `running` jobs are requeued as `queued` with
  a recovery message on database open.

### Epic 82. Ingestion Job Dashboard

Status: done

Evidence:

- `web/dashboard/src/reporting_ingest.js`
- `web/dashboard/src/index.html`
- `web/dashboard/src/app.js`
- `web/dashboard/src/style.css`
- `crates/cortex-server/src/dashboard.rs`
- `crates/cortex-server/src/dashboard_tests.rs`
- `docs/DASHBOARD_UI.md`
- `scripts/dashboard_dist_smoke.py`
- `scripts/dashboard_product_check.py`
- `make ingestion-job-dashboard-check`

Tasks:

- View progress. Done: the dashboard renders total/completed/failed/last-cell
  progress cards for one job and aggregate counters for the job list.
- View failures. Done: persisted job detail renders failure reason and retry
  counters.
- View warnings. Done: ingestion summaries render validation warnings and
  skipped inputs.
- View records. Done: persisted ingestion jobs render a table with job id,
  label, status, progress, failed count, last cell, and message.
- View chunks. Done: ingestion summaries render emitted chunk rows.
- View source refs. Done: emitted chunks show source-ref, source id, document
  id, citation availability, and confidence.

### Epic 83. Structured SourceRef v1

Status: done

Evidence:

- `crates/cortex-engine/src/query/metadata.rs`
- `crates/cortex-engine/src/query/metadata_validation.rs`
- `crates/cortex-engine/src/ingestion/cells.rs`
- `crates/cortex-engine/src/ingestion/adapters.rs`
- `crates/cortex-engine/src/ingestion/report.rs`
- `crates/cortex-engine/tests/ingestion_adapters.rs`
- `crates/cortex-engine/tests/ingestion_validation_report.rs`
- `crates/cortex-server/src/responses.rs`
- `crates/cortex-sdk/src/types.rs`
- `docs/openapi.yaml`
- `docs/CELL_METADATA_MODEL.md`
- `scripts/structured_source_ref_check.py`
- `make structured-source-ref-check`

Tasks:

- Add document ID. Done: SourceRef parser, ingestion adapters, validation
  reports, OpenAPI, HTTP snapshots, and SDK types preserve `document_id`.
- Add page. Done: PDF ingestion writes page SourceRef metadata and ContextPack
  plus validation reports expose `page`.
- Add row. Done: SourceRef parser accepts `row`/`row_number`; CSV ingestion
  writes 1-based source rows and reports expose `row`.
- Add JSON path. Done: JSON ingestion writes flattened `json_path` provenance
  per emitted fact and reports expose it.
- Add source URL. Done: SourceRef metadata and ingestion reports preserve
  `source_url` / `url` when present.
- Add extraction confidence. Done: SourceRef confidence stays fixed-point
  `confidence_q16` across metadata, reports, ContextPack exports, HTTP, and SDK
  surfaces.

### Epic 84. Deterministic Chunking v1

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/chunking.rs`
- `crates/cortex-engine/src/ingestion/formats.rs`
- `crates/cortex-engine/tests/ingestion_chunking_policy.rs`
- `docs/DETERMINISTIC_CHUNKING.md`
- `scripts/deterministic_chunking_check.py`
- `make deterministic-chunking-check`

Tasks:

- Add stable chunk IDs. Done: text chunks use deterministic
  `<sanitized-document>#chunk-000N` ids independent of `CellId`.
- Define overlap policy. Done: `TextOverlapPolicy::FixedChars` is exposed from
  `TextChunkPolicy` and long paragraphs split with fixed character overlap.
- Define JSON policy. Done: `JsonChunkPolicy` uses `.` path separators,
  numeric array path components, and sorted leaf paths before cell writes.
- Define table policy. Done: `TableChunkPolicy` treats row 1 as headers and
  emits 1-based data-row provenance with `cell_range=row-<n>`.

### Epic 85. Chunking Quality Benchmark

Status: done

Evidence:

- `examples/eval/chunking_quality_settings.json`
- `scripts/chunking_quality_benchmark.py`
- `docs/CHUNKING_QUALITY_BENCHMARK.md`
- `docs/BENCHMARKS.md`
- `make chunking-quality-benchmark-check`

Tasks:

- Evaluate chunk size vs retrieval quality. Done: the benchmark compares
  candidate `TextChunkPolicy` values per domain and reports doc-level
  `recall_at_k_q16`, `mrr_q16`, chunk count, and average chunk size.
- Add per-domain settings. Done: settings cover `investment_projects`,
  `support_tickets`, `legal_policies`, and `technical_docs`; the gate fails if
  the selected policy diverges from the current benchmark recommendation.

### Epic 86. PDF Digital Text Adapter

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/pdf.rs`
- `crates/cortex-engine/src/ingestion/pdf_contracts.rs`
- `crates/cortex-engine/src/ingestion/pdf_ingest.rs`
- `crates/cortex-engine/tests/ingestion_pdf_digital.rs`
- `docs/PDF_TEXT_EXTRACTION.md`
- `make pdf-digital-adapter-check`

Tasks:

- Define external parser boundary. Done: `ExternalPdfParserAdapter`,
  `ExternalPdfParserRequest`, and `DisabledExternalPdfParserAdapter` define the
  production parser extension point separately from OCR.
- Capture text extraction metadata. Done: `PdfExtractionStats` now records
  page count, page-level text blocks, literal-string counts, and hex-string
  counts; PDF ingestion writes extraction boundary and count headers.
- Add page source refs. Done: `Database::ingest_pdf_bytes_pages` emits one cell
  per extracted digital-PDF text page with `page=<n>` and
  `cell_range=page-<n>` SourceRef metadata.

### Epic 87. OCR Adapter Trait

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/pdf_contracts.rs`
- `crates/cortex-engine/tests/ingestion_pdf_contracts.rs`
- `docs/PDF_TEXT_EXTRACTION.md`
- `docs/INGESTION.md`
- `make ocr-adapter-trait-check`

Tasks:

- Add external OCR interface. Done: `ExternalOcrAdapter`,
  `ExternalOcrRequest`, `ExternalOcrOutput`, and the fail-closed disabled
  adapter define the extension point.
- Define scanned PDF boundary. Done: `ScannedPdfOcrRequest` makes the
  render-PDF-pages-to-images boundary explicit before OCR is invoked.
- Capture confidence metadata. Done: OCR output supports page-level and
  block-level `confidence_q16`.
- Capture bbox metadata. Done: OCR text blocks support normalized bounding boxes
  and validate zero-size/out-of-bounds coordinates.

### Epic 88. Ingestion Validation Report

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/report.rs`
- `crates/cortex-engine/tests/ingestion_validation_report.rs`
- `crates/cortex-server/src/tests/response_snapshot_tests.rs`
- `docs/openapi.yaml`
- `docs/INGESTION.md`
- `make ingestion-validation-report-check`

Tasks:

- Report processed records. Done: reports now expose `processed_records`
  alongside the compatibility `cells_seen` field.
- Report skipped records. Done: `record_skipped` updates `skipped_records` and
  preserves item-level skip reasons.
- Report warnings. Done: structured warning codes remain exposed and are
  covered by engine and HTTP snapshot tests.
- Report invalid metadata. Done: strict metadata decode increments
  `invalid_metadata_records` and records an `invalid_metadata` warning.
- Report source refs. Done: per-cell SourceRef summaries remain exposed in the
  engine report and HTTP ingestion response schema.

### Epic 89. Ingestion Backpressure

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/backpressure.rs`
- `crates/cortex-engine/tests/ingestion_job_tests.rs`
- `crates/cortex-server/src/tests/ingest_tests.rs`
- `docs/INGESTION.md`
- `make ingestion-backpressure-check`

Tasks:

- Add job queue limits. Done: `IngestionBackpressurePolicy` rejects new
  ingestion starts when persisted queued/running job counts exceed configured
  limits.
- Add memory limits. Done: `max_input_bytes` rejects oversized ingestion
  payloads before WAL/MemTable writes.
- Add rate limits. Done: per-database in-memory request windows reject
  excessive accepted ingestion starts.
- Add cancellation. Done: `ensure_ingestion_job_not_cancelled` prevents
  cancelled durable jobs from being continued by future worker-style ingestion
  paths.

### Epic 90. Ingestion Deduplication

Status: done

Evidence:

- `crates/cortex-engine/src/ingestion/dedup.rs`
- `crates/cortex-engine/src/ingestion/cells.rs`
- `crates/cortex-engine/tests/ingestion_adapters.rs`
- `docs/INGESTION.md`
- `make ingestion-deduplication-check`

Tasks:

- Add content hash. Done: ingestion payloads now emit deterministic
  `content_hash` metadata and `CellMetadata` decodes it.
- Add source hash. Done: ingestion payloads now emit deterministic
  `source_hash` metadata and `CellMetadata` decodes it.
- Detect duplicate chunks. Done: text ingestion can find visible cells with the
  same source/content hash before writing.
- Define update policy. Done: `IngestionUpdatePolicy::AlwaysInsert` preserves
  existing behavior while `SkipExisting` skips duplicate visible chunks.

## I. Security And Access Control

Acceptance: security moves from alpha controls to beta/production-governed controls.

### Epic 91. Dynamic RBAC Policy Store

Status: done

Evidence:

- `crates/cortex-server/src/auth_policy_store.rs`
- `crates/cortex-server/src/auth.rs`
- `crates/cortex-server/src/auth_capability.rs`
- `crates/cortex-server/src/auth_policy_cells.rs`
- `crates/cortex-cli/src/cli_auth_review.rs`
- `crates/cortex-server/src/tests/auth_policy_tests.rs`
- `crates/cortex-cli/src/cli_auth_review_tests.rs`
- `docs/AUTH.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/RBAC_POLICY_STORE_DESIGN.md`
- `make rbac-policy-store-check`

Tasks:

- Add roles. Done: dynamic policy-store principals support `admin` and `data`
  roles and fail closed for unknown roles.
- Add grants. Done: `capabilities` restrict principals to explicit API action
  classes such as `search`, `read`, `write`, `ingest`, and `verify`.
- Add token mapping. Done: policy-store principals map bearer tokens to
  principal IDs, roles, optional AgentView IDs, quotas, capabilities, disabled
  state, and redacted policy-cell mirrors.
- Add scope read/write. Done: policy-store principals can bind to persisted
  AgentViews, and AgentView readable/writable scopes are enforced on data
  routes.
- Add tenant policy. Done: policy-store principals may set `tenants` allowlists;
  invalid tenant policies fail closed and disallowed tenant requests return
  `403`.

### Epic 92. RBAC Admin API

Status: done

Tasks:

- Create role. Done within the Core Alpha static-role boundary: admin
  endpoints can create or update principals bound to supported roles `admin`
  and `data`; dynamic custom role definitions remain future work.
- Grant scope. Done: `POST /v1/admin/auth/scope/grant` mutates persisted
  AgentView readable/writable scopes for existing agents.
- Revoke scope. Done: `POST /v1/admin/auth/scope/revoke` mutates persisted
  AgentView readable/writable scopes for existing agents.
- List policies. Done: `GET /v1/admin/auth/policies` returns a redacted
  policy-store listing with token fingerprints, quotas, capabilities, tenants,
  disabled state, role, and AgentView binding.
- Audit changes. Done at the HTTP route layer: new admin routes are classified
  as admin actions by the existing audit classifier and covered by the
  OpenAPI/error contract gate.

Evidence:

- `crates/cortex-server/src/auth_policy_store.rs`
- `crates/cortex-server/src/auth_scope_admin.rs`
- `crates/cortex-server/src/tests/auth_policy_tests.rs`
- `docs/AUTH.md`
- `docs/API.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/openapi.yaml`
- `make rbac-policy-store-check`
- `make openapi-contract-check`

Boundary:

- Scope grant/revoke intentionally update AgentView, not the auth policy store,
  so AgentView remains the single source of read/write scope permissions.
- Dynamic custom role definitions are not implemented; Core Alpha supports
  static `admin` and `data` roles plus principal capabilities.

### Epic 93. Per-token Quotas

Status: done

Tasks:

- Add request rate quota. Done: policy-store principals can set
  `request_quota_per_minute`, and requests over that per-principal fixed-window
  quota return typed `429 rate_limited`.
- Add body size quota. Done: policy-store principals can set
  `body_quota_bytes_per_minute`, and accepted/rejected body bytes are counted
  per principal.
- Add queue budget. Done: policy-store principals can set `queue_quota`, and
  actor command permits are acquired/released per principal.
- Add context budget per token. Done: policy-store principals can set
  `context_budget_tokens`, which clamps the bound AgentView budget used by AQL
  and ContextPack routes.

Evidence:

- `crates/cortex-server/src/auth.rs`
- `crates/cortex-server/src/auth_capability.rs`
- `crates/cortex-server/src/auth_policy_store.rs`
- `crates/cortex-server/src/auth_policy_cells.rs`
- `crates/cortex-server/src/router.rs`
- `crates/cortex-server/src/tests/security_quota_tests.rs`
- `crates/cortex-server/src/tests/auth_policy_tests.rs`
- `crates/cortex-cli/src/cli_auth_review.rs`
- `crates/cortex-cli/src/cli_auth_review_tests.rs`
- `docs/AUTH.md`
- `docs/API.md`
- `docs/API_JSON_SCHEMAS.md`
- `docs/openapi.yaml`
- `make quota-policy-check`

Boundary:

- Quotas are local fixed-window/process-local guardrails. They are not
  distributed global quotas and reset on process restart.
- `context_budget_tokens` applies when the principal is bound to an AgentView;
  it clamps that AgentView rather than replacing AgentView policy.

### Epic 94. Future Tamper-evident Audit Log

Status: done

Tasks:

- Add hash chain. Done: file-backed audit records include `chain_id`,
  `prev_hash`, and `event_hash`; HTTP and LLM inference audit records use the
  same local chain verifier hash model.
- Add sequence numbers. Done: the audit sink assigns monotonic `sequence`
  values, persists them in JSONL, and continues from the existing tail after
  restart.
- Add audit verify. Done: `cortexdb audit --verify-chain`,
  `cortexdb audit verify <file>`, and SIEM export `--verify-chain` validate
  sequence continuity and event hash integrity offline.
- Add tamper detection. Done: regression tests cover edited route metadata,
  edited LLM inference metadata, deleted records, and reordered records.

Evidence:

- `crates/cortex-server/src/audit.rs`
- `crates/cortex-server/src/audit_chain.rs`
- `crates/cortex-server/src/audit/llm.rs`
- `crates/cortex-server/src/audit_tests.rs`
- `crates/cortex-cli/src/cli_audit.rs`
- `crates/cortex-cli/src/cli_audit_chain.rs`
- `crates/cortex-cli/src/cli_audit_tests.rs`
- `crates/cortex-cli/src/cli_audit_chain_tests.rs`
- `docs/AUTH.md`
- `docs/SECURITY_BETA_BASELINE.md`
- `make audit-chain-check`

Boundary:

- This is a local tamper-evidence foundation for JSONL audit files. It is not
  an immutable compliance ledger, external timestamping service, or
  vendor-managed SIEM delivery guarantee.

### Epic 95. Audit Export and Retention

Status: done

Tasks:

- Export audit events. Done: `cortexdb audit-export-siem` writes local
  normalized JSONL with schema `cortexdb.siem.audit.v1`, preserving principal,
  request, route, status, timing, and audit-chain metadata.
- Define retention policy. Done: `AUDIT_EXPORT_RETENTION_POLICY.md` and `.json`
  define local audit JSONL, SIEM export JSONL, and local-only raw debug
  retention classes.
- Define redaction policy. Done: the policy lists forbidden body/query/token/
  prompt/provider-response fields, safe exported metadata, and required
  `--redaction-check`/`--verify-chain` workflows.

Evidence:

- `crates/cortex-cli/src/cli_audit_siem.rs`
- `crates/cortex-cli/src/cli_audit_siem_tests.rs`
- `docs/AUDIT_EXPORT_RETENTION_POLICY.md`
- `docs/AUDIT_EXPORT_RETENTION_POLICY.json`
- `docs/AUTH.md`
- `docs/SECURITY_HARDENING_EVIDENCE.md`
- `scripts/audit_export_retention_check.py`
- `make audit-export-retention-check`

Boundary:

- CortexDB provides local audit review and local normalized JSONL export. It
  does not provide vendor-managed SIEM delivery, legal retention enforcement,
  or compliance-certified immutable audit custody.

### Epic 96. Encrypted Backups MVP

Status: done

Tasks:

- Add passphrase encryption. Done: this epic is the production-track
  duplicate of Epic 23; `backup-encrypted`, `restore-encrypted`, and
  `Database::encrypted_backup_path` create and restore CortexDB-local
  passphrase archives.
- Add key derivation. Done for MVP: archive metadata records
  `cortexdb.fnv64-passphrase.v1` as the deterministic local KDF boundary.
- Create encrypted archive. Done: `make encrypted-backup-check` writes
  `target/encrypted-backup/backup.cdbenc`.
- Restore encrypted archive. Done: the same gate restores checkpointed data and
  WAL-tail data, validates the restored database, rejects a wrong passphrase,
  and rejects corrupt ciphertext without creating the target database.

Evidence:

- `make encrypted-backup-check`
- `target/encrypted-backup/report.json`
- `scripts/encrypted_backup_check.py`
- `crates/cortex-engine/src/backup/encrypted/`
- `crates/cortex-engine/tests/backup_restore.rs`
- `crates/cortex-cli/src/tests.rs`
- `docs/ENCRYPTED_BACKUPS_DESIGN.md`
- `docs/BACKUP_RESTORE.md`

Boundary:

- This closes the local encrypted-backup MVP for release evidence. The current
  format is passphrase-based and deterministic for local drills; it is not
  KMS-backed, externally audited, or compliance-certified encryption.

### Epic 97. Remote Backup Adapter

Status: done

Tasks:

- Add local adapter. Done: this epic is the production-track duplicate of
  Epic 22; `LocalFilesystemOffsiteAdapter` stages validated backup copies into
  an external/offsite root.
- Design S3-compatible adapter. Done for this MVP boundary: the
  `OffsiteBackupAdapter` trait defines the provider seam, while actual
  S3/GCS/Azure object-store upload remains out of scope until provider-backed
  restore gates exist.
- Add dry-run. Done through the staging preflight: `stage_backup_offsite`
  restores the source backup into a temporary preflight directory before
  publishing, and `make backup-offsite-check` records
  `preflight_restore_completed=true`.
- Add checksum validation. Done: the staged copy is opened through
  `Database::validate_storage`, which reads manifest, WAL, segment, bitmap,
  lexical, vector, and HNSW files through their checksum-aware storage readers.

Evidence:

- `make backup-offsite-check`
- `target/backup-offsite/report.json`
- `crates/cortex-engine/src/backup/offsite.rs`
- `crates/cortex-engine/tests/backup_restore.rs`
- `crates/cortex-cli/src/tests.rs`
- `scripts/backup_offsite_check.sh`
- `docs/BACKUP_RESTORE.md`
- `docs/CLI.md`

Boundary:

- This closes local remote/offsite staging for single-node release evidence.
  It does not provide provider-backed S3/GCS/Azure upload, remote object-store
  restore drills, managed backup custody, or remote durability claims.

### Epic 98. Secret Rotation Workflow

Status: done

Tasks:

- Add token file rotation. Done: `CORTEXDB_AUTH_TOKENS_FILE` and
  `ServerOptions::auth_tokens_file` support file-backed local token policy
  rotation.
- Add reload. Done: the auth layer re-reads the token policy file for every
  request, so replacing the file rotates tokens without changing server
  options or restarting the process.
- Fail closed on invalid token file. Done: missing, empty/comment-only, and
  malformed token files do not authenticate requests.

Evidence:

- `cargo test -p cortex-server auth_rotation_tests`
- `cargo test -p cortex-server token_policy_file`
- `crates/cortex-server/src/auth.rs`
- `crates/cortex-server/src/tests/auth_rotation_tests.rs`
- `crates/cortex-server/src/tests/auth_policy_tests.rs`
- `docs/AUTH.md`
- `docs/BETA_OPERATIONS.md`
- `docs/SECURITY_RELEASE_CHECKLIST.md`

Boundary:

- This closes local file-backed static-token rotation. It does not provide
  external identity provider rotation, managed sessions, hardware-backed
  secrets, or enterprise credential lifecycle automation.

### Epic 99. Security Check Gate v2

Status: done

Evidence:

- `Makefile` target `security-gate-v2-check`
- `scripts/security_gate_v2_check.py`
- `target/security-gate-v2/report.json`
- `target/security/report.json`
- `target/security-hardening/report.json`
- `target/enterprise-rbac/rbac-policy-store.json`
- `target/enterprise-rbac/quota-policy.json`
- `target/enterprise-rbac/audit-chain.json`
- `target/audit-export-retention/report.json`
- `make security-gate-v2-check`

Tasks:

- Check auth. Done: the v2 gate requires the `auth_required`,
  `wrong_token_rejected`, and `data_token_admin_denied` checks from
  `target/security/report.json`.
- Check RBAC. Done: the v2 gate requires passing RBAC policy-store evidence
  from `target/enterprise-rbac/rbac-policy-store.json` and matching hardening
  checks.
- Check tenant isolation. Done: the v2 gate requires the
  `tenant_traversal_rejected` check.
- Check CORS. Done: the v2 gate requires the `cors_allowlist_works` check.
- Check rate limits. Done: the v2 gate requires `rate_limit_works` and the
  per-principal quota report.
- Check audit. Done: the v2 gate requires audit redaction, audit-chain, and
  audit-export-retention reports.
- Check malicious ingestion. Done: the v2 gate requires
  `malicious_ingestion_tests` in the hardening report.

Boundary:

- This closes the local single-node HTTP security gate for auth, RBAC policy
  store, tenant isolation, CORS, rate limits, audit, malicious ingestion, and
  OpenAPI contracts. It does not claim external identity, enterprise compliance
  certification, managed-cloud security, or distributed authorization
  correctness.

### Epic 100. Security Hardening Report

Status: done

Evidence:

- `docs/SECURITY_HARDENING_EVIDENCE.md`
- `docs/SECURITY_RELEASE_CHECKLIST.md`
- `scripts/security_release_report_check.py`
- `Makefile` target `security-release-report-check`
- `target/security-release/report.json`
- `make security-release-report-check`

Tasks:

- Generate security report per release. Done: `make security-release-report-check`
  validates the release security-hardening report after
  `security-gate-v2-check` and `compliance-boundary-check` pass.
- Include remaining risks. Done: `SECURITY_HARDENING_EVIDENCE.md` now lists
  explicit remaining risks for external identity, enterprise compliance,
  managed-cloud security, distributed authorization, KMS-backed backup custody,
  provider-backed object-store backup, compliance-grade audit ledger, and TLS
  lifecycle.

Boundary:

- This closes the local per-release security hardening report. It does not
  convert any remaining risk into a production guarantee or external
  certification.

## J. Observability And Operations

Acceptance: operator can run, observe, debug, and recover CortexDB without author intervention.

### Epic 101. `cortexdb doctor`

Status: done

Evidence:

- `crates/cortex-cli/src/cli_doctor.rs`
- `crates/cortex-cli/src/cli_doctor_checks.rs`
- `crates/cortex-cli/src/tests.rs`
- `docs/CLI.md`
- `Makefile` target `doctor-check`
- `make doctor-check`

Tasks:

- Check DB lock. Done: doctor reports the active lock after a successful open
  and gives stale-lock unlock advice when open fails on an existing lock file.
- Validate storage. Done: doctor includes `validate_storage_report` cell and
  WAL scan evidence.
- Check backup age. Done: doctor checks `CORTEXDB_BACKUP_ROOT` when configured
  and conventional local backup directories otherwise.
- Check server health. Done: doctor optionally checks
  `CORTEXDB_SERVER_URL`/`CORTEXDB_SERVER_ADDR` TCP reachability.
- Check auth. Done: doctor reports inline auth env, token file, and policy file
  configuration and fails on unreadable configured auth files.
- Check tenant. Done: doctor reports the tenant realm and rejects invalid CLI
  tenant identifiers before command execution.
- Print repair advice. Done: doctor prints repair or stale-lock advice in every
  report.

Boundary:

- This closes local operator doctor diagnostics. It does not run repair
  automatically, prove backup restore quality, or guarantee remote application
  health unless a server endpoint is explicitly configured.

### Epic 102. Metrics Contract v2

Status: done

Evidence:

- `docs/METRICS_CONTRACT_V2.md`
- `docs/METRICS.md`
- `scripts/metrics_contract_v2_check.py`
- `scripts/observability_check.py`
- `crates/cortex-server/src/tests/snapshot_tests.rs`
- `Makefile` target `metrics-contract-v2-check`
- `make metrics-contract-v2-check`

Tasks:

- Stabilize metrics names. Done: Metrics Contract v2 freezes the required JSON
  metrics fields, latency histogram fields, and Prometheus series names.
- Document metrics. Done: `METRICS_CONTRACT_V2.md` and `METRICS.md` document
  the full `/v1/metrics` JSON contract, including principal quota counters.
- Add Prometheus examples. Done: the contract references
  `/v1/metrics?format=prometheus`, `examples/observability/prometheus.yml`,
  and the required Prometheus series names.
- Test metrics output. Done: `metrics_prometheus_output_contains_contract_series`
  checks Prometheus output, response snapshots cover JSON shape, and
  `metrics-contract-v2-check` validates Rust/OpenAPI/docs/snapshots/source
  alignment.

Boundary:

- This closes the local metrics field-name contract. It does not add long-term
  metric retention, external Prometheus deployment, or route-level latency
  histograms beyond the existing ANN search histogram.

### Epic 103. Grafana Dashboard Pack

Status: done

Evidence:

- `examples/observability/grafana-cortexdb-core-alpha.json`
- `docs/METRICS.md`
- `scripts/observability_check.py`
- `Makefile` target `observability-check`
- `make observability-check`

Tasks:

- Add JSON dashboard. Done: the Grafana dashboard pack is checked in as
  `examples/observability/grafana-cortexdb-core-alpha.json`.
- Cover storage. Done: panels cover commit/checkpoint, WAL size, WAL write
  rate, live segments, and retired segments.
- Cover requests. Done: panels cover request throughput and mean request
  latency from `cortexdb_request_count` and
  `cortexdb_request_duration_ms_total`.
- Cover errors. Done: `Errors and Rejections` covers request rejections,
  validation failures, and principal quota rejections.
- Cover actor queue. Done: `Actor Queue Pressure` covers queue depth and
  capacity.
- Cover backup age. Done: `/v1/metrics` and Prometheus now expose
  `backup_latest_age_seconds` / `cortexdb_backup_latest_age_seconds`, and the
  dashboard includes a `Backup Age` panel.

Boundary:

- This closes the dashboard pack for local Prometheus/Grafana evidence. It does
  not deploy Grafana, provision dashboards in managed infrastructure, or define
  alert routing.

### Epic 104. Alert Rules Pack

Status: done

Evidence:

- `examples/observability/alerts.yml`
- `docs/OBSERVABILITY_ALERTS.md`
- `scripts/observability_check.py`
- `make observability-check`

Tasks:

- Alert on stale backup. Done: `CortexDbBackupStale` fires when
  `cortexdb_backup_latest_age_seconds` is older than 24 hours.
- Alert on validation failure. Done: `CortexDbValidationFailures` remains a
  critical alert on validation-failure counter growth.
- Alert on high actor queue. Done: `CortexDbActorQueuePressure` watches queue
  depth versus capacity.
- Alert on error rate. Done: `CortexDbOperationalErrorRateHigh` watches recent
  rejected requests plus validation failures over request count.
- Alert on rate-limit spike. Done: `CortexDbRateLimitSpike` watches
  per-principal request, body-size, and queue quota rejection counters.

### Epic 105. Request ID and Trace Correlation

Status: done

Evidence:

- `crates/cortex-server/src/lib.rs`
- `crates/cortex-server/src/audit.rs`
- `crates/cortex-server/src/tests/security_tests.rs`
- `docs/API.md`
- `docs/AUTH.md`
- `docs/METRICS_CONTRACT_V2.md`
- `make metrics-contract-v2-check`

Tasks:

- Add request ID header. Done: every HTTP response carries `x-request-id`,
  echoing safe client IDs or generating `cortexdb-<n>`.
- Add audit correlation. Done: audit records and tracing audit events include
  `request_id`.
- Add logs. Done: HTTP request spans include `request_id`, `method`, and
  `path` for route-level correlation.
- Add metrics labels. Done: `/v1/metrics` exposes client-provided and generated
  request-id counters, plus the low-cardinality Prometheus label series
  `cortexdb_request_id_source_total{source="client|generated"}`.

### Epic 106. Operations Runbook v1

Status: done

Evidence:

- `docs/OPERATIONS_RUNBOOK_V1.md`
- `docs/OPERATIONS_RUNBOOK_EVIDENCE.md`
- `scripts/operations_runbook_check.py`
- `make operations-runbook-check`

Tasks:

- Document startup. Done: runbook includes install, server startup, health,
  auth, and write/read smoke commands.
- Document shutdown. Done: runbook includes foreground, systemd, launchd, stale
  lock, and post-shutdown validation steps.
- Document backup. Done: runbook includes local, encrypted, offsite staging,
  and backup production pack gates.
- Document restore. Done: runbook requires restore into a new directory and
  post-restore validation.
- Document validate. Done: runbook covers CLI validation, stats, WAL, manifest,
  ANN validation, doctor, and HTTP validation.
- Document repair. Done: runbook requires dry-run first, best-effort repair,
  WAL tools, and backup fallback.
- Document upgrade. Done: runbook links upgrade/rollback docs and lists the
  offline upgrade sequence plus compatibility gates.
- Document incidents. Done: runbook includes database busy, invalid tenant,
  suspected corruption, and audit review flows.

### Epic 107. Incident Playbooks

Status: done

Evidence:

- `docs/INCIDENT_PLAYBOOKS.md`
- `docs/INCIDENT_PLAYBOOKS_EVIDENCE.md`
- `scripts/incident_playbooks_check.py`
- `make incident-playbooks-check`

Tasks:

- Add corrupted storage playbook. Done: includes validation, WAL/manifest
  triage, dry-run repair, best-effort repair, restore, containment, and exit
  criteria.
- Add actor busy playbook. Done: includes metrics triage, actor queue
  pressure, client backoff, load smoke evidence, and validation exit criteria.
- Add backup failed playbook. Done: includes stale/missing backup evidence,
  backup drill, offsite staging, pruning containment, and production pack gate.
- Add auth failure spike playbook. Done: includes 401/403 triage, audit review,
  redaction, policy review, token rotation guidance, and security gate.
- Add tenant issue playbook. Done: includes invalid tenant triage, tenant
  naming rules, tenant validation, policy mapping containment, and tenant
  recovery/quota gates.

### Epic 108. Load Testing Suite

Status: done

Evidence:

- `scripts/load_suite_check.py`
- `docs/LOAD_TESTING_SUITE.md`
- `docs/LOAD_TESTING_SUITE_EVIDENCE.md`
- `fixtures/performance/workload_classes.json`
- `make load-suite-check`

Tasks:

- Add read-heavy workload. Done: repeated `GET /v1/cell` lookups after seed
  writes.
- Add write-heavy workload. Done: concurrent `POST /v1/cell` write phase.
- Add context-heavy workload. Done: repeated `/v1/context` calls over seeded
  evidence cells.
- Add verify-heavy workload. Done: repeated `/v1/verify` calls over seeded
  evidence cells.
- Add ingest-heavy workload. Done: repeated `/v1/ingest/text` calls.
- Add mixed-tenant workload. Done: multi-tenant write/read cycles using tenant
  query routing.

### Epic 109. Performance Trend Report

Status: done

Evidence:

- `scripts/performance_trend_check.py`
- `docs/PERFORMANCE_TREND_HISTORY.md`
- `docs/SINGLE_NODE_SLO.md`
- `make performance-trend-check`

Tasks:

- Track p50 per endpoint. Done: trend gate now requires and compares `p50_ms`
  for HTTP smoke and single-node lifecycle flows.
- Track p95 per endpoint. Done: `p95_ms` checks remain required and compared
  to release history.
- Track p99 per endpoint. Done: `p99_ms` checks remain required and compared
  to release history.
- Track trend over releases. Done: report compares current load and
  single-node reports against the latest fixture under
  `fixtures/performance/history`.
- Add regression gates. Done: report fails on missing latency fields,
  threshold violations, failed current reports, missing history, or actor busy.

### Epic 110. Single-node SLO Dashboard

Status: done

Evidence:

- `web/dashboard/src/index.html`
- `web/dashboard/src/app.js`
- `web/dashboard/src/reporting_slo.js`
- `scripts/single_node_slo_dashboard_check.py`
- `docs/DASHBOARD_UI.md`
- `docs/SINGLE_NODE_SLO.md`
- `make single-node-slo-dashboard-check`

Tasks:

- Show availability. Done: `dashboard_slo.v1` summarizes health and
  compatibility status in the Overview route.
- Show latency. Done: the panel displays request count, mean latency, and the
  local latency budget from `/v1/metrics`.
- Show backup freshness. Done: backup age is shown when metrics expose it, and
  the backup evidence gate remains visible when browser backup state is
  operator-controlled.
- Show validation status. Done: validation status and validation error count
  are rendered beside manifest/WAL status.
- Show error budget. Done: rejected requests, quota rejects, validation
  failures, and visible dashboard incidents are rolled into one error-budget
  signal.

## K. Dashboard And UX

Acceptance: dashboard becomes an operational tool, not just a developer demo.

### Epic 111. Dashboard Operational Status View

Status: done

Evidence:

- `web/dashboard/src/index.html`
- `web/dashboard/src/app.js`
- `web/dashboard/src/reporting_operations.js`
- `scripts/dashboard_operational_status_check.py`
- `docs/DASHBOARD_UI.md`
- `make dashboard-operational-status-check`

Tasks:

- Show health. Done: Operational status renders health and compatibility cards.
- Show storage stats. Done: current seq, checkpoint seq, live segments,
  MemTable cells, and WAL bytes are shown in the details grid.
- Show actor queue. Done: actor queue depth and capacity are shown in summary
  and details from `/v1/metrics`.
- Show latest backup. Done: latest backup age is shown when available and the
  backup evidence gate is listed.
- Show validation. Done: validation status, manifest status, WAL validation,
  and validation error count are visible.
- Show recent errors. Done: last request issue, incidents, and incident
  timeline remain visible in the same view.

### Epic 112. ContextPack Explorer

Status: done

Tasks:

- Show cells. Done: ContextPack report renders selected cell previews with
  token estimates, citation state, matched terms, and why-selected text.
- Show source refs. Done: summary cards count source refs and the citation
  explorer lists citation/source-ref visibility for each selected cell.
- Show explain data. Done: explain rows render score, BM25 base,
  source-trust metadata, redundancy penalty, and score component reasons.
- Show anomalies. Done: anomaly explorer renders reported anomaly code,
  message, cell id, and `why_excluded` text when present.
- Show token budget. Done: ContextPack summary now shows token budget,
  estimated tokens, used percentage, and truncation status.
- Evidence gate. Done: `make context-pack-explorer-check`.

### Epic 113. Verification Explorer

Status: done

Tasks:

- Show verdict. Done: Verify report summary cards render verdict and status
  with supported, mixed, and bad-state tones.
- Show supporting evidence. Done: supporting evidence list renders matched
  terms, citation state, source-trust category/q16, and payload previews.
- Show contradicting evidence. Done: contradicting evidence list uses the same
  evidence renderer and highlights present contradictions.
- Show numeric conflicts. Done: numeric conflict explorer renders normalized
  metric disagreements such as budget deltas.
- Show guards. Done: guard explorer renders policy/runtime guard code and
  message by cell or globally.
- Evidence gate. Done: `make verification-explorer-check`.

### Epic 114. Retrieval Quality Explorer

Status: done

Tasks:

- Show recall. Done: `target/retrieval-quality/dashboard.html` renders guarded
  ANN recall in summary metrics, panels, domain rows, and query rows.
- Show MRR. Done: dashboard renderer and query/domain tables include MRR.
- Show nDCG. Done: dashboard renderer and query/domain tables include nDCG.
- Show latency. Done: dashboard renders p95 latency and a latency trend panel
  from retrieval history.
- Break down by domain and query. Done: dashboard includes a domain quality
  table and query-level table.
- Evidence gate. Done: `make retrieval-quality-explorer-check`; full benchmark
  gate remains `make retrieval-quality-check`.

### Epic 115. Permissions View

Status: done

Tasks:

- Show tenants. Done: Permissions route renders the active tenant boundary.
- Show tokens. Done: token active/storage/visibility cards show memory-only
  token posture without rendering the bearer value.
- Show roles. Done: role/access-level cards show public, data, or admin mode.
- Show scopes. Done: selected scope probes are collected from visible dashboard
  forms and rendered in the Permissions Explorer.
- Show AgentView. Done: AgentView source, server enforcement, readable probe,
  writable probe, and server-source-of-truth note are rendered.
- Show denials. Done: public/data/admin/read-only denials render as an explicit
  list for the current session posture.
- Evidence gate. Done: `make permissions-view-check`.

### Epic 116. Audit Viewer v2

Status: done

Evidence:

- `web/dashboard/src/reporting_audit.js`
- `scripts/audit_viewer_v2_check.py`
- `make audit-viewer-v2-check`

Tasks:

- Add filters. Done: the Overview audit panel includes safe category and
  severity filters.
- Add summary. Done: visible event, warning, hash-chain, redaction, and raw-log
  visibility cards render from `dashboard_audit_viewer.v2`.
- Add hash-chain verification. Done: the panel shows the CLI verification
  command and keeps raw audit JSONL out of the browser.
- Show redaction status. Done: query, body, and token visibility are explicitly
  false in the viewer state and rendered as browser-redacted status.
- Evidence gate. Done: `make audit-viewer-v2-check`.

### Epic 117. Ingestion Jobs View

Status: done

Evidence:

- `web/dashboard/src/reporting_ingest.js`
- `docs/DASHBOARD_UI.md`
- `scripts/dashboard_product_check.py`
- `make ingestion-job-dashboard-check`

Tasks:

- Show active jobs. Done: the Ingest route can render the persisted job list
  and counts queued/running jobs.
- Show progress. Done: job detail and list rows show completed/total progress.
- Show warnings. Done: ingestion validation warnings and skipped inputs render
  in the report.
- Show failures. Done: failed counts and failure reasons render in detail and
  list views.
- Show retries. Done: retry count and max retries render in job detail.
- Evidence gate. Done: `make ingestion-job-dashboard-check`.

### Epic 118. Backup/Restore View

Status: done

Evidence:

- `web/dashboard/src/app.js`
- `web/dashboard/src/reporting_operations.js`
- `scripts/backup_restore_view_check.py`
- `make backup-restore-view-check`

Tasks:

- Show latest backup. Done: latest backup age/status renders from
  `cortexdb_backup_latest_age_seconds`.
- Show restore drill status. Done: restore-drill command and
  `make backup-restore-production-pack-check` render in the operational panel.
- Show offsite status. Done: offsite stage command and
  `make backup-offsite-check` render in the operational panel.
- Show RPO/RTO. Done: `dashboard_backup_restore.v1` exposes RPO budget/status
  and RTO release evidence gate.
- Evidence gate. Done: `make backup-restore-view-check`.

### Epic 119. Incident View

Status: done

Evidence:

- `web/dashboard/src/app.js`
- `web/dashboard/src/reporting_operations.js`
- `scripts/incident_view_check.py`
- `make incident-view-check`

Tasks:

- Show errors. Done: `dashboard_incident_view.v1` summarizes failed dashboard
  API calls separately from rate-limit shaped failures.
- Show rate limits. Done: the view exposes request rejection, aggregate quota
  rejection, and per-principal request/body/queue quota rejection counters.
- Show actor busy status. Done: actor queue depth, capacity, ratio, and busy /
  near-capacity state render from `/v1/metrics`.
- Show storage warnings. Done: validation failures and missing admin validation
  checks render as storage warning events with operator evidence.
- Show backup failures. Done: validation-blocked backup posture, stale backup
  evidence, and missing admin/operator evidence render as backup failure events.
- Evidence gate. Done: `make incident-view-check`.

### Epic 120. Dashboard Role-based UI

Status: todo

Tasks:

- Add admin UI.
- Add data user UI.
- Add read-only UI.
- Hide dangerous operations by role.

## L. Packaging, Install, Deployment

Acceptance: user can install, run, and upgrade CortexDB on Linux/macOS without Rust toolchain.

### Epic 121. Linux/macOS Binary Release Pipeline

Status: todo

Tasks:

- Build Linux x86_64 binary.
- Build macOS arm64 binary.
- Build macOS x86_64 binary.
- Add checksums.
- Add smoke tests.

### Epic 122. Install Script

Status: todo

Tasks:

- Download release artifact.
- Verify checksum.
- Install binaries.
- Print next steps.

### Epic 123. Platform Support Matrix

Status: todo

Tasks:

- Document supported OS/arch.
- Document unsupported Windows statement.
- Document filesystem requirements.

### Epic 124. Systemd Unit

Status: todo

Tasks:

- Add service file.
- Add env file.
- Add data dir convention.
- Add token file convention.
- Add log convention.

### Epic 125. Launchd Plist

Status: todo

Tasks:

- Add macOS service config.
- Define paths.
- Define logs.
- Define environment.

### Epic 126. Docker Hardening v2

Status: todo

Tasks:

- Add read-only root option.
- Ensure non-root runtime.
- Add healthcheck.
- Validate volume permissions.

### Epic 127. Docker Compose Production Example

Status: todo

Tasks:

- Add reverse proxy.
- Add auth token.
- Add volume.
- Add backup sidecar example.

### Epic 128. Upgrade/Rollback CLI Flow

Status: todo

Tasks:

- Add preflight.
- Backup before upgrade.
- Restore rollback.
- Validate after upgrade.

### Epic 129. Release Artifact Manifest

Status: todo

Tasks:

- Include binary checksums.
- Include evidence checksums.
- Include SDK versions.
- Include OpenAPI version.
- Include storage format versions.

### Epic 130. Homebrew/Package Manager Feasibility

Status: todo

Tasks:

- Evaluate Homebrew formula.
- Evaluate Linux package.
- Write decision doc.

## M. Product Demos And Adoption

Acceptance: external developer understands the value, can run demos, and can contribute.

### Epic 131. Official Beta Landing Page

Status: todo

Tasks:

- Add one-liner.
- Add demo.
- Add value proposition.
- Add limitations.
- Add quickstart.
- Add architecture diagram.

### Epic 132. Use-case Pack: Investment Projects

Status: todo

Tasks:

- Polish demo.
- Add queries.
- Add ContextPack examples.
- Add Verify examples.
- Add benchmark report.

### Epic 133. Use-case Pack: Legal Policies

Status: todo

Tasks:

- Add corpus.
- Add search demo.
- Add ContextPack demo.
- Add Verify contradiction demo.
- Add citation demo.

### Epic 134. Use-case Pack: Support Tickets

Status: todo

Tasks:

- Add customer issue retrieval.
- Add memory updates.
- Add resolution verification.

### Epic 135. Use-case Pack: Technical Docs

Status: todo

Tasks:

- Add docs retrieval.
- Add tool hints.
- Add version conflicts.
- Add source refs.

### Epic 136. Public Benchmarks Page

Status: todo

Tasks:

- Publish storage benchmarks.
- Publish retrieval benchmarks.
- Publish ContextPack benchmarks.
- Publish Verify benchmarks.
- Publish LongMemEval results.
- Publish release trends.

### Epic 137. LongMemEval Evidence Page

Status: todo

Tasks:

- Publish retrieval-only results.
- Include official evaluator command.
- Include log format.
- Include limitations.

### Epic 138. Comparison Docs v2

Status: todo

Tasks:

- Compare with vector DB.
- Compare with RAG storage.
- Compare with Postgres.
- Compare with memory frameworks.
- Compare with document search.

### Epic 139. Contributor Onboarding v2

Status: todo

Tasks:

- Add module map.
- Add good first issues.
- Add test commands.
- Add issue templates.

### Epic 140. Community Roadmap Board

Status: todo

Tasks:

- Add milestones.
- Add beta blockers.
- Add production blockers.
- Add experimental tracks.

## N. Advanced Agent-native Layers

Acceptance: CortexDB evolves beyond RAG into an agent-native memory and context platform.

### Epic 141. Agent Memory v2

Status: todo

Tasks:

- Add long-term memory.
- Add working memory.
- Add private/shared memory.
- Add TTL/decay.
- Add feedback.

### Epic 142. Memory Quality Benchmark

Status: todo

Tasks:

- Benchmark update handling.
- Benchmark stale memory detection.
- Benchmark preference retrieval.
- Benchmark temporal changes.

### Epic 143. Tool Registry v1

Status: todo

Tasks:

- Add tool cells.
- Add permissions.
- Add input/output schema.
- Add tool retrieval by task.

### Epic 144. Tool Recommendation in ContextPack

Status: todo

Tasks:

- Include relevant tools with context.
- Explain why each tool was selected.

### Epic 145. Knowledge Graph Layer v1

Status: todo

Tasks:

- Add entity cells.
- Add relation cells.
- Add source-supports-fact edges.
- Add fact-contradicts-fact edges.

### Epic 146. Graph Retrieval

Status: todo

Tasks:

- Add multi-hop retrieval.
- Add graph proximity score.
- Explain graph edges.

### Epic 147. Graph Verification

Status: todo

Tasks:

- Make Verify use relation graph.
- Make Verify use source support edges.

### Epic 148. Agent Session Model

Status: todo

Tasks:

- Add session context.
- Add temporary memory.
- Add session TTL.
- Add session-scoped retrieval.

### Epic 149. Feedback Learning Loop

Status: todo

Tasks:

- Add feedback cells.
- Let feedback influence ranking.
- Add feedback decay.
- Explain feedback contribution.

### Epic 150. Future Managed Cloud Feasibility Track

Status: todo

Tasks:

- Evaluate hosted model. This remains out of scope for the current local single-node release boundary.
- Evaluate tenant isolation.
- Evaluate billing and quotas.
- Evaluate remote backup.
- Estimate operations cost.
- Write go/no-go decision.
