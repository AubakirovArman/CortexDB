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

- Done: 32 / 150
- Partial: 1 / 150
- Todo: 117 / 150
- Current closed epic: Epic 32, Engine Determinism Audit
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

Status: todo

Tasks:

- Estimate MemTable memory.
- Estimate index memory.
- Estimate ContextPack memory.
- Expose stats.

### Epic 34. Engine Config Model

Status: todo

Tasks:

- Add formal config struct.
- Add env loading.
- Align CLI/server config.
- Document config.

### Epic 35. Engine Panic Audit

Status: todo

Tasks:

- Search for `unwrap`, `expect`, and `panic`.
- Replace in core paths.
- Add regression tests.

## D. AQL / Query Layer

Acceptance: AQL becomes stable contract, explainable, safe, and tested across SDK/API.

### Epic 36. AQL v0.4 Compatibility Pack

Status: todo

Tasks:

- Add golden parser tests.
- Add golden binder tests.
- Add malformed query tests.
- Add permission denial tests.
- Add unknown field tests.

### Epic 37. AQL Explain

Status: todo

Tasks:

- Implement `EXPLAIN RETRIEVE`.
- Output bitmap plan.
- Output filters.
- Output candidate counts.
- Output selected retrieval mode.

### Epic 38. AQL Error Taxonomy

Status: todo

Tasks:

- Add `invalid_aql`.
- Add `permission_denied`.
- Add `unknown_field`.
- Add `unsupported_operator`.
- Map errors to HTTP and SDK.

### Epic 39. AQL Compatibility Changelog

Status: todo

Tasks:

- Require changelog for every grammar change.
- Add examples for grammar changes.

### Epic 40. AQL Query Cache

Status: todo

Tasks:

- Add parse/bind cache.
- Invalidate by AgentView/catalog version.

### Epic 41. AQL Require Semantics v1

Status: todo

Tasks:

- Formalize `REQUIRE citations`.
- Formalize confidence.
- Formalize source trust.
- Formalize freshness.

### Epic 42. AQL Limit/Budget Semantics

Status: todo

Tasks:

- Clarify candidate limit.
- Clarify ContextPack cell limit.
- Clarify token budget.

### Epic 43. AQL Security Fuzzing

Status: todo

Tasks:

- Generate random WHERE/NOT/AND/OR queries.
- Verify no scope bypass.

### Epic 44. AQL SDK Helpers

Status: todo

Tasks:

- Add SDK builder methods for retrieve.
- Add SDK builder methods for verify.
- Add SDK builder methods for remember.
- Reduce string-only query usage.

### Epic 45. AQL Query Examples Pack

Status: todo

Tasks:

- Add investment examples.
- Add legal examples.
- Add support examples.
- Add technical docs examples.

## E. Retrieval, Search, ANN

Acceptance: retrieval is measurable, explainable, multi-domain, and has official benchmark adapters.

### Epic 46. Multi-domain Retrieval Corpus v2

Status: todo

Tasks:

- Add `legal_policies`.
- Add `support_tickets`.
- Add `technical_docs`.
- Add corpus, chunks, queries, and ground truth.

### Epic 47. Retrieval Quality History

Status: todo

Tasks:

- Run repeated evaluations per domain.
- Track recall.
- Track MRR.
- Track nDCG.
- Track p95/p99 latency.
- Add no-regression report.

### Epic 48. Public Retrieval Benchmark Page

Status: todo

Tasks:

- Publish metrics table.
- Explain dataset size.
- Explain exact vs ANN.
- Document limitations.

### Epic 49. LongMemEval Retrieval Adapter

Status: todo

Tasks:

- Ingest LongMemEval-S.
- Produce official retrieval log.
- Store official metrics.

### Epic 50. LongMemEval End-to-End Adapter

Status: todo

Tasks:

- Wire ContextPack to reader LLM.
- Run official QA eval.
- Separate retrieval claims from QA claims.

### Epic 51. LoCoMo Adapter

Status: todo

Tasks:

- Ingest conversational memory.
- Start with retrieval-only evaluation.
- Add optional end-to-end evaluation.

### Epic 52. Search Explain API

Status: todo

Tasks:

- Explain term scores.
- Explain vector score.
- Explain fusion score.
- Explain selected strategy.
- Explain matched fields.

### Epic 53. Query Routing Engine

Status: todo

Tasks:

