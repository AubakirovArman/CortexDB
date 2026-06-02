# Dashboard Product UI Evidence

Last local dashboard product UI run: 2026-06-01, passed.

Run:

```bash
make dashboard-product-check
make dashboard-smoke
make dashboard-screenshots
```

Primary artifacts:

```text
target/dashboard/product-ui-report.json
target/dashboard/dashboard-v1.tar.gz
target/dashboard/dashboard-desktop-1440x1000-*.png
target/dashboard/dashboard-mobile-390x900@2x-*.png
target/dashboard/summary.json
```

## Coverage

This gate covers:

- local read-only mode that blocks mutating dashboard actions before they reach
  the API;
- an operational status panel for health, stats, validation, metrics, backup
  posture, last request error state, and visible incidents;
- a version compatibility section for API, SDK, storage format, and migration
  matrix contract state from `/v1/compatibility`;
- an incident timeline that classifies audit, rate-limit, storage, and backup
  events with severity, source, message, and action guidance;
- a ContextPack Explorer covering selected cells, citations, explain score
  components, anomalies, and `why_excluded` messages;
- a Verification Explorer covering mixed evidence, supporting and
  contradicting evidence, numeric conflicts, and guard messages;
- an audit readiness panel that keeps raw audit events out of the browser and
  points operators to file-backed CLI redaction checks;
- a permissions view for tenant, active role, token state, selected scope
  probes, AgentView enforcement posture, admin/data capabilities, and local
  write guard state;
- dashboard release packaging and screenshot artifact wiring.

## Latest Local Checks

```text
read_only_mode: true
operational_status: true
operational_compatibility_versions: true
operational_backup_posture: true
operational_last_error: true
incident_timeline: true
context_pack_explorer: true
verification_explorer: true
audit_readiness: true
permissions_view: true
permissions_agent_view_posture: true
release_artifacts: true
standalone_package: target/dashboard/dashboard-v1.tar.gz
screenshots: 22 route/viewport captures including permissions
```

## Boundary

This is still a dependency-free operational dashboard, not the final product
web UI. The final UI layer can add richer workflow navigation and visual
regression review after these operator-safety surfaces are stable.
