# ADR: ANN sampled-recall degradation must be receipt-visible (per-collection serving epoch)

Status: **Accepted** — records the Track A ↔ Track C design review that is the
merge precondition for **A3.3** (sampled guarded-ANN recall) and closes the
receipt-visibility half of **C3-5**.
Scope: single-node CortexDB v1.0 (replication F02/F03 frozen).

## Context

A3.3 replaces the unconditional exact recomputation on every ANN query
([`search/ann/search.rs`](../crates/cortex-engine/src/search/ann/search.rs)) with
**deterministic sampling plus a persisted recall SLO window**: a sampled subset
of queries recompute exact recall, the last N sampled `recall_q16` values live in
a ring buffer persisted with the index metadata, and when the windowed minimum
falls below the recall floor the collection **degrades to exact-serving** until a
rebuild clears a fresh window.

That persistence is the problem the review had to resolve. The accountability
receipt's core property is that **a verifier re-executes the bound plan and
reproduces the same signed bytes** (see
[`ACCOUNTABILITY_RECEIPT_V1.md`](spec/ACCOUNTABILITY_RECEIPT_V1.md)). But a
sampled-recall window makes the *serving strategy for a given query depend on the
history of previously-arrived queries* — whether this collection is currently in
ANN mode or degraded exact-serving mode is a function of the arrival stream, not
of the query alone. If that degradation state is invisible to the plan/receipt, a
verifier re-executing the same query against a rebuilt (non-degraded) index would
legitimately take the ANN path, produce a different candidate set, and fail to
reproduce the receipt — **silently breaking the verifier-re-executes-plan
property**. The master plan flags exactly this (NEXT_GEN_MASTER_PLAN.md line
1077): "состояние деградации обязано быть plan/receipt-видимым … иначе ломается
свойство «верификатор пере-исполняет план»".

## Decision

**The per-collection ANN serving state must enter the signed/plan-bound surface
as a monotonic `serving_epoch` (per collection), and A3.3 does not merge without
it.** Concretely:

1. Each collection carries a monotonic **`ann_serving_epoch`** that increments on
   every serving-mode transition — ANN→exact (recall-floor breach) and
   exact→ANN (a rebuild clears a fresh window). It is derived from existing
   deterministic state (rebuild counter + degradation transitions), never from
   wall-clock or RNG.
2. The epoch (and the boolean serving mode it implies: `ann` vs `exact_degraded`)
   is recorded in the **bound plan/trace** and surfaced in the receipt's
   determinism input, so two runs that served under different modes produce
   **different** receipts, and a verifier is told which mode to re-execute under.
3. Surfacing this is an **additive-minor schema change**, landed through the
   [`RECEIPT_SCHEMA_VERSIONING.md`](RECEIPT_SCHEMA_VERSIONING.md) procedure
   (schema-version bump + `schema_field_binding_v1.json` entry + golden
   re-baseline + the C4-2 cross-language run) — it is **not** an in-place edit of
   `context_pack.canonical.v1` / `verification_report.canonical.v1`.
4. The sampled `recall_q16` window value already rides in the (telemetry,
   non-hashed) `AnnSearchReport`; only the **epoch + mode** are promoted into the
   signed surface. Raw timing/sample counters stay telemetry.

## Rationale

1. **Minimal signed surface, maximal soundness.** Signing a monotonic epoch +
   mode boolean (not the whole ring buffer) is enough to make the serving
   decision reproducible: the verifier does not need the arrival history, only
   *which* epoch/mode the plan was bound under. The full window stays telemetry.
2. **Determinism preserved.** The epoch is a pure function of deterministic index
   state (rebuild count + recorded transitions); no wall-clock, no RNG — so
   replays and same-input replica receipts stay byte-identical (consistent with
   the INV-3 determinism the accountability track enforces).
3. **Additive, not breaking.** Old receipts (no ANN degradation surface) remain
   verifiable under `…canonical.v1`; the epoch enters via a minor bump, keeping
   the frozen-field compatibility rule intact.
4. **No parallel machinery.** The epoch integrates with the existing
   `ann_drift` / `AnnOutcomes` state A3.3 already extends, not a second system.

## Consequences

- **A3.3 acceptance carries a reference to this ADR** (per C3-5): its
  `ann-guarded-sampling-check` gate must assert the serving epoch/mode is present
  in the bound plan and that a forced recall-floor breach changes the epoch (and
  therefore the receipt), and the C4-2 cross-language run must move with the minor
  bump. A3.3 remains a large ANN + benchmark task (50k-vector fixture, exact-scan
  ≤15%, p50 ≥3×, corrupted-graph degradation) — this ADR unblocks its
  *precondition*, it does not implement it.
- **A7.3 (diversity) and A4.2 (coverage)** likewise route their plan-visible
  options through the same additive-minor procedure; this ADR is the template for
  "a serving/ranking decision that depends on state must be in the signed
  surface".
- **Reopen trigger.** Revisit only if the serving decision gains an input that a
  single monotonic epoch cannot capture (e.g. a continuous degradation *level*
  that changes candidate sets by degree rather than a mode flip), at which point
  the signed surface widens through another minor bump.
- No code lands with this ADR: it is the recorded design-review decision that
  A3.3 references. The versioning procedure and binding gate it depends on already
  exist.
