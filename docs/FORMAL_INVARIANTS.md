# Formal Invariants

EPIC-F10 closes the first formal-invariant slice with a bounded executable
model that is machine-checked in CI and does not require a TLA+ or stateright
runtime dependency. The abstraction is stateright-ready: each section names the state,
actions, safety property, and concrete regression tests that correspond to the
model.

The current checker is `make formal-invariants-check`. It runs
`scripts/formal_invariants_check.py`, writes
`target/formal-invariants/report.json`, exhaustively enumerates the small model
state spaces below, and then runs the linked regression gates.

## Selected Invariants

1. WAL recovery must replay only the committed durable prefix after the durable
   checkpoint sequence. Recovery never invents commit sequence numbers, never
   applies a record at or before the manifest checkpoint, and stops at the first
   missing, corrupt, or partial record tail across rotated WAL files.
2. Snapshot pinning and segment GC must preserve every segment visible through
   the current manifest or an active snapshot pin. A retired segment can be
   collected only when no visible manifest and no pin references it.
3. Policy rewrite must produce no uncovered Scan. Every readable logical read
   surface is rewritten so its scan scopes are a subset of the AgentView's
   readable scopes. A request for an unreadable explicit scope is denied instead
   of being planned and filtered later.

## WAL Recovery Model

State:

- `checkpoint_seq`: highest sequence represented by durable manifest segments.
- `wal_records`: ordered record slots for commit sequences after rotation; each
  slot is `commit`, `corrupt`, or `missing`.
- `rotation_boundary`: a file split point used to prove the same prefix rule is
  independent of WAL file rotation.

Actions:

- append a committed record;
- rotate the WAL file;
- encounter a corrupt or missing tail;
- recover by scanning rotated files in order and skipping records already
  represented by `checkpoint_seq`.

Invariant:

- recovered commit sequences equal the contiguous valid prefix strictly after
  `checkpoint_seq`;
- no recovered sequence is greater than the first bad slot;
- no recovered sequence is less than or equal to `checkpoint_seq`.

Concrete tests:

- `cargo test -p cortex-engine --test wal_commit_seq --all-features`
- `cargo test -p cortex-engine --test checkpoint --all-features recovery_`

## Snapshot Pinning And GC Model

State:

- `manifest_segments`: segment ids visible to the current manifest;
- `pinned_segments`: segment ids referenced by active read snapshots;
- `retired_segments`: segment ids made obsolete by checkpoint or compaction.

Actions:

- publish a new manifest;
- open or close a read snapshot pin;
- retire older segments;
- collect any retired segment that is not manifest-visible and not pinned.

Invariant:

- `collected` is disjoint from `manifest_segments`;
- `collected` is disjoint from `pinned_segments`;
- all manifest-visible and pinned segments remain retained.

Concrete test:

- `cargo test -p cortex-engine --test snapshot_pinning --all-features`

## Policy Rewrite Model

State:

- `readable_scopes`: the AgentView-readable scope set;
- `requested_scope`: either an explicit scope or a broad read;
- `read_surface`: each logical read surface that can introduce a scan.

Actions:

- broad reads are rewritten to scan only readable scopes;
- explicit readable-scope reads are rewritten to a covered scan for that scope;
- explicit unreadable-scope reads are denied before physical planning.

Invariant:

- no successful plan contains a scope outside `readable_scopes`;
- no successful plan contains an uncovered Scan;
- unreadable explicit scopes are denied, not converted into an unrestricted
  scan.

Concrete gates:

- `python3 scripts/policy_rewrite_gate_check.py`
- `cargo test -p cortex-engine --lib all_read_surfaces_rewrite_to_policy_complete_plans --all-features`
