# SDK Release Process

Unified CortexDB public-surface versioning rules are defined in
[`VERSIONING_POLICY.md`](VERSIONING_POLICY.md). This document covers the SDK
release train and registry guardrails.

CortexDB publishes three client surfaces from one versioned source tree plus one
Rust support crate that must be published before the Rust SDK:

- Rust support crate: `crates/cortex-api-types` as `cortex-api-types`
- Python package: `sdk/python` as `cortexdb-client`
- TypeScript package: `sdk/typescript` as `@cortexdb/client`
- Rust crate: `crates/cortex-sdk` as `cortexdb-sdk`

Rust and TypeScript package versions use the canonical workspace version in the
root `Cargo.toml`. Python prerelease packages use the PEP 440 spelling of the
same release when needed; for example, workspace `0.2.0-beta.2` maps to
Python `0.2.0b2`.

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

For SDK example release artifacts, run:

```bash
make sdk-release-artifacts-check
```

This packages the Rust, Python, and TypeScript examples plus SDK quickstart and
lifecycle docs into:

```text
target/sdk-release-artifacts/cortexdb-sdk-examples-<version>.tar.gz
target/sdk-release-artifacts/cortexdb-sdk-examples-<version>.tar.gz.sha256
target/sdk-release-artifacts/report.json
```

The SDK examples artifact is part of the release train evidence. It lets users
download the same minimal examples that the release gates validate locally.

For registry publication guardrails, run:

```bash
make sdk-registry-gate-check
```

This writes:

```text
target/sdk-registry-gate/report.json
```

The registry gate proves that SDK publication is manual-only, tag-gated, bound
to the protected `sdk-release` environment, and wired to PyPI trusted
publishing, npm, and crates.io commands. It does not claim that public registry
publication has happened.

For live API compatibility, run:

```bash
make sdk-contract-check
```

This builds the current `cortex-server` debug binary and runs real request
smoke tests for all three SDKs:

- Python: `scripts/sdk_smoke_test.py`
- TypeScript: `scripts/sdk_ts_smoke_test.mjs`
- Rust: `cargo run -p cortexdb-sdk --example live_contract`

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

1. Create and push a version tag, for example `v0.2.0-beta.2`.
2. Open the `SDK Release` workflow in GitHub Actions.
3. Select the tag ref.
4. Run the workflow with `publish=true`.

The publish job is skipped unless all of these are true:

- The workflow was started with `workflow_dispatch`.
- The selected ref is a tag beginning with `v`.
- The tag version must match the workspace version, for example
  `v0.2.0-beta.2` for workspace version `0.2.0-beta.2`.
- Python package metadata may use the PEP 440-normalized spelling of that same
  release, for example `0.2.0b2`.
- `publish=true` was explicitly set.
- The protected `sdk-release` environment approves the deployment.
- Registry credentials are configured.
- Rust packages publish in order: `cortex-api-types` first, then
  `cortexdb-sdk`.

The local release train gate is:

```bash
make sdk-e2e-release-check
```

It includes the registry gate, SDK examples artifact packaging, OpenAPI/SDK
contract checks, deprecation policy checks, and live local server e2e evidence.

## Required Registry Configuration

- PyPI: `PYPI_API_TOKEN` in the protected `sdk-release` environment, used by
  `pypa/gh-action-pypi-publish`.
- npm: `NPM_TOKEN` repository secret with publish permission for
  `@cortexdb/client`.
- crates.io: `CARGO_REGISTRY_TOKEN` repository secret with publish permission
  for `cortex-api-types` and `cortexdb-sdk`.

## Release Discipline

## Beta Compatibility Policy

For the `v0.2.0-beta.2` target, the SDKs are treated as beta developer/API
contracts:

- Additive endpoint and response-field coverage can ship in patch releases when
  existing typed methods remain backward compatible.
- Breaking SDK or HTTP contract changes require a version bump and release notes
  in both `CHANGELOG.md` and `docs/API_CHANGELOG.md`.
- Deprecated route aliases and removal windows are governed by
  [`SDK_DEPRECATION_POLICY.md`](SDK_DEPRECATION_POLICY.md).
- Public registry publication is not claimed until the manual tag-gated
  workflow runs with registry credentials or trusted publishing.

Current registry status is tracked in
[`SDK_PUBLICATION_STATUS.md`](SDK_PUBLICATION_STATUS.md).
