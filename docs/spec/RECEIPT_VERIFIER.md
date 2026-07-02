# Accountability Receipt Verifier

Status: normative verifier algorithm and threat model, single-node beta scope.

This document specifies the offline verifier contract for
`accountability_receipt.v1`. It describes what a third-party verifier checks
from public inputs and which forgery classes are in scope. It does not change
the receipt JSON format; the frozen schema remains
`docs/schemas/accountability_receipt.v1.json`.

Normative implementation sources:

- `crates/cortex-receipt-verify/src/verifier.rs`
- `crates/cortex-receipt-verify/src/model.rs`
- `crates/cortex-receipt-verify/src/receipt_hash.rs`
- `fixtures/accountability_receipt/verify_input.golden.json`
- `scripts/accountability_receipt_verify_check.py`
- `scripts/accountability_receipt_tamper_check.py`
- `docs/spec/ACCOUNTABILITY_RECEIPT_V1.md`
- `docs/spec/GCE_CONTRACT.md`

## Public Inputs

The verifier consumes `cortexdb.accountability_receipt_verify_input.v1`:

- `pack`: the public canonical pack JSON committed by `pack_root`.
- `determinism_input`: query, AgentView/options/bitmap digests, and frozen
  ranking weights version/hash committed by `determinism_hash`.
- `receipt`: the `accountability_receipt.v1` object.
- `public_key`: `{key_id, public_key_hex}`.
- `admitted_cells`: public `{cell_id, cell_content_hash, raw_content_hex?}`
  evidence used for cell-set and span checks.

The standalone binary is `cortex-receipt-verify`. Its dependency graph must not
include `cortex-engine`, `cortex-storage`, `cortex-aql`, or `cortex-server`.

## Seven Verifier Steps

Step 1 - Validate public input and header shape.

Run `verify_header_shape`. The verifier rejects any mismatch in
`schema_version`, `hash_alg`, `sig_alg`, `db_instance_id`, `key_id`, receipt
signature key id, or public key identity. The required algorithms are
`blake3-256` and `ed25519`. `audit_chain_head` must be a 32-byte lowercase or
uppercase hex hash.

Step 2 - Verify Ed25519 signature over the canonical header.

Run `verify_signature`. The verifier computes `canonical_header_bytes` and
verifies `signature.signature_hex` with `public_key.public_key_hex`. A valid
signature binds the header roots, `pack_root`, `determinism_hash`, and
`audit_chain_head`.

Step 3 - Recompute all root commitments.

Run `verify_roots`. The verifier recomputes:

- `access_root`
- `provenance_root`
- `cell_set_root`
- `verification_root`
- `budget_commitment`
- `conflict_commitment`
- `pack_root`
- `determinism_hash`

Any mismatch is a `RootMismatch`.

Step 4 - Enforce admitted-cell access leaves.

Run `verify_access`. Any `leaf_type=admitted_cell` access leaf must have
`decision=allowed`. Current verifier fixtures prove this leaf-level check.
Future plan-bound verification must also evaluate the signed or committed
bitmap program evidence; trusting an `allowed` leaf alone is not sufficient for
the final GCE moat.

Step 5 - Check cell-set identity and provenance spans.

Run `verify_cell_set` and `verify_provenance`. Every committed `cell_id` must be
present in `admitted_cells`, and `cell_content_hash` must match. If
`raw_content_hex` is supplied for a source cell, `source_byte_start` and
`source_byte_end` must be inside the raw byte length. The defending fields are
`cell_set_root`, `provenance_root`, `cell_content_hash`, `source_cell_id`,
`source_byte_start`, and `source_byte_end`.

Step 6 - Check token budget consistency.

Run `verify_budget`. The summary leaf and per-cell leaves must agree:
`cell_sum == estimated_tokens` and neither `cell_sum` nor `estimated_tokens`
may exceed `token_budget_tokens`. The defending field is `budget_commitment`.

Step 7 - Check verification references and deterministic input binding.

Run `verify_verification_references` after root verification. Verification
leaves may reference only admitted cells. The signed `verification_root`,
`conflict_commitment`, `pack_root`, and `determinism_hash` bind verdict,
conflict, pack JSON, query/options/view/bitmap digests, and frozen ranker
version/hash. `audit_chain_head` binds the signed receipt to the audit-chain
tail observed before receipt emission; tampering with that field invalidates
the signature.

## Eight Forgery Classes

Each in-scope forgery class maps to a defending field and a gate.

