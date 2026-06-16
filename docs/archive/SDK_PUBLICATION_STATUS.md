# SDK Publication Status

Status: beta-ready release lifecycle, local dry-run and e2e evidence complete;
public registry publication is blocked on registry account settings.

Latest audit: `v0.2.0-beta.2` is tagged and released, local SDK gates pass, and
the `sdk-release` GitHub environment has `PYPI_API_TOKEN`, `NPM_TOKEN`, and
`CARGO_REGISTRY_TOKEN` configured. A real publication attempt on 2026-06-16
reached the registries but was blocked externally:

- crates.io rejected `cortex-api-types` because the token owner has no verified
  email address.
- PyPI rejected `cortexdb-client==0.2.0b2` with `403 Forbidden` for the token.
- npm rejected `@cortexdb/client@0.2.0-beta.2` because publishing requires 2FA
  or a granular access token with 2FA bypass enabled.

No public SDK publication is claimed until those account settings are fixed and
the publish run succeeds.
The public registry publication is not claimed for `v0.2.0-beta.2`.

## Packages

| SDK | Package | Registry | Current status |
| --- | --- | --- | --- |
| Rust API types | `cortex-api-types` | crates.io | blocked: verified email required |
| Rust | `cortexdb-sdk` | crates.io | blocked until `cortex-api-types` publishes |
| Python | `cortexdb-client` | PyPI | blocked: token lacks publish permission |
| TypeScript | `@cortexdb/client` | npm | blocked: 2FA/granular token required |

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