- Route lexical queries.
- Route vector queries.
- Route hybrid queries.
- Explain strategy.
- Define fallback behavior.

### Epic 54. HNSW SLO History

Status: todo

Tasks:

- Run 10+ HNSW runs.
- Track latency.
- Track recall.
- Track fallback rate.
- Track graph freshness.

### Epic 55. HNSW Failure Simulation

Status: todo

Tasks:

- Simulate corrupt graph.
- Simulate missing trailer.
- Simulate stale vector.
- Verify fallback to exact.

### Epic 56. Vector Index Rebuild Tool

Status: todo

Tasks:

- Add `cortexdb vector rebuild`.
- Validate ACV/ACH.
- Repair mismatch.

### Epic 57. Embedding Provider Abstraction

Status: todo

Tasks:

- Support OpenAI-compatible providers.
- Support local endpoints.
- Support file-based embeddings.
- Ensure no secrets are committed.

### Epic 58. Embedding Cache

Status: todo

Tasks:

- Cache text hash to embedding.
- Invalidate on model change.
- Invalidate on dimension change.

### Epic 59. Retrieval Regression Dashboard

Status: todo

Tasks:

- Add dashboard panel for recall.
- Add dashboard panel for MRR.
- Add dashboard panel for nDCG.
- Add dashboard panel for latency trends.

### Epic 60. Search Quality Gate v2

Status: todo

Tasks:

- Add per-domain thresholds.
- Add exact parity checks.
- Add ANN safe mode.
- Fail release on regression.

## F. ContextPack Production Layer

Acceptance: ContextPack becomes trustworthy product output, not just internal response.

### Epic 61. ContextPack Quality v2

Status: todo

Tasks:

- Add 25+ cases.
- Cover 4 domains.
- Measure evidence, citation, token, redundancy, and anomaly metrics.

### Epic 62. ContextPack Quality v3

Status: todo

Tasks:

- Add 100+ cases.
- Use external datasets.
- Add failure categories.
- Add per-domain thresholds.

### Epic 63. ContextPack Explain v2

Status: todo

Tasks:

- Explain why selected.
- Explain why excluded.
- Explain source trust.
- Explain redundancy penalty.
- Explain token budget reason.

### Epic 64. ContextPack Prompt Export

Status: todo

Tasks:

- Export JSON format.
- Export Markdown format.
- Export prompt format.
- Add citation instructions.
- Add conflict-handling prompt.

### Epic 65. ContextPack Answerability Score

Status: todo

Tasks:

- Estimate whether context is enough.
- Emit `insufficient_context` anomaly.

### Epic 66. ContextPack Conflict Visibility Metric

Status: todo

Tasks:

- Measure whether conflicting evidence appears in pack.

### Epic 67. ContextPack Private Scope Leak Test

Status: todo

Tasks:

- Ensure forbidden scope never appears in ContextPack.

### Epic 68. ContextPack Token Estimator v2

Status: todo

Tasks:

- Improve token estimation.
- Add model-specific profiles.

### Epic 69. ContextPack Large Cell Policy

Status: todo

Tasks:

- Define truncate policy.
- Define exclude policy.
- Define summarize-placeholder policy.
- Define source-only reference policy.

### Epic 70. ContextPack SDK Types v1

Status: todo

Tasks:

- Add typed SDK models for cells.
- Add typed SDK models for source refs.
- Add typed SDK models for explain.
- Add typed SDK models for anomalies.

## G. Verification And Trust

Acceptance: Verify has measured quality, structured conflicts, and usable reports.

### Epic 71. Verification Dataset v2

Status: todo

Tasks:

- Add 50+ cases.
- Cover 4 domains.
- Include supported, contradicted, mixed, and insufficient labels.

### Epic 72. Verification Dataset v3

Status: todo

Tasks:

- Add 200+ cases.
- Include temporal cases.
- Include numeric cases.
- Include currency cases.
- Include source cases.
- Include ambiguous cases.
- Include outdated evidence cases.

### Epic 73. Engine-native NumericValue

Status: todo

Tasks:

- Add unit parser.
- Add currency parser.
- Add magnitude parser.
- Add normalized comparison.
- Add structured conflicts.

### Epic 74. Date/Temporal Conflict Detection

Status: todo

Tasks:

- Add date parser.
- Add `valid_from`.
- Add `valid_to`.
- Add stale fact detection.

