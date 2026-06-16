# SDK Publication Status

Status: beta-ready release lifecycle, local dry-run and e2e evidence complete;
public registry publication is not claimed until a tag-gated release job or
equivalent registry publish run completes with credentials.

Latest audit: `v0.2.0-beta.2` is tagged and released, and local SDK gates pass.
The `sdk-release` GitHub environment expects `PYPI_API_TOKEN`, `NPM_TOKEN`, and
`CARGO_REGISTRY_TOKEN`. Public SDK publication remains blocked until those
credentials are configured and the publish run succeeds.

## Packages

| SDK | Package | Registry | Current status |
| --- | --- | --- | --- |
| Rust API types | `cortex-api-types` | crates.io | dry-run/package gate only |
| Rust | `cortexdb-sdk` | crates.io | dry-run/package gate only |
| Python | `cortexdb-client` | PyPI | wheel/build gate only |
| TypeScript | `@cortexdb/client` | npm | pack dry-run gate only |

## Beta Publication Rule

The SDKs are part of the `v0.2.0-beta.2` developer/API beta contract only after:

1. `make sdk-check` passes.
2. `make sdk-e2e-release-check` passes.
3. `make openapi-contract-check` passes.
4. `make sdk-registry-gate-check` writes
   `target/sdk-registry-gate/report.json`.
5. The release tag matches the workspace version.
6. The manual `sdk-release` GitHub environment approves publication.
7. Registry credentials are configured outside the repo.
8. The Rust crates publish in order: `cortex-api-types` first, then
   `cortexdb-sdk`.

The registry gate is local evidence that publication is manual-only and
tag-gated. Public registry publication is not claimed until the manual release
job actually runs from the tag and the registry pages exist.

Rust publication order matters: publish `cortex-api-types` first, then
`cortexdb-sdk`. The SDK release workflow keeps the `cortexdb-sdk` crates.io
dry-run behind an explicit `CORTEX_API_TYPES_PUBLISHED=1` repository variable
so preflight can pass before the first support-crate publication.

## Compatibility Policy

- Additive fields and endpoints are allowed in beta patch releases.
- Breaking SDK or HTTP contract changes require a version bump.
- Deprecated API aliases must be listed in `docs/SDK_DEPRECATION_POLICY.md`.
- Changelog entries are required for breaking changes and deprecations.
- SDKs must keep Rust, Python, and TypeScript package versions aligned with the
  workspace release; Python may use the PEP 440 prerelease spelling required by
  PyPI, for example `0.2.0b2` for workspace `0.2.0-beta.2`.

## Non-Claims

This repository does not claim that the SDK packages are currently published in
public registries until registry pages exist for this version. The checked
evidence proves local package construction, example packaging, OpenAPI
compatibility, and live local server e2e behavior.
