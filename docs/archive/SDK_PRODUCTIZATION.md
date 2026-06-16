# SDK Productization

Status: local SDK productization gate plus published public registry evidence.

CortexDB maintains three SDK surfaces:

- Rust: `crates/cortex-sdk`
- Python: `sdk/python`
- TypeScript: `sdk/typescript`

## Local Gate

Run:

```bash
make sdk-productization-check
```

The gate depends on:

```text
make sdk-e2e-release-check
```

That release check includes:

- SDK release contract validation.
- SDK deprecation policy validation.
- SDK example artifact packaging.
- SDK registry gate validation.
- Live local server contract tests for Rust, Python, and TypeScript.

## Python SDK

Covered local evidence:

- `sdk/python/pyproject.toml`
- `sdk/python/cortexdb_client.py`
- `sdk/python/test_cortexdb_client.py`
- `sdk/python/examples/basic.py`
- wheel/build artifact evidence through `make sdk-e2e-release-check`

## TypeScript SDK

Covered local evidence:

- `sdk/typescript/package.json`
- `sdk/typescript/cortexdb-client.ts`
- `sdk/typescript/cortexdb-client.d.ts`
- `sdk/typescript/test.js`
- `sdk/typescript/examples/basic.mjs`
- package dry-run evidence through `make sdk-e2e-release-check`

## Rust SDK

Covered local evidence:

- `crates/cortex-sdk/Cargo.toml`
- `crates/cortex-sdk/src/lib.rs`
- `crates/cortex-sdk/src/types.rs`
- `crates/cortex-sdk/examples/basic.rs`
- `crates/cortex-sdk/examples/live_contract.rs`
- `cargo package` evidence through `make sdk-e2e-release-check`

## Publication Evidence

Published public registry evidence is tracked in
[`SDK_PUBLICATION_STATUS.md`](SDK_PUBLICATION_STATUS.md):

- PyPI for `cortexdb-sdk`
- npm for `cortexdb-sdk`
- crates.io for `cortexdb-sdk`

The local productization gate proves packaging readiness, examples, typed
contracts, registry-gate wiring, and local live-server compatibility. Future
release publication is still manual-only and tag-gated.
