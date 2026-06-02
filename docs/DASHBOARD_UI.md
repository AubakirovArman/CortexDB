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
make dashboard-product-check
make dashboard-smoke
make dashboard-screenshots
```

`dashboard-standalone-smoke` serves `web/dashboard/dist` through a local static
HTTP server and verifies the index plus route-shaped asset paths without
starting `cortex-server`.

`dashboard-smoke` starts a local `cortex-server` and drives the console through
asset loading, tab switching, session controls, cell put/get, keyword search,
search explain, AQL, ContextPack, Verify, Ingest, ANN evaluation, storage
validation, cluster status, role-aware permission messaging, visible request
issue cards, and client-side numeric validation before malformed requests reach
the API. ANN evaluation also
renders a compact report card view for recall, production safety, fallback,
SLO violations, graph shape, and HNSW tuning knobs.
The dashboard also has a local read-only mode that blocks mutating actions
before they reach the API, an operational status panel for visible health,
stats, storage validation, metrics reachability, backup posture, last request
error state, and incident checks, and a Permissions route that explains the
active tenant, role, token state, admin/data capabilities, and local write guard
state.
The Overview route includes an Audit readiness panel that keeps incident review
operators on the safe path: audit logs remain file-backed, raw audit events are
not rendered in the browser, and the panel points operators to the CLI
redaction-check workflow.
### ContextPack Explorer

ContextPack responses render a separate report view for token budget usage,
selected cells, citations, anomalies, and per-cell explain metadata so pack
quality can be reviewed without reading raw JSON. The Context route splits this
into an operator-facing explorer:

- Context cells: selected cell payload previews, token estimates, citation
  status, source refs, matched terms, and why-selected text.
- Citation explorer: one row per selected cell with citation/source-ref
  visibility, including missing-citation cases.
- Explain explorer: per-cell score, BM25 base, source-trust category/bonus,
  redundancy penalty, and score component reasons.
- Anomaly explorer: token overload, missing citation, redundancy, and
  `why_excluded` messages when the engine reports excluded cells.
Storage validation responses render health cards for manifest/WAL status,
checked segments, checked cells, index coverage, safe WAL truncate offset, and
validation errors. The Overview operational status view combines those
validation cards with stats, metrics, request-error triage, and backup posture.
Backups stay outside the browser as operator CLI workflows; the dashboard points
to `make backup-restore-production-pack-check`, `cortexdb backup`,
`cortexdb backup-drill`, `cortexdb backup-encrypted`, and
`cortexdb backup-offsite-stage`.
Cell, Ingest, and Cluster responses render their own report views for sequence
numbers, lookup payload previews, ingest counts, job state, distributed mode,
replication factor, and node list, so operators can review normal operations
without scanning raw JSON.
Report rendering is split by responsibility:

```text
reporting_common.js      shared DOM helpers
reporting_retrieval.js   Search, AQL, Verify, ContextPack
reporting_operations.js  Cell, Ingest, Cluster, Storage, ANN, errors
reporting.js             compatibility facade
```

`dashboard-screenshots` starts the same local server and writes review artifacts:

```text
target/dashboard/dashboard-desktop.png
target/dashboard/dashboard-mobile.png
target/dashboard/summary.json
```

The Rust CI workflow uploads those files as the `dashboard-screenshots`
artifact on the stable toolchain job.

## Frontend Contract

The dashboard stack is intentionally fixed for Core Alpha:

```text
dependency-free-static-html-css-js
```

`web/dashboard/src/dashboard_manifest.json` is the checked source of truth for
the standalone frontend contract. It declares the release channel, asset root,
route IDs, route entrypoints, and the session policy. Tenant selection is
persisted in `sessionStorage`; bearer tokens are memory-only and cleared from
the input after apply. `make dashboard-build` copies that manifest into the
server asset tree and standalone dist. `make dashboard-release-check` packages
and validates it alongside the archive manifest and SHA-256 file checksums.
Form controls use native constraints plus synchronized `aria-invalid` state so
keyboard and assistive-technology users get the same validation feedback as the
visual UI.
Session capability detection renders a permission report for limited, data, and
admin access. Request failures are normalized into a visible Request Issue
report with request label, HTTP status, server code, message, and a
route-aware operator action while the raw response stays available in the JSON
output panel.
The read-only mode is a dashboard-local safety switch. It does not replace
server authorization; it prevents accidental cell writes, tombstones, ingest,
flush, and compact actions from being sent while an operator is inspecting the
database.
Search, AQL, and Verify success responses also render compact report views for
result count, top cells, verdict, evidence, contradictions, guards, and numeric
conflicts.
Report rendering lives outside `app.js` to keep product-facing formatting
separate from request/session control logic. Each renderer reacts only to its
typed response shape, so unrelated responses do not overwrite route-specific
cards.

## Boundary

This is not the final product UI. It is the checked, reviewable bridge between
the Core Alpha HTTP API and the future standalone frontend product. The current
shell already exposes Overview, Permissions, Cells, Search, ANN, AQL, Context,
Verify, Ingest, Storage, and Cluster views from route-level static entrypoints.
The current standalone artifact is a dependency-free static build under
`web/dashboard/dist`; the next UI layer should add broader page-specific
workflow coverage and visual regression coverage.

## Operational View

For Beta Release Candidate evidence, the dashboard counts as an operational
view only for local developer/operator workflows: health, metrics, validation,
storage status, backup posture, ANN evaluation, ingestion jobs, cluster status,
permissions review, audit readiness, local read-only guard state, and typed
error reports. It is not yet a full incident-management console, RBAC
administration UI, audit log browser, or production observability product.
