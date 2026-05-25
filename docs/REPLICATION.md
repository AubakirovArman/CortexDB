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
log and returns a deterministic response frame. `handle_authenticated_replication_frame`
adds shared-token authentication. `ReplicationPeerServer::serve_n` is a small
blocking peer loop used by tests and smoke deployments.

Snapshot transfer v0 uses:

```text
SNAPSHOT <term> <leader_id> <leader_commit> <chunk_index> <last> <hex_payload>
```

The receiver appends chunks into an in-memory snapshot buffer and acknowledges
the number of received bytes.

Durable recovery is still ACLOG-backed through `ReplicationLog`:

```rust
let entries = ReplicationLog::recover_entries(path)?;
let state = ReplicationLog::recover_consensus(path, node, voters, commit_index)?;
```

## Not Yet

- Native TLS. Put the current token-authenticated frame protocol behind a TLS
  terminator for now.
- Durable snapshot install into segment files.
- Automatic distributed repair after a node rejoins.
