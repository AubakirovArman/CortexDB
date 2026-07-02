# Governed Context Engine Contract

Status: normative category contract, single-node beta scope.

This document defines the minimum public contract for a Governed Context Engine
(GCE). It is not a marketing page and it is not a production compliance claim.
A conforming implementation produces governed context as a typed result,
keeps the governance path deterministic and LLM-free, exposes provenance and
verification evidence as first-class outputs, preserves conflict and
completeness signals, and keeps optional proof material additive.

Normative sources in this repository:

- `crates/cortex-engine/src/context/mod.rs`
- `crates/cortex-aql/src/binder.rs`
- `crates/cortex-engine/src/verification/types.rs`
- `crates/cortex-engine/src/canonical.rs`
- `crates/cortex-engine/src/memory/lifecycle.rs`
- `crates/cortex-engine/src/feedback/index.rs`
- `docs/schemas/context_pack.v1.json`
- `docs/spec/ACCOUNTABILITY_RECEIPT_V1.md`
- `docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md`

## Scope And Compatibility

The stable result type is `context_pack.v1`. The frozen JSON Schema is
`docs/schemas/context_pack.v1.json`.

`context_pack.v1` is additive-only until a future `context_pack.v2`: required
fields, enum meanings, and `schema_version` cannot be removed or renamed.
New fields such as `grounding_report` and `accountability_receipt` are optional
top-level fields. Consumers that do not understand them must still be able to
parse the original v1 shape when those fields are absent.

The accountability receipt contract is described separately in
`docs/spec/ACCOUNTABILITY_RECEIPT_V1.md`, and the offline verifier algorithm
plus threat model are specified in `docs/spec/RECEIPT_VERIFIER.md`. A configured
local signed receipt is evidence of internal consistency for the response. The
current standalone verification binary is `cortex-receipt-verify`; the receipt
schema fixes `blake3-256` roots and `ed25519` signatures. A local receipt is
not, by itself, a claim of external transparency, KMS/HSM custody, immutable
compliance storage, or factual truth of the underlying cells.

## Cell ID Layout Boundary

The current agent-scoped persisted cell-id layout is
`agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence`: bits 63..60 carry
the namespace, the next 28 bits carry the agent slot, and the low 32 bits carry
the per-agent sequence. The current high-nibble namespaces are memory `0x8`,
feedback `0x9`, and session `0xA`.

`cell-id-collision-check` treats this as the documented v1 domain. It requires
memory, session, and feedback constructors to reject over-width agent slots
instead of silently truncating them. This closes the original collision bug
without changing the persisted v1 encoding.

A true 31-bit session/feedback agent slot does not fit this v1 layout while
retaining the high-nibble namespace and 32-bit sequence. Any future widening of
the session or feedback agent slot MUST ship as a new persisted schema version
with an explicit migration or refuse-to-read guard, and it must pass the
repository migration compatibility gate before a receipt or determinism claim
can rely on it.

## ContextPack Result Type

A GCE answer surface is a `ContextPack`, not a raw vector hit list. The current
CortexDB struct fields are defined in `crates/cortex-engine/src/context/mod.rs`.
A conforming `ContextPack` result includes these top-level semantics:

- `schema_version`: the public schema marker, currently `context_pack.v1`.
- `token_budget_tokens`: the requested or policy-clamped token budget.
- `estimated_tokens`: deterministic pack token estimate.
- `truncated`: whether packing had to truncate or omit material for budget.
- `citations_required`: whether selected cells were required to carry citation
  material.
- `answerability_q16`: integer Q16 answerability signal.
- `conflict_visibility_q16`: integer Q16 visible-conflict signal.
- `visible_conflict_count`: count of visible conflict groups in the pack.
- `cells`: ordered selected `ContextPackCell` entries.
- `anomalies`: ordered `ContextPackAnomaly` entries such as
  `insufficient_context`, `visible_conflict`, and `retrieval_incomplete`.
- `grounding_report`: optional deterministic post-answer grounding evidence.
- `accountability_receipt`: optional `accountability_receipt.v1` proof object.

Each `ContextPackCell` carries the governed evidence item:

- `cell_id`: stable cell identity used by evidence and receipts.
- `payload` or public `payload_text`: selected cell text bytes.
- `metadata`: parsed descriptor metadata used for scope, source, TTL, trust,
  and citation decisions.
- `estimated_tokens`: deterministic per-cell token estimate.
- `citation`: optional citation string.
- `provenance`: optional `ContextSpanProvenance`.
- `explain`: optional ranking/explain trail.
- `access_decision`: optional `ContextAccessDecision`.

`ContextAccessDecision` records why a cell was admitted:

- `cell_id`
- `decision`, currently `allowed` or `not_recorded`
- `policy`
- `policy_version`
- `reason`
- `scope`
- `scope_id`
- `agent_id`
- `agent_view_digest`

