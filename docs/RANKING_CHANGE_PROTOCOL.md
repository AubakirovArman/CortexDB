# Ranking-Change Protocol (C3-1)

Any change to a **frozen ranking weight** — the numbers that decide retrieval and
context-pack ordering — is a governed change. It cannot land as an ordinary edit,
because a receipt signs the order those weights produce: silently changing a
weight would make previously-signed receipts unreproducible. This document is the
mandatory landing path. The enforcement is already wired as gates; this is the
human procedure that goes with them.

## What counts as a frozen ranking weight

Everything in the single source-of-truth artifact
[`crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json`](../crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json):
the context-pack redundancy penalty, the value-per-token weights, the query-term
weights (base / expansion / phrase / anchor), the lexical field weights (title 8 /
table 6 / path 5 / entity 4 / chunk 2 — owned by `lexical_field_weight`, see
[A7.1]), and the per-mode retrieval-fusion weights. If a code change alters any of
these numbers, this protocol applies. Adding a **default-off** knob that does not
change the default order (e.g. A5 MMR, A4.1 recency window, A4.2 supersession,
A7.2 two-stage rerank) is *not* a frozen-weights change — those keep the goldens
byte-identical by construction and land normally.

## The invariant the gates enforce

The frozen-weights artifact has a **version** (`ranking-frozen-weights-v1`) and an
**artifact hash**, and that hash is folded into the engine's `determinism_hash`
(`frozen_ranking_weights_identity`). So:

> the ranking-weights version and the weight values are cryptographically bound —
> you cannot change a weight without changing the version, and you cannot change
> the version without the determinism hash moving.

The red-test [`weights_version_binding.rs`](../crates/cortex-engine/tests/weights_version_binding.rs)
proves it: it mutates one Q16 weight, asserts the artifact hash **and** the
determinism hash both change, and asserts both restore on revert. An un-versioned
weight mutation therefore fails a gate rather than shipping silently.

## The mandatory landing path

To change a frozen ranking weight:

1. **Motivate with evidence, not intuition.** Re-derive the weight through the
   learned-ranking calibration (`ranking-weights-drift-check`), which regenerates
   the artifact from the calibration fixture and compares it to the checked-in
   one. A change must clear the held-out SLOs — `--min-heldout-mrr-lift-bps` and
   `--min-heldout-win-rate-pct` — on a split with no `question_id`/`document_id`
   overlap. A weight change that does not improve held-out ranking does not land.
2. **Bump the version.** Update `version` in the artifact (`…-v1` → `…-v1.1`); the
   determinism hash moves with it. This is the additive-minor-version step (the
   same discipline as the C3-5 receipt-schema procedure).
3. **Update the frozen artifact** and re-run `ranking-frozen-weights-check`
   (asserts the engine's live weights equal the artifact) and
   `weights-version-binding-check` (asserts the version↔hash binding holds).
4. **Prove explain still tells the truth.** Run
   `ranking-explain-faithfulness-check`: the `explain` output's per-signal
   contributions must reconstruct the final score under the new weights, so a
   receipt's explanation never contradicts the order it signed.
5. **Re-baseline any receipt/pack goldens** the new order changes, and record the
   change in [`NEXT_GEN_PROGRESS.md`](NEXT_GEN_PROGRESS.md) with the held-out lift
   that justified it.

## The gates (CI-enforced)

| Gate | Enforces |
| --- | --- |
| `ranking-frozen-weights-check` | live engine weights == the frozen artifact |
| `weights-version-binding-check` | version ↔ artifact-hash ↔ determinism-hash binding (red-test) |
| `ranking-weights-drift-check` | a re-derived artifact matches, and clears held-out MRR-lift / win-rate SLOs |
| `ranking-explain-faithfulness-check` | explain contributions reconstruct the signed score |

Skipping any of these is skipping the protocol. The point is not bureaucracy: it
is that a governed context engine must be able to say *why* a document ranked
where it did, and reproduce that ranking from a signed receipt forever.

[A7.1]: NEXT_GEN_PROGRESS.md
