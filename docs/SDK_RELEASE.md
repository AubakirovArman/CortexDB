# SDK Release Process

CortexDB publishes three Core Alpha client surfaces from one versioned source
tree:

- Python package: `sdk/python` as `cortexdb-client`
- TypeScript package: `sdk/typescript` as `@cortexdb/client`
- Rust crate: `crates/cortex-sdk` as `cortex-sdk`

All three versions must match the workspace version in the root `Cargo.toml`.

## Preflight

Run the SDK package gate before cutting a release:

```bash
make sdk-check
```

This invokes `sdk/publish/check.sh`, which verifies:

- SDK release manifest consistency (`sdk/release-manifest.json`).
- Python bytecode compilation and unit tests.
- Python wheel build.
- TypeScript/JavaScript syntax and package dry-run when `npm` is installed.
- Rust SDK tests and `cargo package`.
- Cross-SDK version consistency.
- OpenAPI, changelog, package metadata, and publish workflow alignment.
- Deprecation policy and breaking-change changelog coverage.
- Tenant/realm routing coverage.
- ANN evaluation contract coverage.

For the metadata-only gate, run:

```bash
make sdk-release-contract-check
```

This rejects version drift, missing package metadata, missing changelog anchors,
unsafe publish workflow changes, and tracked generated artifacts such as wheels,
`dist/`, or SDK cache directories.

It also runs `scripts/check_sdk_deprecation_policy.py`, which rejects
undocumented OpenAPI deprecations, SDK clients that call deprecated legacy route
aliases, and missing breaking-change/deprecation policy text.

For live API compatibility, run:

```bash
make sdk-contract-check
```

This builds the current `cortex-server` debug binary and runs real request
smoke tests for all three SDKs:

- Python: `scripts/sdk_smoke_test.py`
- TypeScript: `scripts/sdk_ts_smoke_test.mjs`
- Rust: `cargo run -p cortex-sdk --example live_contract`

The scripts set `CORTEXDB_SERVER_BIN` so each SDK talks to the freshly built
server, not a stale release binary left in `target/release`.

The live compatibility gate validates both successful typed response decoding
and structured error decoding. At minimum, each SDK must prove it can decode
`invalid_aql`, `not_found`, and `invalid_tenant` responses from the live server.
The SDK unit tests also decode the full Core Alpha error taxonomy documented in
[`API_ERROR_TAXONOMY.md`](API_ERROR_TAXONOMY.md), including `rate_limited` and
`service_unavailable`.

## GitHub Workflow

`.github/workflows/sdk-release.yml` runs the same preflight on SDK-relevant
pull requests and pushes. The publish job is intentionally manual-only:

1. Create and push a version tag, for example `v0.1.0-core-alpha`.
2. Open the `SDK Release` workflow in GitHub Actions.
3. Select the tag ref.
4. Run the workflow with `publish=true`.

The publish job is skipped unless all of these are true:

- The workflow was started with `workflow_dispatch`.
- The selected ref is a tag beginning with `v`.
- The tag version must match the workspace version, for example
  `v0.1.0` or `v0.1.0-core-alpha` for workspace version `0.1.0`.
- `publish=true` was explicitly set.
- The protected `sdk-release` environment approves the deployment.
- Registry credentials/trusted publishing are configured.

## Required Registry Configuration

- PyPI: trusted publishing for the repository, used by
  `pypa/gh-action-pypi-publish`.
- npm: `NPM_TOKEN` repository secret with publish permission for
  `@cortexdb/client`.
- crates.io: `CARGO_REGISTRY_TOKEN` repository secret with publish permission
  for `cortex-sdk`.

## Release Discipline

The SDKs are Core Alpha contracts. Breaking changes require a version bump and
release notes in both `CHANGELOG.md` and `docs/API_CHANGELOG.md`. Additive
endpoint coverage can ship in patch releases when the server API remains
backward compatible. Deprecated route aliases and removal windows are governed
by [`SDK_DEPRECATION_POLICY.md`](SDK_DEPRECATION_POLICY.md).