`ContextSpanProvenance` records byte and line provenance:

- `source_cell_id`
- `source_byte_start`
- `source_byte_end`
- `source_line_start`
- `source_line_end`
- `source_ref`

`ContextPackAnomaly` records governed incompleteness or exclusion signals:

- `cell_id`
- `code`
- `message`
- `why_excluded`

The `code` enum includes `redundant_cell`, `missing_citation`,
`token_overload`, `scope_mismatch`, `insufficient_context`,
`visible_conflict`, and `retrieval_incomplete`.

## Verification Output

`VERIFY FACT` is the LLM-free verification output associated with GCE answers.
Its typed surface is defined in `crates/cortex-engine/src/verification/types.rs`.
The source types are `VerificationReport`, `VerificationEvidence`,
`VerificationGuard`, and `VerificationNumericConflict`. A conforming
implementation must expose:

- `status`: `supported`, `insufficient`, `contradicted`, or `mixed_evidence`.
- `confidence_q16`: integer Q16 confidence.
- `evidence`: supporting admitted cell evidence.
- `contradicting_evidence`: contradicting admitted cell evidence.
- `guards`: safe guardrail reasons such as missing citation or stale fact.
- `numeric_conflicts`: structured conflicts with `kind` values `numeric`,
  `temporal`, and `citation`.

Verification evidence must refer only to admitted cells, and the hashed or
canonical verification surface must exclude wall-clock telemetry.

## Six GCE Invariants

Invariant 1 - Compiled governed context.

A GCE result is compiled governed context, not a raw nearest-neighbor list. The
result is selected through AQL, policy, scope, budget, citation, provenance,
conflict, and ranking logic, then emitted as `ContextPack`.

Invariant 2 - Deterministic LLM-free Q16 governance.

Governance signals are deterministic integer values. Q16 values use
`65535` as full scale. ContextPack scoring, answerability, conflict visibility,
source trust, source freshness, verification confidence, and recall/quality
signals must not require an LLM call in the core governance path.

Invariant 3 - Fail-closed plan algebra.

The retrieve binder seeds every plan with `PushAgentAllowed`, `PushLive`, and
`BitmapOp::And`, then only ANDs the user `where_clause` constraints on top.
This source is `crates/cortex-aql/src/binder.rs`. A conforming implementation
must not widen read access after planning. For future third-party receipt
verification, trusting an `allowed` leaf alone is insufficient; the verifier
must be able to check the bound plan evidence or an equivalent fail-closed
commitment.

Invariant 4 - Provenance and verification are first-class outputs.

Citation strings, span provenance, source references, access decisions,
verification evidence, guards, and numeric conflicts are result fields, not
debug logs. A conforming implementation must expose them in typed JSON and keep
them stable enough for SDK consumers and offline verifiers.

Invariant 5 - Conflict preservation, not LWW.

Conflicting evidence must not be silently overwritten by last-write-wins
behavior. A conforming implementation must preserve visible conflict signals
through `conflict_visibility_q16`, `visible_conflict_count`,
`visible_conflict` anomalies, and `VERIFY FACT` `numeric_conflicts`.

Invariant 6 - TTL/decay participates in ranking.

TTL, memory decay, and feedback decay are governance signals. A conforming
implementation may rank recent, unexpired, useful context above stale or
negative-feedback context, but must keep the computation deterministic and
explainable. Current sources include `crates/cortex-engine/src/memory/lifecycle.rs`
and `crates/cortex-engine/src/feedback/index.rs`.

## Conformance Obligations

A conforming GCE implementation MUST:

- MUST emit `context_pack.v1` or a versioned successor with an explicit schema.
- MUST preserve additive compatibility for optional fields until the next major
  schema version.
- MUST enforce fail-closed access before result packing.
- MUST use deterministic integer or Q16 governance in the core path.
- MUST expose `ContextPack` cells, anomalies, access decisions, and provenance.
- MUST expose `VERIFY FACT` status, evidence, guards, and numeric conflicts.
- MUST preserve conflict signals instead of silently dropping disagreements.
- MUST surface retrieval or grounding incompleteness when it can affect answer
  accountability.
- MUST keep optional receipt material additive and validate it against the
  receipt schema when present.

A conforming GCE implementation MUST NOT:

- MUST NOT require an LLM to decide access, budget, conflict status, or
  verification status in the core governance path.
- MUST NOT claim production-grade external transparency from a local signed
  receipt alone.
- MUST NOT claim KMS/HSM custody, compliance immutability, or external witness
  guarantees unless those gates are implemented and green.
- MUST NOT treat application-side post-filtering as equivalent to fail-closed
  plan algebra unless the enforcement evidence is independently checkable.

## Current CortexDB Gate Evidence

The current implementation status is tracked in
`docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md`.

Relevant gates include:

- `context-pack-schema-contract-check`
- `canonical-serialization-check`
- `context-access-decision-capture-check`
- `fail-closed-end-to-end-check`
- `verification-quality-check`
- `ranking-frozen-weights-check`
- `pack-determinism-hash-check`
- `pack-completeness-signal-check`
- `accountability-receipt-check`
- `transparency-anchor-check`
- `receipt-replica-invariance-check`

SPEC-1 only publishes and gates this contract document. Later order-8 work adds
receipt threat-model checks and public conformance/adversarial suites.

## Non-Goals And Claim Boundaries

This contract does not claim that every cell is factually true. It defines how
governed evidence is selected, exposed, canonicalized, and optionally committed
for accountability.

CortexDB can optionally write a local transparency log when
`CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE` is set with configured receipt signing.
Each `cortexdb.transparency.log.record.v1` entry chains `pack_root`,
`determinism_hash`, receipt signature, and database/key identity, and the local
gate rejects two records with the same determinism_hash but different
`pack_root` values. This is a local anti-equivocation anchor, not a
third-party witness or compliance ledger.

`transparency-witness-check` adds an external mirror witness record:
`cortexdb.transparency.witness.record.v1` signs the verified local log head,
sequence range, and first/last receipt identities with an independent witness
Ed25519 key. This proves that a separate mirror can countersign the local
`pack_root` chain head for off-database custody. It is not a public
transparency service, CT-style gossip protocol, or compliance ledger.

`transparency-witness-quorum-check` adds
`cortexdb.transparency.witness.quorum.v1` evidence: multiple independently
signed witness records must agree on the same verified log head and sequence
range, and duplicate witness ids, key ids, or public keys are rejected. This
is an independent witness quorum over the local transparency head, not public
transparency service availability, a CT-style gossip protocol, KMS/HSM
custody, or compliance immutability.

`transparency-inclusion-check` adds
`cortexdb.transparency.inclusion.proof.v1` evidence: a local transparency log
record can carry a Merkle inclusion proof from its sequence-bound record hash
to the published transparency root for that log snapshot. The verifier rejects
record-hash tampering and sibling-path tampering. This is a Merkle inclusion
proof primitive for public audit clients, not public transparency service
availability, CT-style gossip/consistency exchange, KMS/HSM custody, or
compliance immutability.

`transparency-consistency-check` adds
`cortexdb.transparency.consistency.v1` append-only consistency evidence:
published transparency snapshots can be compared by record-hash prefixes and
Merkle roots so divergent prefixes and truncated newer snapshots are rejected.
This is a public-monitor consistency primitive, not public transparency service
availability, network gossip fanout, independent monitor uptime, KMS/HSM
custody, or compliance immutability.

`transparency-availability-check` adds
`cortexdb.transparency.availability.evidence.v1` public monitor availability
evidence: fresh HTTPS monitor observations must come from independent monitor
ids and URLs, return available 2xx service status, satisfy monitor uptime and
freshness policy, and agree on the published log count, log head, and Merkle
root. This is CI-safe evidence for public transparency service availability
and independent monitor uptime, not network gossip fanout, KMS/HSM custody, or
compliance immutability.

`transparency-gossip-check` adds
`cortexdb.transparency.gossip.evidence.v1` network gossip fanout evidence:
fresh monitor-to-monitor exchanges must be delivered over HTTPS, satisfy the
declared per-monitor fanout, and agree on the published log count, log head,
and Merkle root. Stale exchanges, insufficient fanout, and split log heads are
rejected. This is CI-safe gossip fanout evidence, not continuous production
SLO compliance, Byzantine monitor key custody, KMS/HSM custody, or compliance
immutability.

`transparency-slo-check` adds
`cortexdb.transparency.slo.evidence.v1` continuous public transparency
operations evidence: ordered operations windows must cover the declared period
without gaps, meet the declared availability percentage, carry monitor quorum
and gossip fanout summaries, report append-only consistency status, and keep
log counts monotonic. Gaps, below-target availability, log-count regression,
and same-count split heads are rejected. This is CI-safe operations/SLO
evidence, not live production deployment, Byzantine monitor key custody,
KMS/HSM custody, or compliance immutability.

`receipt-replica-invariance-check` adds a signed `audit_chain_head` commitment
to `accountability_receipt.v1` and proves that the signed header is
byte-identical for the same committed receipt inputs while changing if the
audit-chain head changes. This is a CI-safe SCALE-3 receipt/audit-head slice,
not a full live multi-node failover proof.

This contract does not close production-grade issuer equivocation by itself.
Live production deployment, Byzantine monitor key custody, KMS/HSM custody,
release-lane soak stability, and compliance immutability remain separate
claims.

This contract does not require a live multi-competitor benchmark in the fast
release lane. A small conformance suite and a documented thin-wrapper reference
are the required category proof path; heavier live baseline matrices belong in
nightly or evidence lanes.
