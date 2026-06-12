# Distributed Consensus Design

Status: future phase 1 local evidence gates started, not production-ready.

## Goal

Define the production distributed consensus system required before CortexDB can
claim multi-node database readiness.

## Failure Model

The first supported model is crash-stop nodes with lossy or partitioned
networks. Byzantine nodes, malicious storage, untrusted clocks, and arbitrary
data corruption are out of scope for the first consensus implementation.

The system must tolerate one failed node in a three-node cluster without
committed-log divergence. A minority partition must not elect a writable leader
or serve stale committed state as current.

## Consensus State

Every node must persist:

1. current term;
2. voted-for node;
3. replicated log id and local node id;
4. commit index;
5. last applied index;
6. snapshot index and term.

These fields are separate from local WAL state. Local WAL remains the storage
engine durability primitive; the consensus log decides which operations are
globally committed.

## Replicated Log

The replicated log owns ordering for distributed writes. A log entry must carry
operation bytes, term, index, checksum, and enough metadata to apply it exactly
once to the local engine.

Conflict resolution must follow the documented leader log. A follower that
detects a conflicting term/index pair truncates only uncommitted suffixes.

## Snapshot Install

Snapshot install must transfer a manifest, storage bundle checksums, and the
snapshot index/term. A follower can only publish the snapshot after all files
are verified and the local manifest switch is durable.

## Membership Changes

Membership changes require a safe transition protocol. The default design target
is joint consensus unless a simpler protocol is documented with equivalent
safety proof and tests.

## Required Gates

1. `make distributed-consensus-check`
2. `make consensus-partition-soak-check`
3. `make consensus-failover-slo-check`
4. `make consensus-rejoin-check`
5. `make public-claims-check`

## Current Evidence Boundary

The current gates are local engineering evidence, not a production distributed
database claim. They connect the existing replication integration suites to
machine-readable reports under `target/consensus/`:

| Gate | Evidence |
| --- | --- |
| `make distributed-consensus-check` | replicated log recovery, log matching, commit advancement, election, membership, and replay/apply idempotence |
| `make consensus-partition-soak-check` | partition matrix, split-brain prevention, rejoin repair, repair worker, and consensus-hardening suites |
| `make consensus-failover-slo-check` | local failover/SLO markers and partition evidence with `production_ready=false` |
| `make consensus-rejoin-check` | partition evidence plus snapshot, membership-rotation, runtime, and repair lifecycle evidence |

Promotion beyond this boundary requires sustained multi-process runs, real
operator lifecycle testing, and measured p95/p99 failover and rejoin timings.

## Acceptance

1. A three-node cluster survives leader loss, partition, restart, and rejoin.
2. Committed entries never diverge.
3. Snapshot install cannot publish partial data.
4. Failover and repair SLOs are measured in machine-readable reports.

## Non-goals

1. Managed cloud orchestration.
2. Multi-region consensus.
3. Byzantine fault tolerance.
4. Production claim before sustained partition/rejoin evidence exists.
