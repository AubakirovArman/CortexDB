# SDK Publication Status

Status: beta-ready release lifecycle, local dry-run and e2e evidence complete;
Rust crates, the Python package, and the TypeScript package are published.

Latest audit: `v0.2.0-beta.2` is tagged and released, local SDK gates pass, and
the `sdk-release` GitHub environment has `PYPI_API_TOKEN`, `NPM_TOKEN`, and
`CARGO_REGISTRY_TOKEN` configured. A real publication attempt on 2026-06-16
reached the registries but was blocked externally for all three registries:

- crates.io rejected `cortex-api-types` because the token owner has no verified
  email address.
- PyPI rejected `cortexdb-client==0.2.0b2` because that name belongs to a
  different project owner.
- npm rejected the planned scoped package `@cortexdb/client@0.2.0-beta.2`
  because the account did not have publish access to the `@cortexdb` scope.

After the crates.io account email was fixed, `cortex-api-types` and
`cortexdb-sdk` were published successfully on 2026-06-16 and are visible in the
crates.io index. The original PyPI name `cortexdb-client` is owned by another
project, so the Python package was renamed to `cortexdb-sdk` and published
successfully on 2026-06-16. The planned scoped npm name `@cortexdb/client`
was not available to the token, so the TypeScript package was published as the
unscoped `cortexdb-sdk` package on 2026-06-16.

## Packages

| SDK | Package | Registry | Current status |
| --- | --- | --- | --- |
| Rust API types | `cortex-api-types` | crates.io | published: `0.2.0-beta.2` |
| Rust | `cortexdb-sdk` | crates.io | published: `0.2.0-beta.2` |
| Python | `cortexdb-sdk` | PyPI | published: `0.2.0b2` |
| TypeScript | `cortexdb-sdk` | npm | published: `0.2.0-beta.2` |

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
tag-gated. Future public registry publication is not claimed until the manual
release job actually runs from the tag and the registry pages exist.

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

This repository claims Rust crates.io publication for `cortex-api-types` and
`cortexdb-sdk`, PyPI publication for `cortexdb-sdk`, and npm publication for
`cortexdb-sdk` at `0.2.0-beta.2`. The checked evidence proves local package
construction, example packaging, OpenAPI compatibility, and live local server
e2e behavior.
