# CortexDB Production v1.0 Boundary

Status: local single-node production evidence gate.

This document defines the narrow Production v1.0 claim for CortexDB. The claim
is intentionally limited to a local single-node production boundary:

```text
one host
one database root
local WAL + storage files
typed HTTP/CLI/SDK contracts
documented backup/restore
repeatable local evidence gates
```

It does not claim managed cloud readiness, external security certification, or
production distributed consensus.

## Stable API/SDK

The stable API/SDK surface for this boundary is the versioned Core Alpha public
contract:

- OpenAPI contract: `docs/openapi.yaml`
- HTTP API behavior: `docs/API.md`
- JSON response examples: `docs/API_JSON_SCHEMAS.md`
- compatibility policy: `docs/API_COMPATIBILITY.md`
- SDK package lifecycle: `docs/SDK_RELEASE.md`
- SDK deprecation policy: `docs/SDK_DEPRECATION_POLICY.md`

Required local gates:

```bash
make openapi-contract-check
make sdk-release-contract-check
make sdk-deprecation-check
```

These gates prove that the published client contract is internally consistent
for this checkout. They do not publish packages to external registries.

## Supported Backup/Restore

Supported Backup/Restore means the local operator workflow is documented and
validated by repeatable drills:

```bash
make backup-drill-check
make backup-offsite-check
```

The supported local path is:

```text
cortexdb backup -> staged archive -> cortexdb restore -> cortexdb validate
```

The backup boundary is documented in `docs/BACKUP_RESTORE.md` and the RPO/RTO
boundary is documented in `docs/RPO_RTO.md`.

## Operational Completeness

Operators should use:

```bash
make production-v1-check
make production-candidate-check
make release-check
cortexdb validate ./data
```

`make production-v1-check` composes the local evidence matrix for this boundary:

- production-candidate evidence;
- release evidence;
- OpenAPI contract;
- SDK release and deprecation checks;
- backup/restore drills;
- public-claims guard.

The generated report is:

```text
target/production-v1/report.json
```

## Distributed Production Is Out Of Scope

Distributed replication and consensus are still experimental engineering
surfaces. They are documented for development and fault-model work, but they
are not production rollout evidence for Production v1.0.

Do not market or operate CortexDB v1.0 as a production distributed database
until a separate distributed-consensus evidence gate exists and passes sustained
multi-process failover, rejoin, repair, and operational lifecycle tests.

## Release Gate

Before claiming this boundary locally, run:

```bash
make production-v1-check
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

The gate is evidence for the current checkout and host. It is not a public SLA.
