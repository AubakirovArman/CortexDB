# Beta Release Candidate Evidence

Last local beta RC run: 2026-06-01.

This document records the Epic 3 evidence gate from
[`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md). It focuses on whether the
current Core Alpha surface can be treated as a Beta Release Candidate for local
experiments. It is not a production release claim.

## Command

```bash
make beta-rc-check
```

The command writes:

```text
target/beta-rc/report.json
target/beta-rc/*.log
```

Result:

```text
passed
```

## Suites

| Suite | Purpose |
| --- | --- |
| `operational_docs` | Confirms operations, security, ingestion, and dashboard docs cover the beta RC surface. |
| `beta_foundation` | Reuses the Epic 2 SDK/API/ContextPack/VERIFY/Search foundation gate. |
| `backup_restore_drill` | Runs local backup/restore/readback evidence. |
| `backup_offsite_stage` | Runs validated local offsite-staging evidence. |
| `security_model_tests` | Runs HTTP security tests for auth, limits, tenant validation, and audit redaction. |
| `auth_policy_tests` | Runs admin/data token split and agent-scope policy tests. |
| `ingestion_jobs` | Runs ingestion endpoint and job lifecycle tests. |
| `dashboard_operational_view` | Builds, packages, and validates the dashboard release artifact. |

## Boundary

This gate proves:

- backup and restore evidence is locally repeatable;
- operations/security/ingestion/dashboard docs are present;
- the operations guide has a first-10-minutes path, consolidated install /
  validate / backup / restore / repair / metrics / upgrade / rollback runbook
  links, and troubleshooting for stale locks, corrupt WAL, corrupt segment,
  busy actor queue, failed auth, and tenant errors;
- security and auth policy tests pass;
- ingestion job lifecycle tests pass;
- dashboard release packaging is repeatable.

It does not prove:

- production security certification;
- managed service operational readiness;
- full product web UI maturity;
- long-running beta traffic SLO history.
