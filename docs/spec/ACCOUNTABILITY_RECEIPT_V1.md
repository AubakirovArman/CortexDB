# Accountability Receipt v1

Status: schema frozen; runtime JSON emission is enabled when a receipt signing
key is configured.

The normative JSON Schema lives at
[`docs/schemas/accountability_receipt.v1.json`](../schemas/accountability_receipt.v1.json).
The golden fixture lives at
[`docs/schemas/accountability_receipt.v1.golden.json`](../schemas/accountability_receipt.v1.golden.json).
The contract is guarded by `make accountability-receipt-schema-check`.

`accountability_receipt.v1` is an additive optional field on `context_pack.v1`.
Consumers that only understand the original v1 fields must continue to parse a
ContextPack when the field is absent. Consumers that see the field can validate
the receipt object against the receipt schema.

Runtime JSON receipt emission is fail-closed behind configured receipt key custody.
When no receipt signing key is configured, `accountability_receipt` remains absent.
Prompt/markdown exports do not embed the receipt object.

## Receipt Key Custody Modes

The local runtime custody mode is local-seed backed. Server startup accepts
`CORTEXDB_RECEIPT_SIGNING_KEY_FILE` or `CORTEXDB_RECEIPT_SIGNING_KEY_HEX`,
derives the Ed25519 public key from `signing_seed_hex`, and signs the canonical
receipt header in-process.

The External signer/KMS-HSM custody contract is intentionally stricter and is
the non-local runtime path. Server startup accepts
`CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND`,
`CORTEXDB_RECEIPT_EXTERNAL_SIGNER_KEY_ID`,
`CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX`, and optional
`CORTEXDB_RECEIPT_EXTERNAL_SIGNER_REF`. In external signer mode, the server must
not load `signing_seed_hex` or any equivalent signing seed into process memory.
It holds only `key_id`, `public_key_hex`, the external command, and an
operator-provided signer reference. For every receipt, the server sends the
canonical unsigned receipt header bytes, `key_id`, `public_key_hex`, and signing
domain `cortexdb.accountability_receipt.sign.v1` to the command as
`cortexdb.receipt_external_sign_request.v1`; the command returns
`cortexdb.receipt_external_signature.v1` with `signature_hex`. The server must
verify the returned signature against the configured public key before emitting
the receipt. Production custody must fail closed; no fallback to local seed is
allowed when the external signer is unavailable or returns an invalid
signature. This runtime command path is not itself proof of KMS/HSM custody;
that claim requires separate operator evidence that the command is backed by
KMS/HSM-held key material.

The custody evidence schema is
`cortexdb.receipt_kms_hsm_custody_evidence.v1`. A valid evidence file records
`custody_mode`, provider name, provider key reference, `signer_ref`, `key_id`,
`public_key_hex`, signing domain, `key_material_exportable=false`,
`local_seed_material_present=false`, external-command request/response schema
binding, a `runtime_signing_probe` signed by the same runtime public key,
operator attestation controls, and hashed custody artifacts. The probe records
the external signer request/response schemas, matching `key_id`,
`public_key_hex`, and `signer_ref`, a canonical header challenge, request and
response SHA-256 digests, `signature_hex`, `signature_sha256_hex`, and
`signed_at`; the signature must verify over
`cortexdb.accountability_receipt.sign.v1 || 0x00 || canonical_header_hex bytes`.
The gate only treats this as KMS/HSM custody when the evidence also matches the
expected runtime `key_id`, `public_key_hex`, and `signer_ref` supplied to
`receipt-kms-hsm-custody-check`, carries a valid
`cortexdb.operator_evidence_origin_proof.v1` `production_origin_proof`, and
binds that proof to separately supplied expected key-attestor inputs:
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF`. A valid
runtime signing probe alone is runtime signer evidence, not KMS/HSM custody
evidence.

## Standalone Verification Input

AR-7 adds `cortex-receipt-verify`, a standalone verifier binary that does not
link `cortex-engine`, `cortex-storage`, `cortex-aql`, or `cortex-server`.
Its fixture input is
`cortexdb.accountability_receipt_verify_input.v1` with:

- `pack`: the public canonical pack JSON committed by `pack_root`.
- `determinism_input`: the public determinism commitment input
  (`schema_version`, query, AgentView/options/bitmap digests, and
  `frozen_weights.version` plus `frozen_weights.artifact_hash`).
- `receipt`: the `accountability_receipt.v1` object.
- `public_key`: `{key_id, public_key_hex}` for the receipt signing key.
- `admitted_cells`: public `{cell_id, cell_content_hash, raw_content_hex?}`
  evidence for admitted cells and span-bound checks.

The verifier independently canonicalizes JSON, recomputes every root, verifies
the Ed25519 header signature, checks admitted access leaves are `allowed`,
checks public cell hashes, validates byte-span bounds when raw bytes are
present, checks budget consistency, and rejects verification leaves that
reference non-admitted cells.

The AR-8 tamper matrix must accept the genuine verifier fixture and reject:
budget mutation, admitted access mutation, provenance span mutation, dropped
visible conflict, swapped verification status, replay under a different
determinism input, and signature-byte mutation.

## Shape

The receipt has three top-level fields:

- `schema_version`: always `accountability_receipt.v1`.
- `header`: the fixed receipt header containing hash/signature algorithms,
  root commitments, `pack_root`, `determinism_hash`, `audit_chain_head`, and
  signature material.
- `leaves`: the ordered leaf sets consumed by the future Merkle roots:
  `access`, `provenance`, `cell_set`, `verification`, `budget`, and `conflict`.

The header fixes the cryptographic contract to `blake3-256` roots and
`ed25519` signatures. AR-4 fills the roots from canonical leaf bytes, AR-5
signs the canonical header with the receipt key custody path, and AR-7
recomputes the same schema from public inputs in a standalone verifier.

`audit_chain_head` is the 32-byte audit-chain tail observed before receipt
emission. When persisted audit logging is disabled or no audit JSONL path is
configured, the value is the audit-chain zero hash. Including this field in the
signed header makes same-input replica receipt comparison explicit: two
eligible replicas must share the same database-instance id, key id/signing key,
receipt timestamp epoch, pack roots, determinism input, and committed audit
chain head to emit byte-identical receipts.

## Additive ContextPack Field

`context_pack.v1` keeps all existing required fields unchanged. The optional
`accountability_receipt` property is additive and may be absent or `null` when
receipt signing key custody is not configured.

This preserves the v1 compatibility rule: existing required fields, enum
meanings, and `schema_version` are frozen; optional additive fields are allowed
until `context_pack.v2`.
