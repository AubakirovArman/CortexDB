# SDK Publication Status

Status: beta-ready release lifecycle, local dry-run and e2e evidence complete;
public registry publication is not claimed until a tag-gated release job is run
with registry credentials.

## Packages

| SDK | Package | Registry | Current status |
| --- | --- | --- | --- |
| Rust | `cortex-sdk` | crates.io | dry-run/package gate only |
| Python | `cortexdb-client` | PyPI | wheel/build gate only |
| TypeScript | `@cortexdb/client` | npm | pack dry-run gate only |

## Beta Publication Rule

The SDKs are part of the `v0.2.0-beta.1` developer/API beta contract only after:

1. `make sdk-check` passes.
2. `make sdk-e2e-release-check` passes.
3. `make openapi-contract-check` passes.
4. The release tag matches the workspace version.
5. The manual `sdk-release` GitHub environment approves publication.
6. Registry credentials or trusted publishing are configured outside the repo.

## Compatibility Policy

- Additive fields and endpoints are allowed in beta patch releases.
- Breaking SDK or HTTP contract changes require a version bump.
- Deprecated API aliases must be listed in `docs/SDK_DEPRECATION_POLICY.md`.
- Changelog entries are required for breaking changes and deprecations.
- SDKs must keep Rust, Python, and TypeScript package versions aligned with the
  workspace version.

## Non-Claims

This repository does not claim that the SDK packages are currently published in
public registries. The checked evidence proves local package construction,
example packaging, OpenAPI compatibility, and live local server e2e behavior.
