# ADR: agent-cell-id.v2 (31-bit agent slot) — NO-GO for v1.0

Status: **Accepted (NO-GO)** — closes defect (f) from the master plan.
Scope: single-node CortexDB v1.0.

## Context

Cell ids are 64-bit. The `agent-cell-id.v1` layout
([`cell_ids.rs`](../crates/cortex-engine/src/cell_ids.rs)) partitions them as:

```
bits 63..60  namespace nibble (session / feedback / memory / generic)
bits 59..32  28-bit agent slot   (AGENT_CELL_ID_SLOT_MASK = 0x0fff_ffff)
bits 31..0   32-bit sequence     (CELL_ID_SEQUENCE_MASK   = 0xffff_ffff)
```

This gives **2^28 = 268,435,456 agent slots**, each with 2^32 ≈ 4.29 billion
sequence values. A "true" 31-bit agent slot (`v2`) would raise the agent ceiling
8× (to 2.1 billion) but requires a **new persisted schema** and a migration of
existing databases.

Defect (f) asked whether v1's 28-bit slot is a latent correctness risk. It is
**not**: `namespaced_agent_cell_id` (cell_ids.rs:20-29) returns `None` — a
**fail-closed** error — when either the agent slot exceeds the 28-bit mask or the
sequence exceeds the 32-bit mask (this is the CP-3 hardening). There is no
aliasing or silent wraparound; overflow is rejected. `cell-id-collision-check`
gates this.

## Decision

**Do not migrate to a 31-bit v2 layout for v1.0.** Keep `agent-cell-id.v1`.

## Rationale

1. **28 bits is far beyond single-node need.** 268M distinct agent slots on one
   node is orders of magnitude past any realistic single-node agent population,
   which is the entire v1.0 scope (replication/multi-node F02/F03 are frozen).
2. **Overflow is already safe.** The ceiling is a fail-closed rejection, not a
   corruption. A deployment cannot silently alias two agents' cells; it gets an
   error long before any integrity risk.
3. **A schema migration is pure risk here.** `v2` means a new persisted format,
   a migration path for existing DBs, and re-baselining the storage-format-freeze
   and collision goldens — cost and risk with **no demonstrated need**. The
   governed-engine principle is to not change a frozen persisted format without a
   forcing function.

## Consequences

- The 28-bit ceiling is documented as a known, bounded, fail-closed limit (not a
  defect). `cell-id-collision-check` continues to enforce no-aliasing.
- **Reopen trigger.** This decision is revisited only if a real deployment
  approaches the ceiling — concretely, if a single node's distinct agent count
  exceeds ~10^7 (a factor of ~25 safety margin below 2^28), or if multi-node
  addressing (F02/F03) unfreezes and needs a wider agent namespace. At that point
  `v2` lands via the storage-format-freeze procedure with an explicit migration.
- No code change accompanies this ADR: the fail-closed behavior and the gate
  already exist; this record makes the GO/NO-GO explicit and auditable.