### Epic 75. Source Trust Model v1

Status: todo

Tasks:

- Add source trust categories.
- Add trust score in ContextPack.
- Add trust score in Verify.

### Epic 76. Source Trust Calibration

Status: todo

Tasks:

- Define official/internal/extracted/inferred weights.
- Explain trust contribution.

### Epic 77. Contradiction Index v1

Status: todo

Tasks:

- Persist known conflicts.
- Query by entity.
- Query by metric.
- Query by source.

### Epic 78. Verification Markdown Export

Status: todo

Tasks:

- Export report table.
- Include supporting evidence.
- Include contradicting evidence.
- Include guards.
- Include limitations.

### Epic 79. Verification SDK Helpers

Status: todo

Tasks:

- Add typed verify request builders.
- Add result enums.
- Add conflict types.

### Epic 80. Verification Quality Dashboard

Status: todo

Tasks:

- Add confusion matrix.
- Track false positives.
- Track false negatives.
- Track per-domain quality.

## H. Ingestion And Data Pipeline

Acceptance: ingestion becomes operationally safe and provenance-rich.

### Epic 81. Ingestion Jobs v2

Status: todo

Tasks:

- Add durable jobs.
- Add retry.
- Add cancel.
- Add progress.
- Add failure reasons.
- Resume after restart.

### Epic 82. Ingestion Job Dashboard

Status: todo

Tasks:

- View progress.
- View failures.
- View warnings.
- View records.
- View chunks.
- View source refs.

### Epic 83. Structured SourceRef v1

Status: todo

Tasks:

- Add document ID.
- Add page.
- Add row.
- Add JSON path.
- Add source URL.
- Add extraction confidence.

### Epic 84. Deterministic Chunking v1

Status: todo

Tasks:

- Add stable chunk IDs.
- Define overlap policy.
- Define JSON policy.
- Define table policy.

### Epic 85. Chunking Quality Benchmark

Status: todo

Tasks:

- Evaluate chunk size vs retrieval quality.
- Add per-domain settings.

### Epic 86. PDF Digital Text Adapter

Status: todo

Tasks:

- Define external parser boundary.
- Capture text extraction metadata.
- Add page source refs.

### Epic 87. OCR Adapter Trait

Status: todo

Tasks:

- Add external OCR interface.
- Define scanned PDF boundary.
- Capture confidence metadata.
- Capture bbox metadata.

### Epic 88. Ingestion Validation Report

Status: todo

Tasks:

- Report processed records.
- Report skipped records.
- Report warnings.
- Report invalid metadata.
- Report source refs.

### Epic 89. Ingestion Backpressure

Status: todo

Tasks:

- Add job queue limits.
- Add memory limits.
- Add rate limits.
- Add cancellation.

### Epic 90. Ingestion Deduplication

Status: todo

Tasks:

- Add content hash.
- Add source hash.
- Detect duplicate chunks.
- Define update policy.

## I. Security And Access Control

Acceptance: security moves from alpha controls to beta/production-governed controls.

### Epic 91. Dynamic RBAC Policy Store

Status: todo

Tasks:

- Add roles.
- Add grants.
- Add token mapping.
- Add scope read/write.
- Add tenant policy.

### Epic 92. RBAC Admin API

Status: todo

Tasks:

- Create role.
- Grant scope.
- Revoke scope.
- List policies.
- Audit changes.

### Epic 93. Per-token Quotas

Status: todo

Tasks:

- Add request rate quota.
- Add body size quota.
- Add queue budget.
- Add context budget per token.

### Epic 94. Future Tamper-evident Audit Log

Status: todo

Tasks:

- Add hash chain.
- Add sequence numbers.
- Add audit verify.
- Add tamper detection.

### Epic 95. Audit Export and Retention

Status: todo

Tasks:

- Export audit events.
- Define retention policy.
- Define redaction policy.

### Epic 96. Encrypted Backups MVP

Status: todo

Tasks:

- Add passphrase encryption.
- Add key derivation.
- Create encrypted archive.
- Restore encrypted archive.

### Epic 97. Remote Backup Adapter

Status: todo

Tasks:

- Add local adapter.
- Design S3-compatible adapter.
- Add dry-run.
- Add checksum validation.

### Epic 98. Secret Rotation Workflow

Status: todo

Tasks:

