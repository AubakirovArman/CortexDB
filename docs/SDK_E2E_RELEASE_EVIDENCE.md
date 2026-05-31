# SDK E2E Release Evidence

Last local SDK e2e release run: 2026-05-31, passed.

Run:

```bash
make sdk-e2e-release-check
```

Primary artifact:

```text
target/sdk-e2e-release/report.json
```

## Coverage

This gate covers:

- Rust, Python, and TypeScript SDK package metadata;
- live local server smoke through all three SDKs;
- SDK release manifest and manual publish controls;
- SDK deprecation policy;
- quickstart and release documentation.

## Latest Local Checks

```text
live_sdk_contract: true
release_contract: true
deprecation_policy: true
quickstart: true
packages: rust, python, typescript
```

## Boundary

This gate proves local SDK compatibility and release-train wiring. Actual public
registry publication remains a manual release operation and is not required for
local Core Alpha evidence.
