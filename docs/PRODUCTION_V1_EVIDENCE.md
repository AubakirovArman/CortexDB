# Production v1.0 Evidence

Last local Production v1.0 run: 2026-05-31.

Run:

```bash
make production-v1-check
```

Primary artifact:

```text
target/production-v1/report.json
target/production-v1/*.log
```

Latest local status: passed.

## Matrix

| Suite | Purpose |
| --- | --- |
| production candidate | Confirms RPO/RTO, SLO, API/SDK compatibility, migration, and binary release evidence. |
| release check | Confirms the broader local release matrix and release artifacts. |
| OpenAPI contract | Confirms the typed HTTP API contract is current. |
| SDK release contract | Confirms SDK metadata and version lock-step. |
| SDK deprecation policy | Confirms lifecycle/deprecation rules remain documented and enforced. |
| SDK e2e release | Confirms live SDK compatibility and SDK examples release artifact packaging. |
| backup/restore support | Confirms the local backup drill. |
| backup offsite support | Confirms local offsite staging. |
| public claims | Confirms public wording does not overclaim the product boundary. |

## Boundary

The local Production v1.0 gate proves:

- single-node production-v1 evidence gates pass locally;
- stable API/SDK compatibility gates pass;
- supported backup/restore gates pass;
- operational docs cover the single-node boundary;
- distributed production is explicitly out of scope.

The gate does not prove:

- managed cloud service readiness;
- production distributed consensus;
- release artifacts for every platform;
- external security certification.
