# Dashboard UI

The current dashboard is a dependency-free developer console served at
`/dashboard`. Its source of truth is `web/dashboard/src`, and the built static
assets are checked into both `web/dashboard/dist` and
`crates/cortex-server/assets/dashboard/v1`.

## Commands

```sh
make dashboard-build
make dashboard-check
make dashboard-standalone-check
make dashboard-standalone-smoke
make dashboard-smoke
make dashboard-screenshots
```

`dashboard-standalone-smoke` serves `web/dashboard/dist` through a local static
HTTP server and verifies the index plus route-shaped asset paths without
starting `cortex-server`.

`dashboard-smoke` starts a local `cortex-server` and drives the console through
asset loading, tab switching, cell put/get, keyword search, search explain,
storage validation, cluster status, and visible request error states.

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
Ingest, Storage, and Cluster views from static assets. The current standalone
artifact is a dependency-free static build under `web/dashboard/dist`; the next
UI layer should add route-level pages, a chosen frontend stack if needed, and
broader visual regression coverage.
