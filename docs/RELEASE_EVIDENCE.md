# Release Evidence

Last local release evidence run: 2026-06-01.

This document records the latest local evidence for the Core Alpha release
surface. It does not claim production readiness. It records what was run, which
artifacts were produced, and which parts remain beta-stage work.

## Latest Run

Command:

```bash
make release-check
```

Result:

```text
passed
```

Git commit:

```text
f7b48865fa7f277d6f3053ffa38abcd254416940
```

The annotated tag `v0.1.0-core-alpha` already exists in the local and remote
repository, so this run did not create a new tag.

Host/toolchain profile:

```text
os=Linux srv 6.8.0-87-generic x86_64
rustc=1.95.0 (59807616e 2026-04-14)
cargo=1.95.0 (f2d3ce0bd 2026-03-21)
node=v22.21.0
npm=10.9.4
```

Run scope:

```text
local clean worktree; not an independent hosted CI or clean-container rerun
```

## Gates Covered

`make release-check` includes:

1. `alpha-check`
2. binary release package validation
3. production evidence sweep
4. offsite backup staging check
5. crash/fault check
6. chaos restart check
7. replication lifecycle check
8. HTTP smoke test
9. SDK smoke test

## Evidence Artifacts

| Artifact | Status | Notes |
| --- | --- | --- |
| `target/production-evidence/report.json` | passed | Includes OpenAPI, backup drill, single-node performance, tenant recovery, ANN release evidence, real-embedding readiness, and replication partition steps. |
| `target/backup-drill/report.json` | ok | Backup/restore/prune/validate/readback drill. |
| `target/backup-offsite/report.json` | ok | Local backup staged to offsite target and validated. |
| `target/single-node-performance/report.json` | ok | 500-cell strict and balanced lifecycle matrix completed under the configured time budget. |
| `target/tenant-recovery/report.json` | passed | Tenant isolation, invalid tenant rejection, backup/restore, and restored readback. |
| `target/crash-fault/report.json` | ok | Crash matrix, restart matrix, corruption matrix, and repair tests. |
| `target/chaos-restart/report.json` | ok | 24-step seeded restart/repair/readback scenario. |
| `target/replication-partition/report.json` | ok | Partition, split-brain, repair, and consensus-hardening suites. |
| `target/replication-lifecycle/report.json` | ok | 50 replication lifecycle tests across snapshot, repair, membership, runtime, and topology suites. |
| `target/release-artifacts/cortexdb-dev-linux-x86_64.tar.gz` | passed | Binary package generated and validated with checksum `9bd5b82a0026ce98dd9f347286e2c01457a14fc2fdc605b6265ce0f53a7da48e`. |
| `target/dashboard/dashboard-v1.tar.gz` | passed | Dashboard package generated and validated. |

## Important Boundary

`target/ann/real-embedding/readiness.json` was generated during the production
evidence sweep. In this local `make release-check` invocation it reported
blocked readiness because the shell did not provide real embedding source/query
and endpoint environment variables. That is acceptable for Core Alpha evidence,
but not enough for beta promotion. Beta promotion still requires repeated
real-domain embedding runs in a stable environment with documented SLO history.

## Demo Evidence

The release run executed both demo paths:

- `./examples/demo/investment_projects/run.sh`
- `make rag-demo-smoke`

The RAG demo smoke reported:

```text
ok=true
search_results=8
aql_cells=10
context_cells=10
verify_verdict=mixed_evidence
ingested_records=74
```

## Release Boundary

This evidence supports the Core Alpha claim:

- local single-node durable core;
- AQL retrieval;
- ContextPack;
- VERIFY FACT;
- typed HTTP API;
- CLI and SDK smoke;
- guarded ANN evidence;
- backup/restore and crash/fault evidence.

It does not support these production claims:

- production distributed consensus;
- enterprise RBAC/compliance;
- production HNSW without exact fallback;
- managed cloud service;
- legal-grade verification.
