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
```

Current boundary:

- no framework or bundler dependency;
- source assets live in `web/dashboard/src`;
- build output is versioned under `/dashboard/assets/v1/`;
- Playwright smoke covers asset loading, tab switching, cell put/get, and
  keyword search.

Future work is a full standalone frontend app with separate pages, screenshots,
and a broader e2e suite.
