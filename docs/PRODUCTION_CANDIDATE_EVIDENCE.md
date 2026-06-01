# Production Candidate Evidence

Last local production candidate run: 2026-05-31.

This document records the Epic 5 evidence gate from
[`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md). It prepares a production-like
single-node candidate. It does not claim CortexDB is production v1.0.

## Command

```bash
make production-candidate-check
```

The command writes:

```text
target/production-candidate/report.json
target/production-candidate/*.log
```

Result:

```text
passed
```

## Suites

| Suite | Purpose |
| --- | --- |
| `candidate_docs` | Confirms RPO/RTO, SLO, SDK/API compatibility, and upgrade/rollback docs exist. |
| `production_hardening` | Reuses the Epic 4 hardening evidence gate. |
| `backup_rpo_rto_drill` | Runs backup/restore/readback evidence for the RPO/RTO boundary. |
| `single_node_slo` | Runs local single-node performance gate; performance trend history is covered through `production_hardening`. |
| `openapi_contract` | Verifies HTTP API schema compatibility. |
| `sdk_release_contract` | Verifies SDK package metadata, version lock-step, and release workflow. |
| `sdk_deprecation_policy` | Verifies deprecated routes and SDK source compatibility policy. |
| `migration_policy` | Verifies upgrade/rollback policy wiring. |
| `migration_compatibility` | Verifies storage/API/SDK compatibility fixture. |
| `binary_release` | Builds and validates local CLI/server release archive. |

## Boundary

This gate proves:

- single-node production-candidate gates are locally repeatable;
- RPO/RTO and SLO boundaries are documented;
- local performance trend history is checked before candidate claims;
- SDK/API compatibility and deprecation gates pass;
- upgrade and rollback policy gates pass;
- binary release package is buildable and valid.

It does not prove:

- production v1.0 support claim;
- managed service readiness;
- online rolling upgrade support;
- cross-platform binary release matrix;
- production distributed consensus.
