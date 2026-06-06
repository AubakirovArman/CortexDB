# Operations Runbook Evidence

Focused gate:

```bash
make operations-runbook-check
```

Primary artifact:

```text
target/operations-runbook/report.json
```

## What It Proves

The gate verifies that [`OPERATIONS_RUNBOOK_V1.md`](OPERATIONS_RUNBOOK_V1.md) is
self-contained enough for a local single-node operator to:

- install from a release archive or build from source;
- start `cortex-server` and verify `/v1/health`;
- stop the server before backup, repair, or upgrade;
- use bearer-token authentication for protected routes;
- write, read, flush, validate, inspect stats, and run `doctor`;
- create normal, encrypted, and offsite-staged backups;
- restore and validate backup copies;
- run dry-run and best-effort repair;
- inspect/truncate WAL only through explicit tools;
- review audit logs without query/body leakage;
- run startup, shutdown, validation, backup, restore, repair, upgrade, and
  incident-response flows;
- run deployment, observability, migration, backup, and soak evidence gates.

The gate also checks that the runbook links to the supporting install, systemd,
launchd, backup/restore, RPO/RTO, upgrade, metrics, failure-scenario, CLI, and
API docs.

## Boundary

This is local single-node operator evidence. It does not claim:

- managed service operations;
- production multi-node failover;
- KMS-backed backup custody;
- 24-hour storage soak unless `target/storage-soak-history/report.json` reports
  `twenty_four_hour_evidence.met=true`.
