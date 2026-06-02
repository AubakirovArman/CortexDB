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

## Architecture Notes

- **Database core is blocking**. The async server wraps it in `DatabaseActor` with a bounded queue.
- **WAL is the source of truth**. All writes go to WAL first, then MemTable, then segments.
- **Tenant = directory**. Each tenant maps to a subdirectory under `realms/`.
- **Fixed-point math only**. Distance metrics use integer arithmetic (`u128::isqrt()`), no `f64` in the scoring path.

## Getting Help

- Open an issue for bugs or feature requests.
- Use [`docs/GOOD_FIRST_ISSUES.md`](docs/GOOD_FIRST_ISSUES.md) to choose a
  bounded first task.
- Check `docs/` for deeper architecture and design documents.
