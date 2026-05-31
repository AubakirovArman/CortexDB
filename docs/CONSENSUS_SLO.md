# Consensus SLO Gates

This document defines the release gates for the experimental CortexDB
consensus layer. These are Core Alpha engineering gates, not production SLAs.
They exist so future beta promotion can be based on repeatable evidence instead
of optimistic wording.

## Status

Consensus is still experimental. CortexDB has Raft-like primitives, replicated
log recovery, partition tests, snapshot transfer, membership rotation, durable
repair progress, and repeatable local hardening suites. Production promotion
still requires sustained multi-process failover/rejoin runs and operator
lifecycle evidence under real deployment conditions.

## Safety Gates

| Gate | Core Alpha requirement | Beta promotion requirement |
| --- | --- | --- |
| Minority partition safety | A minority partition must not advance `commit_index`. | Repeated multi-process partition runs must show no minority commits. |
| Rejoin repair | Followers with lag within `snapshot_threshold` must catch up by append repair. | Repair completion p95 must be measured and bounded for configured lag sizes. |
| Snapshot handoff | Followers beyond `snapshot_threshold` must request snapshot install instead of unsafe large append batches. | Snapshot install p95 and failure recovery must be measured across restarts. |
| Replay safety | Consensus-log recovery must fail closed on invalid shape and restore term/index boundaries on valid logs. | Replay latency p95 must be measured for realistic log sizes. |
| Membership lifecycle | Committed membership entries must reconcile durable repair progress before repair planning. | Long-running rotation/rejoin scenarios must pass with retired-node cleanup. |
| Topology reload | Operator topology reload must not plan repairs for stale peers after committed membership recovery. | Online reload lifecycle must be tested against running nodes. |

## Current Local Evidence

The local partition gate is:

```bash
make replication-partition-check
```

It runs the failure-injection, partition matrix, repair, repair cycle, repair
worker, and consensus hardening integration suites. The hardening suite adds:

- repeatable split-brain/rejoin repair soak;
- follower-lag repair classification from append repair to snapshot handoff and
  back to idle;
- membership rotation resume followed by another rotation.

The lifecycle gate is:

```bash
make replication-lifecycle-check
```

It checks snapshot sender/install behavior, repair background progress,
membership rotation resume, topology config loading, and runtime progress
reconciliation.

## Target SLOs Before Beta

These target thresholds are intentionally conservative placeholders until
multi-process evidence exists:

- failover detection and leader replacement: p95 <= 5 seconds on a local
  three-node deployment;
- append repair after rejoin: p95 <= 2 repair ticks for lag within
  `snapshot_threshold`;
- snapshot handoff after rejoin: request emitted on the first repair sweep after
  lag classification;
- consensus replay: p95 duration recorded for 1k, 10k, and 100k log entries;
- membership reload: stale retired-node progress removed before the first
  post-restart repair sweep.

If a run misses one of these targets, the capability remains experimental and
the release notes must describe the failure mode.

## Non-Goals For Core Alpha

- no production HA claim;
- no automated cluster-manager claim;
- no WAN consensus claim;
- no Byzantine fault tolerance;
- no promise that the embedded test transport is equivalent to production
  network behavior.
