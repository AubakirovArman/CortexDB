# ADR: the embedding profile must be receipt-visible when vector retrieval is used (C3-5-embref)

Status: **Accepted** — records the Track A ↔ Track C design decision that closes
the embedding-ref half of **C3-5** (the A2.1 deferred item). Follows the template
established by [ADR-ann-degradation-receipt-visibility](ADR-ann-degradation-receipt-visibility.md).
Scope: single-node CortexDB v1.0.

## Context

The accountability receipt's core property is that **a verifier re-executes the
bound plan and reproduces the same signed determinism hash** (see
[`ACCOUNTABILITY_RECEIPT_V1.md`](spec/ACCOUNTABILITY_RECEIPT_V1.md)). The
determinism input today binds the query, the agent-view digest, the
context-options digest, the bitmap-program digest, the frozen ranking weights, and
(A3.3) the ANN `serving_epoch`.

It does **not** bind the **embedding profile** (model + dimension + metric). For
**keyword** retrieval that is correct — the candidate set does not depend on any
embedding model. But for **hybrid / semantic** retrieval the candidate set is a
direct function of the embedding profile: the same query against the same cells
but embedded by a *different* model (or metric, or dimension) yields a different
vector neighbourhood and therefore a different pack. If the profile is invisible
to the receipt, a verifier re-executing that query under a different embedding
profile would legitimately produce a different candidate set and fail to reproduce
the receipt — the same "serving strategy depends on out-of-band state" hole the
ANN-degradation ADR closed for `serving_epoch`.

A2.1 already records the profile in the manifest `EMBD` section
([`STORAGE_FORMATS.md`](STORAGE_FORMATS.md)); what remained (the A2.1 deferred
item) is promoting it into the *signed* surface.

## Decision

**The DB embedding *profile* identity enters the signed determinism surface as an
additive `embedding_ref`, gated so the default (keyword-only, no profile binding)
path is byte-identical to pre-C3-5-embref.** Concretely:

1. The determinism input gains `embedding_ref: Option<String>` = the **profile**
   identity `emb1:<model>:<dimension>:<metric>` (the first four fields of the
   per-cell `embedding_ref`, **without** the per-cell `content_hash` — it is the
   pack-level profile, not a cell).
2. It is surfaced through the **additive-minor determinism-input `Option`
   mechanism** the `serving_epoch` ADR generalized ("this ADR is the template for
   a serving/ranking decision that depends on state must be in the signed
   surface") — **not** a `context_pack.canonical` / `verification_report.canonical`
   field-set bump. When `None`, `determinism_hash_input_value` emits no
   `embedding_ref` key, so committed accountability goldens verify byte-for-byte.
3. Binding is opt-in via a default-off knob `EmbeddingRefReceiptOptions { enabled }`
   (the same opt-in shape as A3.3 `AnnGuardedSamplingOptions` and A5.2 learned
   ranking). Enabled **and** an embedding profile present ⇒ `Some(profile_ref)`;
   otherwise `None`. So every existing deployment and every committed golden stays
   byte-identical until an operator opts in.
4. The profile ref is derived from deterministic manifest state (model + dimension
   + metric), never from wall-clock or RNG — replays and replica receipts stay
   byte-identical (INV-3).

## Rationale

1. **Minimal signed surface, correct soundness.** The candidate set depends on the
   *profile* (model/dimension/metric), not on any per-query embedding state, so a
   single profile identity is exactly enough for a verifier to reproduce a
   hybrid/semantic pack. The per-cell `embedding_ref` (with content hash) stays in
   the cell payload; only the profile identity is promoted.
2. **Determinism preserved.** The ref is a pure function of the manifest embedding
   profile; no wall-clock, no RNG.
3. **Additive, not breaking, no golden churn.** Reusing the `serving_epoch`
   `Option` mechanism (determinism input, not the canonical field set) means the
   default path emits no key — no schema-version bump, no signed-golden
   regeneration, no cross-language re-derivation change (the C4-2 Python check does
   not cover the determinism input, and `None` adds nothing). Old receipts remain
   verifiable unchanged.
4. **No parallel machinery.** It integrates with the existing
   `AccountabilityDeterminismInput` the receipt already builds.

## Consequences

- A golden-safety test (`embedding_ref_is_additive_and_golden_safe`) asserts
  `None` adds no key (goldens byte-identical) and `Some` both adds the key and
  changes the hash — the same guarantee `serving_epoch_is_additive_and_golden_safe`
  gives for A3.3.
- **Reopen trigger.** Revisit only if the candidate set gains a dependence on
  embedding state finer than the profile (e.g. per-query quantization parameters
  that change neighbourhoods), at which point that state joins the signed surface
  through another additive `Option`.
- This is a **hardening** of an already-manifest-recorded field, landed opt-in and
  golden-safe; it is not a v1.0 blocker and does not alter the default (keyword)
  receipt bytes.
