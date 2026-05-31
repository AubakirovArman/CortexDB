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

The in-memory transport can also model network partitions by restricting
source-target links to explicit components. `replicate_to_best_effort` then
counts only reachable successful followers, which lets the failure-injection
suite cover minority partitions, healed quorums, and majority-side elections
without treating a dropped link as a storage error.

AppendEntries now enforces the core Raft log-matching invariant:

- the follower rejects an append if it does not contain `prev_log_index` with
  `prev_log_term`;
- entry indexes in a request must be contiguous from `prev_log_index + 1`; gaps
  and out-of-order batches are rejected before mutating the follower log;
- conflicting suffixes are truncated before replacement entries are appended;
- follower commit indexes in the in-memory transport are advanced to
  `min(leader_commit, last_replicated_index)`.

Follower election state also rejects a conflicting same-term leader after a
leader has already been accepted for that term. A higher-term leader still
forces the follower to step down and replace the previous leader metadata.

Snapshot transfer v0 uses:

```text
SNAPSHOT <term> <leader_id> <leader_commit> <chunk_index> <last> <hex_payload>
```

The receiver appends chunks into an in-memory snapshot buffer and acknowledges
the number of received bytes. A non-zero chunk cannot be accepted as the first
snapshot chunk, so a peer cannot silently start from the middle of a snapshot
stream.

`assemble_snapshot_chunks` validates offline chunk reassembly before install:
chunks must start at index `0`, be contiguous, come from one leader/term/commit
tuple, and end with a final `last=true` chunk. This prevents peer resync tests
from accidentally accepting missing, reordered, or mixed-leader snapshots.

Snapshot payloads can also be encoded as `SnapshotSegment` values and installed
durably through `Database::install_snapshot_segment`. Install writes a normal
segment bundle (`.acs/.acb/.aci/.acv/.ach`), publishes the manifest, resets the
WAL tail, and rebuilds the MemTable from the installed snapshot.

`ReplicationPeerServer::serve_n_with_snapshot_install` connects those two
surfaces: it accepts TCP `SNAPSHOT` chunks, assembles the stream on the final
chunk, decodes the `SnapshotSegment`, and installs it into the follower
database before acknowledging success to the sender.

