# SDK E2E Release Evidence

Last local SDK e2e release run: 2026-06-01, passed.

Run:

```bash
make sdk-e2e-release-check
```

Primary artifact:

```text
target/sdk-e2e-release/report.json
target/sdk-release-artifacts/report.json
target/sdk-release-artifacts/cortexdb-sdk-examples-0.1.0.tar.gz
```

## Coverage

This gate covers:

- Rust, Python, and TypeScript SDK package metadata;
- live local server smoke through all three SDKs;
- Bearer auth success and structured `401 unauthorized` failures through all
  three SDKs;
- SDK release manifest and manual publish controls;
- SDK examples artifact for Rust, Python, and TypeScript examples;
- SDK deprecation policy;
- quickstart and release documentation.

## Latest Local Checks

```text
live_sdk_contract: true
release_contract: true
release_artifacts: true
deprecation_policy: true
quickstart: true
packages: rust, python, typescript
```

The live contract smoke covers health, put/get, search, stats, validate, AQL,
ContextPack, VERIFY FACT, REMEMBER, ingest text, tenant routing, Bearer auth,
and structured errors (`unauthorized`, `invalid_aql`, `not_found`,
`invalid_tenant`) for Rust, Python, and TypeScript SDKs.

## Boundary

This gate proves local SDK compatibility and release-train wiring. Actual public
registry publication remains a manual release operation and is not required for
local Core Alpha evidence.
