# Consensus Design Document

This document defines the consensus model, architecture, safety invariants, and non-goals of the CortexDB distributed layer.

## Consensus Model: Raft-like Active Replication

CortexDB implements a strong-leader, active-replication consensus model derived from Raft. It ensures that committed entries survive node failures, network partitions, and cluster restarts.

### Roles and States
1. **Leader**: Coordinates all database mutations, appends entries to its local log, and replicates them to Follower nodes.
2. **Follower**: Passive recipient of replicated log entries, heartbeat frames, and snapshots.
3. **Candidate**: State assumed by a node when initiating an election.

### Core Safety Invariants
- **Election Safety**: At most one leader can be elected in a given term.
- **Leader Append-Only**: A leader never overwrites or truncates its own log; it only appends new entries.
- **Log Matching**: If two logs contain an entry with the same index and term, then they are identical in all entries up through the given index.
- **Leader Completeness**: If a log entry is committed in a given term, that entry will be present in the logs of the leaders for all higher-numbered terms.
- **State Machine Safety**: If a server has applied a log entry at a given index to its state machine, no other server will ever apply a different log entry for the same index.

---

## Architectural Separation

To guarantee disk consistency and prevent metadata corruption, the storage layer separates data concerns:

1. **Local WAL (`db.wal`)**: Captures immediate, uncheckpointed local write mutations (`PutCell`, `Tombstone`).
2. **Replication Log (`replication.aclog`)**: A persistent, append-only log storing replicated transaction payloads along with consensus metadata.
3. **Consensus Metadata**: Persisted node identity, current term, and voter configuration.

---

## Non-Goals

The following architectural boundaries are explicitly out-of-scope for the current consensus layer:
- **Multi-Raft / Sharding**: CortexDB is designed for single-node vertical scale and replicated master-slave high availability. Multi-shard consensus is a future track.
- **Full Joint Consensus Membership**: CortexDB has a persisted membership-entry primitive for committed voter rotations, but production-grade Raft joint consensus, automatic rotation, and removed-node lifecycle handling are future work.
- **Byzantine Fault Tolerance**: We assume a non-byzantine environment (fail-stop nodes, non-malicious network errors).
- **Auto-healing Network Relays**: Nodes must be directly reachable. NAT traversal and WAN routing/relaying must be handled by external network overlays (e.g., WireGuard).

---

## Operational Scenarios

### 1. Leader Election
When a follower's election timer expires, it transitions to **Candidate**, increments the term, votes for itself, and broadcasts `VOTE` frames. It must receive votes from a majority of cluster voters to become Leader.

### 2. Log Replication & Commit Rule
- Mutations are sent as `APPEND` frames.
- Entry commits require majority acknowledgments.
- Leaders can only directly commit entries of their **current term**. Stale-term entries are committed indirectly when a current-term entry gains majority.

### 3. Snapshot Transfer
When a follower is too far behind the leader's commit index (crossing the configured lag threshold), the leader transitions to snapshot transfer. It streams chunks via `SNAPSHOT` frames. The follower installs the segment bundle durably, resets its WAL tail, and restarts its state machine.