`plan_replication_recovery` is the first recovery orchestrator. It compares a
follower commit index with a leader commit index and chooses either append-entry
catch-up or snapshot install when the lag crosses a configured threshold.
`repair_lagging_voter` applies that plan for a voter that rejoined after a
partition: small lag is repaired by sending the missing contiguous
`AppendEntries`, while large lag returns an `InstallSnapshot` plan instead of
trying an unsafe oversized append batch. `repair_lagging_voters` runs the same
logic across the current voter set and reports how many followers were already
caught up, repaired by append, or deferred to snapshot install.
`plan_replication_repair_sweep` is the scheduling half of that flow: it reads
durable follower progress, rejects inconsistent progress, ignores non-voter
inputs, and classifies every current voter as already caught up, append-repair,
or snapshot-required before any network mutation is attempted.
`run_replication_repair_cycle` connects the scheduler to the transport for one
leader tick: caught-up voters are reported, append-repair decisions send
`AppendEntries`, and snapshot-required decisions are returned as explicit
snapshot requests for the snapshot sender.
`send_replication_snapshot_request` sends those requests over a snapshot
transport by encoding a `SnapshotSegment`, chunking it deterministically, and
requiring cumulative byte acknowledgements from the follower before the sender
reports success.
`run_replication_repair_worker` ties the pieces into a repeatable repair loop:
each tick reads durable follower progress, executes append repair, sends
available snapshot repairs, and stops when the cluster is idle or when a
snapshot is still pending from the caller-provided snapshot source. This keeps
the background-task boundary explicit while making the node-rejoin repair path
testable without a long-running thread.
`spawn_replication_repair_background_task` owns the leader snapshot, transport,
progress source, and snapshot source in a bounded OS-thread loop. It supports a
deterministic stop handle plus finite-run policies for tests and smoke
deployments, so node-rejoin repair can run as a runtime task instead of being
manually stitched together by callers.
`ReplicationDatabaseSnapshotSource` provides the database-backed snapshot source
for that runtime: it reads a current `Database::replication_snapshot_segment`
through `Arc<RwLock<Database>>` and only returns it when the requested repair
checkpoint matches the database commit sequence.
`ReplicationFollowerProgressStore` persists follower commit/observed indexes in
an atomically written local progress file, and `ReplicationStoredProgressSource`
adapts that file-backed state to the background repair worker. This closes the
restart gap for repair planning input: after a repair runtime restarts, it can
resume from the last durable follower progress snapshot instead of requiring a
caller to rebuild all progress from memory.
`ReplicationProgressRecordingTransport` wraps any replication/snapshot transport
and records successful `AppendEntries` ACKs plus final validated snapshot ACKs
into that same store. This gives the repair loop a direct path from live peer
ACKs to durable repair-planning progress.
`spawn_replication_repair_background_task_with_progress_store` is the standard
runtime helper for that pairing: it builds a `ReplicationStoredProgressSource`
and wraps the supplied transport in `ReplicationProgressRecordingTransport`, so
one durable progress store is used for both repair planning input and successful
peer ACK output.
`ReplicationFollowerProgressStore::reconcile_voters`,
`reconcile_voting_config`, and `reconcile_consensus_state` connect that durable
repair state to membership lifecycle changes. Reconciliation keeps progress for
current remote voters, removes retired voters, drops local-node progress, and
seeds newly joined voters at `0/0` so the next repair sweep explicitly catches
them up through append repair or snapshot install.
`ClusterConfig::replication_paths` provides the operator-facing path placement
for that state. It validates the configured local node, rejects duplicate or
invalid node identities, and places cluster config, consensus log, repair
progress, and snapshot inbox files under `replication/node-<id>.*` paths so
multiple node identities do not share one repair-progress file by accident.
`ClusterConfig::store` and `ClusterConfig::load` persist the local operator
topology as `CORTEXDB_CLUSTER_CONFIG_V1` using atomic file replacement, giving
restart code a durable source of truth for local node identity and peer
addresses.
`open_replication_node_runtime` wires that durable topology into startup: it
loads the node-scoped cluster config, recovers committed membership from the
consensus log, rejects impossible commit indexes, reconciles durable follower
progress with the recovered voter set, and then opens the node-scoped
consensus-log writer. This keeps local startup from accidentally planning
repairs against stale peers after membership rotation or operator reload.

Durable recovery is still ACLOG-backed through `ReplicationLog`:

```rust
let entries = ReplicationLog::recover_entries(path)?;
let state = ReplicationLog::recover_consensus(path, node, voters, commit_index)?;
```

`recover_consensus` and `recover_consensus_with_membership` validate the
consensus-log shape before publishing a recovered state: recovered entries must
have non-zero terms, contiguous indexes starting at `1`, and a `commit_index`
that is not ahead of the recovered log. Raw `recover_entries` remains available
for diagnostics and WAL inspection, but runtime consensus recovery is
fail-closed.

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

## Membership Reconfiguration v0

`ConsensusState::reconfigure_voters` and `ElectionState::reconfigure_voters`
provide the first explicit membership lifecycle surface. The voter set must be
non-empty and must keep the local node in the configuration. Consensus commit
counting and election vote tracking then use the updated voter set, so joined
nodes can count toward quorum and removed nodes stop contributing progress.

`MembershipConfig` can also be encoded as a replicated log entry. Recovery uses
only committed membership entries (`entry.index <= commit_index`) to rebuild the
effective voter set from `replication.aclog`:

```rust
let state = ReplicationLog::recover_consensus_with_membership(
    path,
    node,
    bootstrap_voters,
    commit_index,
)?;
```

