# Engine Panic Audit

Status: Epic 35 engine panic audit.

Core production paths should return typed errors for invalid input, corrupt
storage, closed writers, and malformed local files. They should not rely on
`unwrap`, `expect`, or `panic!` in normal database, storage, or engine code.

## Scope

The panic audit scans production Rust sources under:

- `crates/cortex-core/src`
- `crates/cortex-engine/src`
- `crates/cortex-storage/src`

It intentionally excludes:

- Rust doc examples;
- `#[cfg(test)] mod tests`;
- `src/bin` developer/check tooling;
- standalone test files.

Those excluded areas may use `unwrap`/`expect` for test clarity, but production
paths must use explicit error returns or control-flow checks.

## Gate

```bash
make engine-panic-audit-check
```

The gate writes:

```text
target/engine-panic-audit/report.json
```

It fails if a production source line contains:

```text
.unwrap()
.expect(
panic!
```

## Current Fixes

- Storage binary readers now convert fixed-width byte slices through fallible
  array helpers and return the relevant `StorageError` variant.
- WAL codec decode helpers return `InvalidWalRecord` or `InvalidWalFileHeader`
  instead of panicking on width mismatch.
- WAL writer append paths branch on the optional file handle instead of calling
  `unwrap`.
- Tool-cell name extraction no longer uses `unwrap` and tolerates missing
  `name=...` metadata.
