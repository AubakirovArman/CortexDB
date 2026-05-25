# Replication v0

CortexDB has a deterministic consensus-model layer and two transport surfaces:

- `InMemoryReplicationTransport` for unit tests and local simulation.
- `TcpReplicationTransport` for a minimal line-framed network path.

The TCP transport is intentionally small. It supports:

```text
VOTE <term> <candidate_id> <last_log_index> <last_log_term>
APPEND <term> <leader_id> <leader_commit> <term>:<index>:<hex_payload>...
```

`handle_replication_frame` applies the frame to an `ElectionState` plus follower
log and returns a deterministic response frame. The current tests cover
loopback append-entry delivery and the older in-memory election/ack path.

Durable recovery is still ACLOG-backed through `ReplicationLog`:

```rust
let entries = ReplicationLog::recover_entries(path)?;
let state = ReplicationLog::recover_consensus(path, node, voters, commit_index)?;
```

## Not Yet

- Long-running peer server.
- TLS/authenticated replication links.
- Snapshot transfer.
- Automatic distributed repair after a node rejoins.
