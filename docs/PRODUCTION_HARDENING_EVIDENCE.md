# Production Hardening Evidence

Last local production hardening run: 2026-05-31.

This document records the Epic 4 evidence gate from
[`PL_EXTRACTED_EPICS.md`](PL_EXTRACTED_EPICS.md). It hardens the local
single-node reliability surface; it does not claim full production readiness.

## Command

```bash
make production-hardening-check
```

The command writes:

```text
target/production-hardening/report.json
target/production-hardening/*.log
```

Result:

```text
passed
```

## Suites

| Suite | Purpose |
| --- | --- |
| `hardening_docs` | Confirms load/fault, migration, audit/rate-limit, and encrypted-backup design docs exist. |
| `load_smoke` | Runs local concurrent write/read/search/context/verify smoke. |
| `single_node_performance` | Runs embedded Strict/Balanced lifecycle with flow latency percentiles. |
| `performance_trends` | Validates release history, p95/p99 thresholds, and actor busy metrics. |
| `crash_fault` | Runs crash/fault and repair evidence. |
| `migration_compatibility` | Runs storage/API/SDK compatibility fixture validation. |
| `audit_hardening` | Runs audit classification, JSONL sink, and redaction tests. |
| `rate_limit_and_quota_boundary` | Runs typed `429 rate_limited` behavior tests. |
| `cli_audit_tooling` | Runs CLI audit review, filters, summary, and redaction checks. |

## Boundary

This gate proves:

- load smoke evidence is locally repeatable;
- single-node performance trend history is checked;
- crash/fault evidence is locally repeatable;
- migration compatibility gate passes;
- audit and rate-limit behavior is tested;
- encrypted backup design exists and is explicitly not implemented yet.

It does not prove:

- production traffic SLO history beyond local release trend artifacts;
- implemented encrypted backups;
- per-user quota enforcement;
- tamper-evident audit chain;
- production distributed reliability.
