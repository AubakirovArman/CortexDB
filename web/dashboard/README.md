# CortexDB Dashboard Frontend

This directory is the source of truth for the dependency-free developer
dashboard served at `/dashboard`.

The Rust server serves the built asset copy from:

```text
crates/cortex-server/assets/dashboard/v1/
```

Build and verify:

```sh
make dashboard-build
make dashboard-check
make dashboard-smoke
make dashboard-screenshots
```

Current boundary:

- no framework or bundler dependency;
- source assets live in `web/dashboard/src`;
- build output is versioned under `/dashboard/assets/v1/`;
- Playwright smoke covers asset loading, tab switching, cell put/get, and
  keyword search.
- Playwright screenshots are written to `target/dashboard/` as CI review
  artifacts for desktop and mobile viewports.

Future work is a full standalone frontend app with separate pages, screenshots,
and a broader e2e suite.
