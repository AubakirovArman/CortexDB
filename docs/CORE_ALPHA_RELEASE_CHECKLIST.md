# Core Alpha Release Checklist

This checklist gates the `v0.1.0-core-alpha` tag. A release is allowed only when
every required row is green on `main`.

## Required Gates

| Gate | Required evidence |
| --- | --- |
| Workspace CI | GitHub Actions `Rust` workflow passes on stable and beta. |
| Local check | `RUSTFLAGS="-D warnings" cargo check --workspace` passes. |
| Local tests | `RUSTFLAGS="-D warnings" cargo test --workspace --all-features` passes. |
| Formatting | `cargo fmt --check` passes. |
| Lints | `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings` passes. |
| File size audit | All checked-in Rust source and integration test files stay under 300 lines. |
| SDK smoke | `./sdk/publish/check.sh` passes Python bytecode, Python SDK unit tests, and TypeScript package dry-run. |
| Storage safety | WAL, segment, bitmap, lexical, and manifest corruption tests pass. |
| Lifecycle safety | open, close, Drop, lock, and stale unlock tests pass. |
| Repair safety | `Database::repair_best_effort` removes orphan temps and truncates only safe WAL tails. |
| Production evidence sweep | `make production-evidence-sweep` writes `target/production-evidence/report.json` and per-step logs for OpenAPI contract, backup drill, ANN release evidence, and replication partition checks. |
| Backup drill evidence | `make backup-drill-check` writes `target/backup-drill/report.json` after backup, restore, prune, validate, and readback checks. |
| Offsite backup staging | `make backup-offsite-check` writes `target/backup-offsite/report.json` after local drill, validated offsite staging, staged validation, and readback checks. |
| Restart safety | put, patch, tombstone, checkpoint, compact, and WAL tail tests pass. |
| Crash matrix | `make crash-fault-check` writes `target/crash-fault/report.json` and targeted test logs for orphan bundles, temp manifests, restart tails, corruption, and repair. |
| Chaos restart evidence | `make chaos-restart-check` writes `target/chaos-restart/report.json` after repeatable HTTP writes, flushes, compacts, forced server kills, stale unlocks, repair, restart, and readback checks. |
| Consistency audit | `CORE_CONSISTENCY_AUDIT.md` is current and full-stack consistency tests pass. |
| Atomic audit | `ATOMIC_WRITE_AUDIT.md` and `STORAGE_FORMATS.md` match the current writers/readers. |
| Benchmark baseline | `cargo bench -p cortex-engine --bench core_baseline` runs without external services. |
| Dashboard smoke | `make dashboard-check`, `make dashboard-smoke`, and `make dashboard-screenshots` pass; CI uploads desktop/mobile dashboard artifacts. |
| ANN fixture gate | `make ann-fixture-check`, `make ann-drift-check`, `make ann-external-check`, `make ann-metric-matrix-check`, and `make ann-corpus-smoke-check` pass; CI uploads `target/ann/*report.json`, `target/ann/corpus-runs/**`, and `target/ann/release-baselines/**`. |
| ANN release package | `make ann-release-evidence-check` produces and validates `.tar.gz` release assets for the smoke corpus and demo-domain corpus, with `package_manifest.json`, SHA-256 file checksums, `history.json`, generated ground truth, and `production_safe=true`. |
| Query safety | AQL retrieve respects AgentView masks and candidate mappings. |
| ContextPack v1 | AQL-to-ContextPack tests pass for budget, explain details, source refs, and citation anomalies. |
| Docs | README, Core Alpha docs, invariants, failure scenarios, and task pools are current. |

## Release Command

```bash
git tag -a v0.1.0-core-alpha -m "CortexDB Core Alpha"
git push origin v0.1.0-core-alpha
```

Pushing a `v*` tag also runs the `Release` workflow. `make release-check`
invokes `make production-evidence-sweep`, `make backup-offsite-check`,
`make crash-fault-check`, `make chaos-restart-check`,
`make replication-lifecycle-check`, `make smoke-test`, and
`make sdk-smoke-test`. Those gates validate OpenAPI contracts, ANN baseline
archives, backup/restore drill evidence, replication partition and lifecycle
evidence, offsite backup staging, crash/fault repair evidence, process-level
kill/restart evidence, and final CLI/SDK smoke paths before the tag should be
cut. The workflow attaches the ANN `.tar.gz` baseline package to the GitHub
Release as a durable release asset.

## Latest Local Gate Evidence

On 2026-05-26, `make alpha-check` passed locally. That covered workspace check,
all-features tests, formatting, clippy with `-D warnings`, SDK smoke checks, the
core benchmark matrix, and the investment projects demo script.

## Explicit Non-Goals For This Tag

- Production BM25 ranking.
- Persistent approximate vector indexes beyond exact `.acv` scan.
- Production HNSW.
- Distributed consensus.
- Document ingestion.
- LLM integration.