| Forgery class | Defending field | Gate evidence |
|---|---|---|
| `access_allowed_to_denied`: mutate admitted access from `allowed` to another decision | `access_root`, `signature` | `accountability-receipt-tamper-check` |
| `source_byte_start_shift`: fabricate or shift citation/span bounds | `provenance_root`, `cell_content_hash`, `signature` | `accountability-receipt-tamper-check` |
| `drop_visible_conflict`: remove a visible conflict signal | `conflict_commitment`, `signature` | `accountability-receipt-tamper-check` |
| `swap_verdict`: change verification status or evidence meaning | `verification_root`, `signature` | `accountability-receipt-tamper-check` |
| `budget_estimated_tokens`: overspend or rewrite token accounting | `budget_commitment`, `signature` | `accountability-receipt-tamper-check` |
| `replay_different_query`: reuse a receipt under a different query or determinism input | `determinism_hash`, `signature` | `accountability-receipt-tamper-check` |
| `flip_signature_byte`: mutate signed bytes or signature material post hoc | `signature`, `public_key` | `accountability-receipt-verify-check` and `accountability-receipt-tamper-check` |
| `audit_chain_head_rewrite`: rewrite the committed audit-chain tail | `audit_chain_head`, `signature` | `receipt-replica-invariance-check` |

Additional verifier checks cover raw admitted cell substitution through
`cell_set_root` and `cell_content_hash`. This is part of Step 5 even when it is
not a separate AR-8 mutation name.

## Gate Requirements

`receipt-threat-model-check` validates this document against the implementation
markers and existing gate names. It does not execute the full verifier matrix;
that remains assigned to:

- `accountability-receipt-verify-check`
- `accountability-receipt-tamper-check`
- `accountability-receipt-determinism-check`
- `transparency-anchor-check`
- `receipt-replica-invariance-check`

The threat model check must fail if a named forgery class is removed from this
document, if a defending field is missing, if the seven verifier steps are not
enumerated, or if make/phony wiring for the check disappears.

## Out Of Scope

The verifier proves internal consistency of a single receipt. It does not prove
the factual truth of cell contents.

The verifier does not prevent issuer equivocation by itself: a database can
sign two different individually valid receipts for the same logical request.
CortexDB's local `cortexdb.transparency.log.record.v1` anchor records
`pack_root` and `determinism_hash` when
`CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE` is configured, and the
`transparency-anchor-check` rejects same-input records that map to different
roots. The `transparency-witness-check` gate adds
`cortexdb.transparency.witness.record.v1`, an external mirror witness record
that signs the verified local log head and sequence range with an independent
Ed25519 key. This gives off-database mirror evidence for the `pack_root` chain
head, but does not claim public service availability, CT-style gossip,
KMS/HSM custody, or compliance immutability.

The `transparency-witness-quorum-check` gate adds
`cortexdb.transparency.witness.quorum.v1`, an independent witness quorum
evidence record. Verifiers can require multiple valid witness records to agree
on the same log head and sequence range, while rejecting duplicate witness
ids, key ids, or public keys and rejecting split log heads. This still does
not claim public transparency service availability, CT-style gossip, KMS/HSM
custody, or compliance immutability.

The `transparency-inclusion-check` gate adds
`cortexdb.transparency.inclusion.proof.v1`, a Merkle inclusion proof from a
sequence-bound transparency record hash to the published transparency root for
that log snapshot. Verifiers can reject record-hash tampering and sibling-path
tampering. This still does not claim public transparency service availability,
CT-style gossip/consistency exchange, KMS/HSM custody, or compliance
immutability.

The `transparency-consistency-check` gate adds
`cortexdb.transparency.consistency.v1`, append-only consistency evidence across
published transparency snapshots. Verifiers can reject divergent snapshot
prefixes and truncated newer snapshots by comparing record-hash prefixes and
Merkle roots. This still does not claim public transparency service
availability, network gossip fanout, independent monitor uptime, KMS/HSM
custody, or compliance immutability.

The `transparency-availability-check` gate adds
`cortexdb.transparency.availability.evidence.v1`, public monitor availability
evidence for a published transparency head. Verifiers can require fresh HTTPS
monitor observations from independent monitor ids and URLs, available 2xx
service status, sufficient monitor uptime, and agreement on the published log
count, log head, and Merkle root. This still does not claim network gossip
fanout, KMS/HSM custody, or compliance immutability.

The `transparency-gossip-check` gate adds
`cortexdb.transparency.gossip.evidence.v1`, monitor-to-monitor gossip fanout
evidence. Verifiers can require fresh delivered HTTPS exchanges, a declared
minimum per-monitor fanout, and agreement on the published log count, log head,
and Merkle root. Stale exchanges, insufficient fanout, and split log heads are
rejected. This still does not claim continuous production SLO compliance,
KMS/HSM custody, or compliance immutability.

The `transparency-slo-check` gate adds
`cortexdb.transparency.slo.evidence.v1`, continuous public transparency
operations/SLO evidence. Verifiers can require ordered operations windows that
cover the declared period without gaps, meet the declared availability
percentage, carry monitor quorum and gossip fanout summaries, report
append-only consistency status, and keep log counts monotonic. Gaps,
below-target availability, log-count regression, and same-count split heads
are rejected. This still does not claim live production deployment, KMS/HSM
custody, or compliance immutability.

The verifier does not claim production-grade KMS/HSM custody or compliance
immutability. Those claims require separate green gates.
