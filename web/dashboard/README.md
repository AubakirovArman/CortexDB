# CortexDB Dashboard Frontend

This directory is the source of truth for the dependency-free developer
dashboard served at `/dashboard`.

The Rust server serves the built asset copy from:

```text
crates/cortex-server/assets/dashboard/v1/
```

The standalone static build is written to:

```text
web/dashboard/dist/
```

It can be served by any static file server from the `dist` root. The checked
artifact keeps `/dashboard/assets/v1/` paths so it mirrors the server route
layout while remaining independent of the Rust crate.

Build and verify:

```sh
make dashboard-build
make dashboard-check
make dashboard-standalone-check
make dashboard-standalone-smoke
make dashboard-release-check
make dashboard-smoke
make dashboard-screenshots
```

Current coverage:

- no framework or bundler dependency;
- source assets live in `web/dashboard/src`;
- standalone build output lives in `web/dashboard/dist`;
- build output is versioned under `/dashboard/assets/v1/`;
- `dashboard_manifest.json` declares the frontend contract: stack, release
  channel, asset root, route IDs, route entrypoints, and session policy;
- bearer tokens are memory-only and cleared from the password field after
  apply; tenant selection is the only persisted session value;
- standalone smoke serves `web/dashboard/dist` over HTTP and verifies the
  expected route-shaped asset paths plus the frontend manifest;
- views are addressable through route-level URLs such as `/dashboard/overview`,
  `/dashboard/cells`, `/dashboard/search`, `/dashboard/storage`, and
  `/dashboard/cluster`, so browser back/forward and copied
  dashboard links preserve the selected page;
- the standalone build writes per-route HTML entrypoints under
  `web/dashboard/dist/dashboard/<route>/index.html`;
- `make dashboard-release-check` packages the standalone build into
  `target/dashboard/dashboard-v1.tar.gz` and validates the archive manifest,
  frontend stack, route manifest, file sizes, and SHA-256 checksums before it
  can be uploaded as a release artifact;
- views cover Overview, Cells, Search, ANN, AQL, Context, Verify, Ingest,
  Storage, and Cluster;
- Search includes both result execution and explain output;
- Storage and Cluster views expose validation, metrics, flush/compact,
  cluster status, and ANN metrics;
- request failures are surfaced through a visible status banner plus JSON
  details;
- Playwright smoke covers asset loading, route switching, session controls,
  cell put/get, keyword search, search explain, AQL, ContextPack, Verify,
  Ingest, ANN evaluation, storage validation, cluster status, and error states.
- Playwright screenshots are written to `target/dashboard/` as CI review
  artifacts for desktop and mobile viewports.

Future work is a fuller standalone frontend app with a broader page-specific
workflow suite and visual regression coverage.
