# Dashboard UI

The current dashboard is a dependency-free developer console served at
`/dashboard`. Its source of truth is `web/dashboard/src`, and the built static
assets are checked into `crates/cortex-server/assets/dashboard/v1`.

## Commands

```sh
make dashboard-build
make dashboard-check
make dashboard-smoke
make dashboard-screenshots
```

`dashboard-smoke` starts a local `cortex-server` and drives the console through
asset loading, tab switching, cell put/get, keyword search, search explain,
storage validation, and cluster status.

`dashboard-screenshots` starts the same local server and writes review artifacts:

```text
target/dashboard/dashboard-desktop.png
target/dashboard/dashboard-mobile.png
target/dashboard/summary.json
```

The Rust CI workflow uploads those files as the `dashboard-screenshots`
artifact on the stable toolchain job.

## Boundary

This is not the final product UI. It is the checked, reviewable bridge between
the Core Alpha HTTP API and the future standalone frontend product. The current
shell already exposes Overview, Cells, Search, ANN, AQL, Context, Verify,
Ingest, Storage, and Cluster views from static assets. The next UI layer should
turn these checked views into an independently built frontend product with
route-level pages, richer error states, and broader visual regression coverage.
