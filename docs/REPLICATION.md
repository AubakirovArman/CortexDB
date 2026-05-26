# Replication v0

CortexDB has a deterministic consensus-model layer and two transport surfaces:

- `InMemoryReplicationTransport` for unit tests and local simulation.
- `TcpReplicationTransport` for a minimal line-framed network path.

The TCP transport is intentionally small. It supports:

```text
VOTE <term> <candidate_id> <last_log_index> <last_log_term>
APPEND <term> <leader_id> <prev_log_index> <prev_log_term> <leader_commit> <term>:<index>:<hex_payload>...
```

`handle_replication_frame` applies the frame to an `ElectionState` plus follower
log and returns a deterministic response frame. `handle_authenticated_replication_frame`
adds shared-token authentication. `ReplicationPeerServer::serve_n` is a small
blocking peer loop used by tests and smoke deployments.

AppendEntries now enforces the core Raft log-matching invariant:

- the follower rejects an append if it does not contain `prev_log_index` with
  `prev_log_term`;
- conflicting suffixes are truncated before replacement entries are appended;
- follower commit indexes in the in-memory transport are advanced to
  `min(leader_commit, last_replicated_index)`.

Snapshot transfer v0 uses:

```text
SNAPSHOT <term> <leader_id> <leader_commit> <chunk_index> <last> <hex_payload>
```

The receiver appends chunks into an in-memory snapshot buffer and acknowledges
the number of received bytes.

Snapshot payloads can also be encoded as `SnapshotSegment` values and installed
durably through `Database::install_snapshot_segment`. Install writes a normal
segment bundle (`.acs/.acb/.aci/.acv/.ach`), publishes the manifest, resets the
WAL tail, and rebuilds the MemTable from the installed snapshot.

`plan_replication_recovery` is the first recovery orchestrator. It compares a
follower commit index with a leader commit index and chooses either append-entry
catch-up or snapshot install when the lag crosses a configured threshold.

Durable recovery is still ACLOG-backed through `ReplicationLog`:

```rust
let entries = ReplicationLog::recover_entries(path)?;
let state = ReplicationLog::recover_consensus(path, node, voters, commit_index)?;
```

## Commit Rule

`ConsensusState` follows the Raft current-term commit restriction:

- a majority ACK can directly advance `commit_index` only for an entry that
  exists in the local leader log and belongs to `current_term`;
- old-term entries are committed indirectly only when a later current-term entry
  reaches quorum;
- `record_match_indexes` computes the highest current-term index replicated on
  a majority of voters and ignores non-voter progress.

This keeps the model from treating arbitrary ACK sets or stale-term entries as
committed data.

## Not Yet

- Native TLS. Put the current token-authenticated frame protocol behind a TLS
  terminator for now.
- Automatic distributed repair after a node rejoins.
