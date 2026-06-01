# Distributed Consensus Design

Status: future design gate, not implemented.

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
