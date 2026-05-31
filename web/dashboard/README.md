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
make dashboard-smoke
make dashboard-screenshots
```

Current coverage:

- no framework or bundler dependency;
- source assets live in `web/dashboard/src`;
- standalone build output lives in `web/dashboard/dist`;
- build output is versioned under `/dashboard/assets/v1/`;
- standalone smoke serves `web/dashboard/dist` over HTTP and verifies the
  expected route-shaped asset paths;
- views are addressable through route-level URLs such as `/dashboard/overview`,
  `/dashboard/cells`, `/dashboard/search`, `/dashboard/storage`, and
  `/dashboard/cluster`, so browser back/forward and copied
  dashboard links preserve the selected page;
- the standalone build writes per-route HTML entrypoints under
  `web/dashboard/dist/dashboard/<route>/index.html`;
- views cover Overview, Cells, Search, ANN, AQL, Context, Verify, Ingest,
  Storage, and Cluster;
- Search includes both result execution and explain output;
- Storage and Cluster views expose validation, metrics, flush/compact,
  cluster status, and ANN metrics;
- request failures are surfaced through a visible status banner plus JSON
  details;
- Playwright smoke covers asset loading, route switching, cell put/get, keyword
  search, search explain, storage validation, cluster status, and error states.
- Playwright screenshots are written to `target/dashboard/` as CI review
  artifacts for desktop and mobile viewports.

Future work is a full standalone frontend app with separate pages, screenshots,
and a broader e2e suite.
