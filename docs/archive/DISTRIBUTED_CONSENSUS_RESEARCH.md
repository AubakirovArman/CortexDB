# Distributed Consensus Research Track

Status: research evidence track, not production distributed consensus.

This track records local consensus evidence without changing the public claim
boundary. CortexDB still claims a local single-node database core. Multi-node
high availability remains a future product layer until sustained operational
evidence exists.

## Current Evidence Gates

Run:

```bash
make distributed-consensus-research-check
```

The aggregate gate depends on:

```text
make distributed-consensus-check
make consensus-partition-soak-check
make consensus-failover-slo-check
make consensus-rejoin-check
```

Generated reports:

```text
target/consensus/distributed-consensus.json
target/consensus/partition-soak.json
target/consensus/failover-slo.json
target/consensus/rejoin.json
target/consensus/research-summary.json
```

Every report must keep:

```text
production_ready=false
```

## What Is Covered Locally

- Replication log persistence and recovery.
- Log matching and conflicting suffix truncation.
- Commit advancement rules.
- Leader election term replacement.
- Joint-consensus membership primitives.
- Replay/apply idempotence.
- Partition matrix and split-brain prevention simulations.
- Rejoin repair, snapshot sender, snapshot fault, and lifecycle evidence.

## What Is Not Covered Yet

- Long-running multi-process cluster soak.
- Measured p95/p99 failover and rejoin SLOs from real processes.
- Operator lifecycle for node add/remove/fail/recover in production.
- Multi-host network partitions.
- Production support, SLAs, or managed orchestration.

## Promotion Requirement

Before CortexDB can claim production distributed consensus, this research track
must be replaced by sustained operational evidence:

1. A real multi-process three-node cluster run.
2. Repeated split-brain/rejoin drills.
3. Failover and rejoin latency reports.
4. Snapshot install and repair under process restarts.
5. A release gate that can set `production_ready=true` without weakening public
   claims policy.
