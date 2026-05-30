# SDK Deprecation Policy

This policy protects published Rust, Python, and TypeScript clients from
silent API drift. The SDKs target `/v1/*` routes and must remain forward
compatible with additive JSON response fields.

## Deprecated Compatibility Aliases

The server keeps a few legacy aliases for early local users. SDK clients MUST
NOT expose deprecated compatibility aliases; they must call the versioned
routes listed here.

| Deprecated route | Replacement | First deprecated | Removal target |
| --- | --- | --- | --- |
| `/get` | `GET /v1/cell` | `v0.1.0-core-alpha` | no earlier than the first beta minor release |
| `/put` | `POST /v1/cell` | `v0.1.0-core-alpha` | no earlier than the first beta minor release |
| `/flush` | `POST /v1/flush` | `v0.1.0-core-alpha` | no earlier than the first beta minor release |
| `/tombstone` | `DELETE /v1/cell` | `v0.1.0-core-alpha` | no earlier than the first beta minor release |

The minimum deprecation window is one minor release after the replacement is
available and documented.

## Breaking SDK/API Changes

Breaking SDK/API changes require all of:

1. A version bump across `Cargo.toml`, OpenAPI, Rust SDK, Python package, and
   TypeScript package.
2. A `CHANGELOG.md` entry under `Changed`, `Deprecated`, or `Removed`.
3. A `docs/API_CHANGELOG.md` entry naming the affected routes, fields, or SDK
   methods.
4. A migration note with the replacement API or a reason no replacement exists.
5. A contract test update proving the new response shape or SDK method.

Non-breaking additive fields and methods still require changelog notes when they
are user-facing.
