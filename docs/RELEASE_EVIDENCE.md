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

## Unified Release Evidence Bundle

The production release train now has a single evidence bundle gate:

```bash
make release-evidence-bundle-check
```

It writes:

```text
target/release-evidence-bundle/manifest.json
target/release-evidence-bundle/report.json
target/release-evidence-bundle/release-evidence.tar.gz
target/release-evidence-bundle/release-evidence.tar.gz.sha256
```

The bundle manifest records SHA-256 checksums for every included artifact. The
current local bundle includes release, SDK, benchmark, security, storage,
operations, dashboard, and explicitly experimental evidence categories.

Latest local bundle result:

```text
status=passed
artifact_count=25
archive_sha256_sidecar=target/release-evidence-bundle/release-evidence.tar.gz.sha256
```

This archive is local release evidence packaging. It still does not prove a
production distributed database, managed cloud readiness, enterprise compliance
certification, legal-grade verification, or unrestricted HNSW without fallback.

## Public Release Artifact Audit

The public artifact audit target is `v0.1.0-core-alpha.5`. It supersedes the
older `v0.1.0-core-alpha` draft and the earlier `v0.1.0-core-alpha.4` public
page for release-readiness evidence.

The required public assets are:

1. `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz`
2. `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz.sha256`
3. `dashboard-v1.tar.gz`
4. `v0.1.0-core-alpha.5-ann-smoke.tar.gz`
5. `v0.1.0-core-alpha.5-ann-demo-domain.tar.gz`

The release notes for this target are maintained in
[`RELEASE_NOTES_v0.1.0-core-alpha.5.md`](RELEASE_NOTES_v0.1.0-core-alpha.5.md).
Do not treat a GitHub release as current Core Alpha evidence unless those assets
are present and the notes preserve the explicit non-production limits.

Publication audit:

```text
release_url=https://github.com/AubakirovArman/CortexDB/releases/tag/v0.1.0-core-alpha.5
tag=v0.1.0-core-alpha.5
commit=3b717992e84f5917316c158fcae401c1fe13e067
draft=false
prerelease=true
assets=5/5 required assets present
```

Archive SHA-256 values:

| Asset | SHA-256 |
| --- | --- |
| `cortexdb-v0.1.0-core-alpha.5-linux-x86_64.tar.gz` | `1b1d3522c0ec35dddcc37f3f4552b231057add9e68b4e327f13314fd48b5cf39` |
| `dashboard-v1.tar.gz` | `6599d3c145d5d977275a240cf98926b3154968cb2a7f7b78ebc2092d964624b1` |
| `v0.1.0-core-alpha.5-ann-smoke.tar.gz` | `6e3221e5aa5d7cfc9e6e8ad118d9bc7abeeee652f76e23cfeca690875bc3aa80` |
| `v0.1.0-core-alpha.5-ann-demo-domain.tar.gz` | `e230864ac25f71b5a113d152eac492e2c6b1e04ee44a15cf1414d533b6399b04` |

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
