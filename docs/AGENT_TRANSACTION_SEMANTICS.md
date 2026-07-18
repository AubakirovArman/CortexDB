# Agent Transaction Semantics

Status: accepted research design and guarded prototype.

F04 defines deterministic single-node semantics for multi-agent writes. The
prototype is intentionally small: it reuses the durable `WriteBatch` path, adds
an agent-scoped transaction request, and detects stale same-cell writes before a
commit is appended to the WAL.

## Goals

- Make concurrent agent writes deterministic.
- Keep the existing WAL and `WriteBatch` format unchanged.
- Enforce `AgentView` writable scope before any transaction commit.
- Return structured conflict reports instead of panicking or silently
  overwriting a stale same-cell write.

## Non-goals

- No distributed transaction or consensus semantics.
- No long-lived transaction manager.
- No cross-node idempotency or distributed transaction coordinator.

## Prototype Flag

The prototype is disabled by default and enabled through:

- `DatabaseOptions::agent_transactions.enabled = true`;
- `CORTEXDB_AGENT_TRANSACTIONS=true`.

When disabled, `Database::commit_agent_transaction` returns
`FeatureDisabled("agent_transactions")`.

## Request Contract

`AgentTransactionRequest` contains:

- `agent_id`: must match the supplied `AgentView`.
- `scope`: every write payload descriptor, and every tombstoned current cell,
  must match this scope.
- `base_seq`: the read snapshot sequence the agent planned from.
- `batch`: existing `WriteBatch` operations.
- `idempotency_key`: persisted in a reserved, non-retrievable ledger cell.

The write path commits through the existing atomic batch WAL path. If the report
is `Committed`, the returned `committed_seq` is visible to later reads, so
read-your-writes holds for the single-node engine.

## Conflict Semantics

The prototype uses optimistic same-cell isolation:

- If a target cell has a visible version with `created_seq > base_seq`, the
  transaction returns `AgentTransactionOutcome::Conflict` with
  `AgentTransactionConflictKind::StaleCell`.
- If a target cell was tombstoned after `base_seq`, the transaction returns
  `AgentTransactionConflictKind::TombstonedCell`.
- If concurrent transactions touch disjoint cells, both may commit in WAL order.
- Data contradictions between different cells are not transaction conflicts;
  they remain B09 conflict-index facts for retrieval and verification.

This is deterministic because every decision uses the current committed
`CommitSeq`, the request `base_seq`, and the target `CellId` set.

## Retry And Idempotency

Clients retry a conflict by reading a new snapshot, rebuilding the batch, and
submitting with the new `base_seq`. For a committed request, the user mutations
and its idempotency-ledger entry are written inside one WAL batch, so recovery
exposes both or neither. Repeating the same `(agent_id, idempotency_key)` with
the same request digest returns the original `committed_seq` without rewriting;
reusing the key for a different digest is rejected.

## Verification

Local gate:

```bash
make agent-transaction-semantics-check
```

The gate validates this document, the prototype flag, public request/report
types, and the regression tests in `crates/cortex-engine/tests/agent_transactions.rs`.
