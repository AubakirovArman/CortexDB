# CortexDB SDK Surface

This directory contains Beta HTTP clients for early integrations and
package publication dry-runs. They target the stable `/v1/*` `cortex-server`
surface and intentionally keep runtime dependencies small.

- `python/cortexdb_client.py`: stdlib Python client with PyPI metadata.
- `typescript/cortexdb-client.js`: fetch-based JavaScript runtime entrypoint.
- `typescript/cortexdb-client.d.ts`: TypeScript declarations.
- `typescript/cortexdb-client.ts`: TypeScript source reference.
- `crates/cortex-api-types`: shared serde wire types used by server and Rust SDK.
- `crates/cortex-sdk`: Rust HTTP client with crates.io metadata. It exports the
  blocking `CortexDbClient` by default and `AsyncCortexDbClient` when the
  `async` feature is enabled.

The SDK APIs are Beta contracts and may still receive additive changes.
Search clients expose a typed response shape with `search_mode`, `ann_report`,
and ranked `results` so ANN fallback behavior is visible to callers.
ANN evaluation clients expose the `/v1/search/ann-evaluate` contract as a typed
response with exact top-k, ANN top-k, overlap, and fixed-point `recall_q16`.
`ann_report` also exposes `recall_q16`, `min_recall_q16`,
`hnsw_ef_construction`, `production_safe`, `fallback_performed`, and
`slo_violations` so clients can enforce ANN/HNSW recall, fallback, graph-shape,
and visit-budget guardrails.
All clients support additive tenant/realm scoping so package users can target
the same per-tenant database layout exposed by `cortex-server` and `/dashboard`.
All clients also expose AQL builder helpers for `RETRIEVE CONTEXT`,
`VERIFY FACT`, and `REMEMBER` so common integrations do not have to assemble
query strings by hand.
`publish/check.sh` validates Python bytecode/tests/wheel packaging, Rust tests
with all Rust SDK features,
`cortex-api-types` packaging, SDK version consistency, tenant routing, ANN
evaluation surface presence, shared API type contracts, and npm package
dry-runs when npm is installed.
OpenAPI is the source of truth for generated schema-shaped SDK type artifacts:
`scripts/generate_openapi_sdk_types.py` emits
`typescript/cortexdb-client/generated/openapi-types.ts` and
`python/_cortexdb_client/generated/openapi_types.py` from `docs/openapi.yaml`.
The generated types use the `OpenApi*` prefix so they can coexist with the
hand-written ergonomic client models. Rust keeps using the shared
`cortex-api-types` wire structs instead of a second generated Rust type layer;
`make openapi-contract-check` still gates Rust/server/schema drift through live
response validation, error taxonomy checks, generated SDK type freshness, and
SDK surface drift checks.
Publish order matters for Rust: publish `cortex-api-types` first, then rerun
with `CORTEX_API_TYPES_PUBLISHED=1` so `cortex-sdk` package verification can
resolve the new dependency from crates.io.
`make sdk-contract-check` validates live API compatibility by building the
current `cortex-server` binary and running Python, TypeScript, and Rust SDK
smoke tests against real `/v1/*` responses. The smoke contract covers health,
put/get, search, stats, validate, AQL, Context Pack, Verify Fact, Remember,
ingest text, and structured error decoding for `invalid_aql`, `not_found`, and
`invalid_tenant`.
`sdk/release-manifest.json` records the package names, registries, dry-run
commands, deprecation policy, and manual tag-gated publish policy.
`make sdk-release-contract-check` validates that lifecycle contract without
building packages.
`make sdk-release-artifacts-check` packages Rust, Python, and TypeScript
examples into a checksummed tarball under `target/sdk-release-artifacts/` so
the release train carries runnable examples alongside package dry-runs.

Use `make sdk-check` for the local gate. The GitHub `SDK Release` workflow runs
the same preflight on SDK changes and can publish all three packages manually
from a version-matching `v*` tag through the protected `sdk-release`
environment after registry credentials are configured. See
[`docs/archive/SDK_RELEASE.md`](../docs/archive/SDK_RELEASE.md).
