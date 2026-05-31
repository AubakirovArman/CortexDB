# Deployment And Upgrade Evidence

Status: local evidence gate for Epic 19.

## Goal

Prove that Core Alpha install, service, binary packaging, upgrade, rollback, and
release-upload instructions are present and internally linked.

## Local Gate

```bash
make deployment-upgrade-check
```

The gate writes:

```text
target/deployment-upgrade/report.json
```

## What The Gate Checks

- `docs/INSTALL.md` covers binary install, source build, first database, and
  validation.
- `docs/SYSTEMD.md` contains a single-node service unit and health checks.
- `docs/UPGRADE_ROLLBACK.md` covers pre-upgrade validation, backup drill,
  upgrade, rollback, and compatibility gates.
- `docs/BINARY_RELEASES.md` documents tarball contents and GitHub release
  upload behavior.
- `.github/workflows/release.yml` uploads binary archives and `.sha256` files
  on tag releases.
- `README.md` and the documentation index link the deployment docs.

## Latest Local Run

Last local run: 2026-05-31, passed.

Report:

```text
target/deployment-upgrade/report.json
```

Result:

```json
{
  "docs_checked": 4,
  "links_checked": 3,
  "release_workflow_checked": true,
  "status": "passed"
}
```
