# Contributing to CortexDB

Thank you for your interest in CortexDB! This document will help you get started.

## Quick Start

For a short first-session path, start with
[`docs/CONTRIBUTOR_ONBOARDING.md`](docs/CONTRIBUTOR_ONBOARDING.md).

```bash
# Clone the repository
git clone https://github.com/AubakirovArman/CortexDB.git
cd CortexDB

# Run the 15-minute contributor smoke path
make contributor-onboarding-check

# Run the full quality gate
make release-check

# Run only tests
cargo test --workspace --all-features

# Run the demo
make demo
```

## Project Structure

| Crate | Purpose |
|-------|---------|
| `crates/cortex-core` | In-memory MVCC MemTable, cell versions |
| `crates/cortex-storage` | WAL, segments, bitmap/lexical/vector indexes |
| `crates/cortex-engine` | Database loop, compaction, AQL, ContextPack, VERIFY FACT |
| `crates/cortex-aql` | AQL parser, AST, binder, bytecode VM |
| `crates/cortex-server` | Async HTTP API (Axum/Tokio) with per-tenant actors |
| `crates/cortex-cli` | Local CLI tool |
| `crates/cortex-sdk` | Rust HTTP client |

## Before Submitting

1. Run `make release-check` — it must pass.
2. Follow existing code style (`cargo fmt`, `cargo clippy -D warnings`).
3. Add tests for new functionality.
4. Update OpenAPI schema (`docs/openapi.yaml`) if you change the API.
5. Update snapshot tests if response shapes change.

## File Size Discipline

CortexDB uses file size as an architecture signal, not as a mechanical law. Split
code by responsibility, not by line count.

| Size | Rule |
|------|------|
| `<=300` lines | Normal target for new source files. |
| `300-600` lines | Allowed when the file has one clear responsibility and a module-level doc comment. |
| `600-1000` lines | Requires a clear split plan or review justification. |
| `>1000` lines | Ratchet zone: do not grow the file; put new code behind a responsibility-based split first. |

Tests use doubled limits (`600` soft / `1200` warning) because scenario tables
and fixtures are naturally larger. Generated files, snapshots, and golden
fixtures are excluded from the size gate.

The CI check is a ratchet:

- Existing large files are captured in `quality/file_size_baseline.json`.
- A tracked file fails the check only if it grows beyond its baseline count.
- A new source file fails if it starts above its warning limit.
- Reductions are encouraged; update the baseline only in the same PR that
  intentionally reduces or moves code.

Use:

```bash
make file-size-report
make file-size-check
```

When splitting a large file, keep the PR move-only: move code by responsibility,
preserve public paths with `pub use` where needed, and avoid behavior changes in
the same commit.

## Architecture Notes

- **Database core is blocking**. The async server wraps it in `DatabaseActor` with a bounded queue.
- **WAL is the source of truth**. All writes go to WAL first, then MemTable, then segments.
- **Tenant = directory**. Each tenant maps to a subdirectory under `realms/`.
- **Fixed-point math only**. Distance metrics use integer arithmetic (`u128::isqrt()`), no `f64` in the scoring path.

## Getting Help

- Open an issue for bugs or feature requests.
- Use [`docs/GOOD_FIRST_ISSUES.md`](docs/archive/GOOD_FIRST_ISSUES.md) to choose a
  bounded first task.
- Check `docs/` for deeper architecture and design documents.