- Add token file rotation.
- Add reload.
- Fail closed on invalid token file.

### Epic 99. Security Check Gate v2

Status: todo

Tasks:

- Check auth.
- Check RBAC.
- Check tenant isolation.
- Check CORS.
- Check rate limits.
- Check audit.
- Check malicious ingestion.

### Epic 100. Security Hardening Report

Status: todo

Tasks:

- Generate security report per release.
- Include remaining risks.

## J. Observability And Operations

Acceptance: operator can run, observe, debug, and recover CortexDB without author intervention.

### Epic 101. `cortexdb doctor`

Status: todo

Tasks:

- Check DB lock.
- Validate storage.
- Check backup age.
- Check server health.
- Check auth.
- Check tenant.
- Print repair advice.

### Epic 102. Metrics Contract v2

Status: todo

Tasks:

- Stabilize metrics names.
- Document metrics.
- Add Prometheus examples.
- Test metrics output.

### Epic 103. Grafana Dashboard Pack

Status: todo

Tasks:

- Add JSON dashboard.
- Cover storage.
- Cover requests.
- Cover errors.
- Cover actor queue.
- Cover backup age.

### Epic 104. Alert Rules Pack

Status: todo

Tasks:

- Alert on stale backup.
- Alert on validation failure.
- Alert on high actor queue.
- Alert on error rate.
- Alert on rate-limit spike.

### Epic 105. Request ID and Trace Correlation

Status: todo

Tasks:

- Add request ID header.
- Add audit correlation.
- Add logs.
- Add metrics labels.

### Epic 106. Operations Runbook v1

Status: todo

Tasks:

- Document startup.
- Document shutdown.
- Document backup.
- Document restore.
- Document validate.
- Document repair.
- Document upgrade.
- Document incidents.

### Epic 107. Incident Playbooks

Status: todo

Tasks:

- Add corrupted storage playbook.
- Add actor busy playbook.
- Add backup failed playbook.
- Add auth failure spike playbook.
- Add tenant issue playbook.

### Epic 108. Load Testing Suite

Status: todo

Tasks:

- Add read-heavy workload.
- Add write-heavy workload.
- Add context-heavy workload.
- Add verify-heavy workload.
- Add ingest-heavy workload.
- Add mixed-tenant workload.

### Epic 109. Performance Trend Report

Status: todo

Tasks:

- Track p50 per endpoint.
- Track p95 per endpoint.
- Track p99 per endpoint.
- Track trend over releases.
- Add regression gates.

### Epic 110. Single-node SLO Dashboard

Status: todo

Tasks:

- Show availability.
- Show latency.
- Show backup freshness.
- Show validation status.
- Show error budget.

## K. Dashboard And UX

Acceptance: dashboard becomes an operational tool, not just a developer demo.

### Epic 111. Dashboard Operational Status View

Status: todo

Tasks:

- Show health.
- Show storage stats.
- Show actor queue.
- Show latest backup.
- Show validation.
- Show recent errors.

### Epic 112. ContextPack Explorer

Status: todo

Tasks:

- Show cells.
- Show source refs.
- Show explain data.
- Show anomalies.
- Show token budget.

### Epic 113. Verification Explorer

Status: todo

Tasks:

- Show verdict.
- Show supporting evidence.
- Show contradicting evidence.
- Show numeric conflicts.
- Show guards.

### Epic 114. Retrieval Quality Explorer

Status: todo

Tasks:

- Show recall.
- Show MRR.
- Show nDCG.
- Show latency.
- Break down by domain and query.

### Epic 115. Permissions View

Status: todo

Tasks:

- Show tenants.
- Show tokens.
- Show roles.
- Show scopes.
- Show AgentView.
- Show denials.

### Epic 116. Audit Viewer v2

Status: todo

Tasks:

- Add filters.
- Add summary.
- Add hash-chain verification.
- Show redaction status.

### Epic 117. Ingestion Jobs View

Status: todo

Tasks:

- Show active jobs.
- Show progress.
- Show warnings.
- Show failures.
- Show retries.

### Epic 118. Backup/Restore View

Status: todo

Tasks:

- Show latest backup.
- Show restore drill status.
- Show offsite status.
- Show RPO/RTO.

### Epic 119. Incident View

Status: todo

Tasks:

- Show errors.
- Show rate limits.
- Show actor busy status.
- Show storage warnings.
- Show backup failures.

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
