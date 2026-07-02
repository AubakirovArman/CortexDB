# GCE Conformance Suite

Status: CI-safe conformance and adversarial contract.

This document defines the fast-lane Governed Context Engine conformance suite.
It is intentionally smaller than a live head-to-head benchmark. The goal is to
prove the structural category boundary: CortexDB passes the accountability
axes, while a documented thin-wrapper reference fails the axes that require a
governed engine rather than application-side post-filtering.

Normative companion documents:

- `docs/spec/GCE_CONTRACT.md`
- `docs/spec/RECEIPT_VERIFIER.md`
- `docs/spec/ACCOUNTABILITY_RECEIPT_V1.md`

## Fast-Lane Cases

The CI-safe suite includes these adversarial cases:

- `scope_widening`: a user query or application layer must not widen access
  beyond the plan-bound readable set.
- `fabricated_citation`: a citation/span mutation must be rejected by receipt
  verification.
- `dropped_conflict`: removing `visible_conflict` evidence must be rejected.
- `forged_audit_entry`: audit/receipt binding must reject tampered receipt
  hashes or chain material.
- `anti_correlation`: anti-correlated vectors must not score as perfect cosine
  matches.
- `receipt_verifiability`: a third party must be able to verify a receipt
  without linking the engine.
- `determinism`: canonical pack/verification bytes and determinism hashes must
  be stable across process and restart boundaries.
- `plan_binding`: access evidence must be tied to fail-closed plan algebra, not
  only to an application-side post-filter.

## CortexDB Evidence Gates

`aab-conformance-check` runs or requires these existing gates before writing
the conformance report:

- `gce-spec-doc-check`
- `receipt-threat-model-check`
- `accountability-receipt-verify-check`
- `accountability-receipt-tamper-check`
- `pack-determinism-hash-check`
- `fail-closed-end-to-end-check`
- `cosine-metric-correctness-check`
- `verification-quality-check`
- `audit-receipt-binding-check`

The check then validates
`fixtures/gce_conformance/thin_wrapper_reference.json` and writes
`target/gce-conformance/report.json`.

## Thin-Wrapper Reference

The thin-wrapper reference is a documented pgvector plus policy-engine plus RAG
library shape. It may retrieve, cite, and sign application JSON, but it is not a
conforming GCE in this suite unless it supplies third-party-verifiable receipt
roots, deterministic canonical bytes, and plan-bound access evidence.

The required thin-wrapper failure axes are:

- `receipt_verifiability`
- `determinism`
- `plan_binding`

The reference may pass ordinary retrieval/citation/token cases. The conformance
result is still non-conforming when any required accountability axis fails.

## Pass Criteria

CortexDB passes when every fast-lane case is backed by a green gate and the
generated report has:

- `cortexdb_passed_all=true`
- `thin_wrapper_failed_axis_count >= 3`
- `thin_wrapper_failed_required_axes` containing `receipt_verifiability`,
  `determinism`, and `plan_binding`

The suite is not a production transparency claim. External witnessed
transparency for `pack_root`, KMS/HSM custody, and compliance immutability
remain separate roadmap gates.
