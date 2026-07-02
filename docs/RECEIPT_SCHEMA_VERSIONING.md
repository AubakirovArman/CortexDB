# Receipt / Canonical Schema Versioning (C3-5)

The accountability receipt signs a **canonical serialization** of the governed
result — the ContextPack and the VerificationReport. The exact set of fields
that enter those canonical bytes is a contract: a verifier re-executes the plan
and must reproduce the same signed bytes. Adding, removing, or renaming a
canonical field silently would change what the receipt signs and break that
contract. This document defines the only allowed way to change it.

## What is bound

Two field sets in [`canonical.rs`](../crates/cortex-engine/src/canonical.rs) are
the canonical (hashed / receipt-signed) surface:

- `CONTEXT_PACK_HASHED_FIELDS` → schema `context_pack.canonical.v1`
- `VERIFICATION_REPORT_HASHED_FIELDS` → schema `verification_report.canonical.v1`

Each set is bound to its schema version by
[`fixtures/canonical/schema_field_binding_v1.json`](../fixtures/canonical/schema_field_binding_v1.json)
and guarded by the `canonical_field_sets_are_bound_to_schema_versions` test
(run by `make canonical-schema-field-binding-check`, part of
`make accountability-receipt-check`). The test fails if a canonical field set
changes without a matching fixture entry — so an un-versioned change cannot land.

## Additive-minor procedure

A plan-visible retrieve option or a new piece of signed metadata — for example
`embedding_ref` (A2.1), a `TemporalWindow` option (A4.1), A4.2 coverage, A7.3
diversity, or B2.1 `latest_only` — enters the canonical serialization **only**
through this procedure, all in **one PR**:

1. **Bump the canonical schema version** for the affected surface (e.g.
   `context_pack.canonical.v1` → `context_pack.canonical.v2`) in `canonical.rs`,
   and add the field to the corresponding `*_HASHED_FIELDS` set.
2. **Add a new fixture entry** in `schema_field_binding_v1.json` keyed by the new
   schema version, listing the new sorted field set. Keep the old entry (frozen).
3. **Re-baseline the determinism goldens** in the same PR: `pack-determinism-hash-check`,
   `accountability-receipt-determinism-check`, and the receipt schema golden
   (`docs/schemas/accountability_receipt.v1.golden.json`).
4. **Run the cross-language SDK-type check** (C4-2) so the generated Python /
   TypeScript receipt types move together with the schema.

The binding test enforces steps 1–2 (a field-set change without a version bump
or fixture entry fails); the determinism gates enforce step 3. This procedure
**must be in place before A7.3 lands** (per the master plan) so plan-visible
ranking/diversity options cannot silently change the signed surface.

## Why additive-minor, not in-place

Old receipts stay verifiable: a verifier that knows `…canonical.v1` can still
re-check a v1 receipt, while new receipts carry `…canonical.v2`. Because the old
fixture entry and golden are frozen, the change is provably additive.

## Related

- [`ACCOUNTABILITY_RECEIPT_V1.md`](spec/ACCOUNTABILITY_RECEIPT_V1.md) — receipt spec.
- [`STORAGE_FORMATS.md`](STORAGE_FORMATS.md) — the manifest `EMBD` section that
  records the embedding profile whose `embedding_ref` a future minor bump would
  promote into the receipt.