This keeps membership rotation in the consensus log rather than in the local
database WAL.

Joint membership entries can represent the transitional `old + new`
configuration used during safe reconfiguration. The consensus model only marks
a joint entry committed after both the old voter set and the new voter set have
their own majorities. This prevents a change from being accepted by only one
side of a split reconfiguration.

`rotate_membership_with_joint_consensus` is the first automated rotation
orchestrator. It catches up old/new voters, appends a durable joint membership
entry, requires both old and new majorities, appends the final stable membership
entry, and only then publishes the new voter set locally. If joint consensus is
not reached, the stable config is not appended.

`resume_joint_membership_rotation` handles the crash/restart boundary between
those two phases. After recovery, it requires a committed joint config in the
replication log before it can append the final stable config. An uncommitted
joint entry is not enough to publish the new voter set.

This is still not a full Raft membership implementation. CortexDB now has the
durable entry format, majority-safety primitive, and a two-phase orchestrator;
removed-node lifecycle handling and distributed rejoin/repair remain
post-Core Alpha work.

## Failure-Injection Coverage

The first post-Core Alpha consensus failure harness is an integration test suite
under `crates/cortex-engine/tests/replication_failure_injection.rs`. It covers:

- a minority partition that cannot advance `commit_index` until a majority
  heals;
- a higher-term majority that rejects a stale partitioned leader before the
  follower log is mutated;
- idempotent replication-log replay after restart, including preservation of
  the next log index;
- chunked snapshot resync that installs durably on a lagging follower and
  rejects missing or mixed chunks;
- peer snapshot transport that installs a multi-chunk snapshot durably into a
  follower database and survives reopen;
- peer snapshot fault handling: partial transfers, stale second chunks, and
  corrupt final chunks leave the follower's previous durable state intact;
- membership join/leave behavior that changes quorum counting without allowing
  empty configs or local-node removal.
- a five-node partition matrix that blocks minority writes, commits after heal,
  elects a majority-side leader, and rejects stale minority-leader appends;
- restart inside a partition: uncommitted leader entries remain uncommitted
  after recovery until heal, while committed leader entries resume with the
  recovered commit index and catch up healed followers;
- TCP snapshot transport smoke coverage for multi-chunk segment payloads.
- committed membership rotation surviving replication-log restart.
- joint-consensus membership entries requiring majorities from both old and new
  voter sets before commit.
- automated membership rotation that commits joint and stable entries durably,
  recovers the final voter set after restart, and refuses to publish a stable
  config when the joint quorum is unavailable.
- rotation resume after restart: committed joint configs can finish the stable
  phase, while uncommitted joint configs are rejected.
- node rejoin repair planning: a lagging voter that rejoins with small lag is
  caught up by missing `AppendEntries`, while lag beyond the configured
  threshold selects snapshot install without mutating the follower log.
- voter-set repair sweep: caught-up, append-repair, and snapshot-required
  followers are handled in one deterministic pass while non-voter progress
  inputs are ignored.
- progress-aware repair scheduling: durable follower progress is validated
  before the repair loop decides whether to no-op, append entries, or defer to
  snapshot install.
- one-shot repair cycle execution: append-repair decisions are sent through the
  transport, while snapshot-required followers are handed off as explicit
  requests instead of being silently ignored.
- snapshot repair sending: explicit snapshot requests can now be streamed as
  checked `SnapshotChunk` frames to a follower that durably installs the
  decoded snapshot segment.

This is not a full distributed consensus certification yet. The remaining
production work is broader membership rotation crash/restart coverage and
rejoin/repair handling after network partitions.

## Not Yet

- Native TLS. Put the current token-authenticated frame protocol behind a TLS
  terminator for now.
- A full cluster config manager that coordinates topology changes with live
  replication tasks. The operator topology file is durable, but automatic
  online reload/rotation remains future work.
