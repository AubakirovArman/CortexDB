# Accountability Implementation Status

Snapshot: 2026-06-29.

Source plan: `docs/ACCOUNTABILITY_ROADMAP.md`.

Ordering rule: execute the reconciled roadmap in dependency order. Do not build
or sign `accountability_receipt.v1` before Phase 0 canonical bytes and Phase 1
evidence-correctness gates are green.

## Current Ordered Status

| Order | Phase | Status | Gate evidence |
|---:|---|---|---|
| 0 | Canonical serialization + determinism foundation | done | canonical bytes, field classification, cross-process fixture, and determinism gates are in place |
| 1 | Correctness prerequisites | done | `correctness-prerequisites-check` aggregates cosine, cell-id, conflict normalization, ANN disclosure, ANN metric matrix, conflict visibility, and determinism gates in release/beta wiring; CP-3 is closed as `agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence`, with any true 31-bit session/feedback widening requiring a new persisted schema version plus migration/refuse-to-read gate |
| 2 | Real crypto base | done | `crypto-foundation-check` is green and wired into `security-gate-v2-check`/`security-release-report-check`; CRY-1 through CRY-8 are green, including configured local receipt emission, CRY-5 audit receipt-hash binding, and CRY-6 receipt/audit re-anchor records |
| 3 | Accountability receipt MVP | done | AR-2 receipt schema freeze, AR-3 DB-computed `cell_content_hash`, admitted/denied access inputs, AR-4 deterministic receipt body/root assembly, AR-5 signed header, AR-7 standalone verifier, AR-8 tamper/umbrella gates, and configured-key runtime JSON ContextPack/VERIFY emission are green |
| 4 | Captured access + standalone verifier | done | admitted AQL cells carry captured `Allowed` decisions with `policy_version` and `agent_view_digest`; denied candidates excluded by AQL agent access filtering produce bounded hash-only evidence in `RetrieveExecutionReport`; standalone no-engine verifier is green against public verifier fixtures |
| 5 | Physical fail-closed parity | done | FC-2 `ann-scope-parity-check` gates bound bitmap program derived persisted ANN/lexical allowed-set parity; FC-3 `ann-sparse-scope-recall-check` gates bounded sparse-relative-to-graph allowed-set exact top-k routing with `sparse_allowed_set`; FC-6 `scope-leak-bench-check` gates >=200 output surfaces with zero forbidden sentinel bytes; FC-7 `fail-closed-invariant-model-check` exports pinned `model_hash`; FC-8 `fail-closed-end-to-end-check` aggregates FC-1 through FC-7 plus private-scope/determinism gates and is wired into the beta release lane |
| 6 | Crypto hardening for audit/backups | done | reconciled with current gates: `crypto-foundation-check` covers real AEAD backup, keyed audit chain, audit receipt-hash binding, key management/rotation, database instance identity, secrets hygiene, and honest crypto claims |
| 7 | Verification strength + frozen deterministic ranker | done | DV1 `context-pack-conflict-visibility-check` proves ContextPack/VERIFY numeric conflict agreement; DV2 `verify-numeric-normalization-check` gates integer-only unit-class normalization and labeled `CurrencyMismatch`; DV3 `verify-multivalue-extraction-check` gates contextual multi-value fact indexing; DV4 `verify-temporal-conflict-check` gates dated numeric conflicts and temporal overlap filtering; DV5 `verify-citation-conflict-check` gates same-`source_ref` value disagreement as a typed citation conflict; DV6 `verify-determinism-check` gates clock-free canonical VERIFY conflict serialization; DV7-DV8 measured conflict recall/docs claims are green; RANK-1 freezes current ranking constants behind `ranking-frozen-weights-check`; RANK-2 compiles trainer-selected profiles into the artifact with `ranking-weights-drift-check`; RANK-3 proves engine-side learned lift with `ranking-learned-lift-check`; REPRO-2 binds `determinism_hash` to the frozen weights artifact with `weights-version-binding-check`; REPRO-3 proves cross-process pack/verify determinism hashes with `pack-determinism-hash-check`; RANK-4 proves ContextPack explain score faithfulness with `ranking-explain-faithfulness-check`; RANK-5 proves hashed pack completeness signals with `pack-completeness-signal-check` |
| 8 | GCE spec + conformance | done | SPEC-1 publishes the normative `docs/spec/GCE_CONTRACT.md` and gates it with `gce-spec-doc-check`; SPEC-3 publishes the offline receipt verifier algorithm/threat model and gates it with `receipt-threat-model-check`; CONF-1 publishes CI-safe GCE conformance/adversarial coverage and gates it with `aab-conformance-check` |
| 9 | Transparency log + cluster accountability | partial | local `transparency-anchor-check` gates chained `pack_root` transparency records, tamper detection, and same-determinism equivocation rejection; `transparency-witness-check` gates an external mirror witness record that signs the verified local transparency log head and sequence range with an independent witness Ed25519 key; `transparency-witness-quorum-check` gates independent witness quorum evidence for multiple unique witness ids/key ids/public keys that agree on the same verified log head and rejects split heads; `transparency-inclusion-check` gates Merkle inclusion proofs from sequence-bound transparency record hashes to a published transparency root and rejects record/path tampering; `transparency-consistency-check` gates append-only consistency evidence across published transparency snapshots and rejects divergent prefixes/truncated newer snapshots; `transparency-availability-check` gates fresh public HTTPS monitor observations with independent monitor ids/URLs, 2xx availability, monitor uptime, freshness, and single log head/root agreement; `transparency-gossip-check` gates fresh delivered monitor-to-monitor HTTPS exchanges with required per-monitor fanout and single log head/root agreement; `transparency-slo-check` gates continuous public transparency operations windows with no period gaps, declared availability target, monitor quorum/fanout summaries, append-only consistency status, and monotonic log counts; `receipt-replica-invariance-check` gates signed `audit_chain_head` binding plus byte-identical canonical ContextPack bytes, `determinism_hash`, and signed `accountability_receipt.v1` bytes across two real `Database` instances after replicated snapshot install; `consensus-failover-binder-check` gates the `[PushAgentAllowed, PushLive, And]` binder seed and zero forbidden cells through follower-read, failover, partition, and partition-heal harness paths; `multi-agent-cluster-consistency-check` gates read-your-writes and monotonic-read across nodes through follower promotion and partition heal at declared `MemoryConsistencyLevel`; `http-raft-routing-accountability-check` gates Axum HTTP ContextPack/receipt reads from arbitrary replicated node roots after TCP Raft snapshot install; `raft-ingress-production-guard-check` gates configured multi-node cluster status plus fail-closed primary-unavailable ingress; `raft-ingress-forwarding-check` gates fixed-primary live HTTP forwarding from a non-primary server to the configured primary while preserving signed accountability receipt payloads; `raft-ingress-leader-hint-check` gates explicit operator-provided leader-id ingress forwarding and fail-closed unknown-leader behavior; `raft-ingress-auto-discovery-check` gates separate Raft/HTTP topology, read-only Raft `STATUS` leader discovery, and live HTTP forwarding to the discovered leader ingress; `raft-ingress-health-routing-check` gates remote leader HTTP health probing and stale-leader skip-through to a newer Raft peer leader during a simulated leadership change; `raft-ingress-lifecycle-monitor-check` gates startup/background cached monitor state used by the Axum ingress path after Raft status peers become unavailable; `raft-ingress-load-policy-check` gates per-leader in-flight admission for cached monitor forwarding; `raft-ingress-adaptive-scheduling-check` gates adaptive refresh-on-overload when a cached leader is saturated and Raft status reports a newer known leader; `raft-ingress-load-metrics-check` gates operator-tunable cached ingress load limit plus Prometheus cached ingress load gauges; `consensus-release-lane-check` gates N consecutive soak-green partition/failover/rejoin plus SCALE-1/SCALE-2/SCALE-3 runs from `release-check`; `receipt-kms-hsm-custody-check` records external command signer runtime plus optional operator KMS/HSM evidence validation while default runs keep the evidence blocker; `compliance-boundary-check` records optional operator external compliance certification/immutability evidence while default runs keep the certification blocker; `receipt-production-readiness-check` aggregates receipt/transparency/key-management/KMS-HSM-custody/security/compliance evidence and reports production-grade public receipt blockers with `production_ready=false`; `receipt-production-ready-check` is the strict production claim gate and fails until `production_ready=true`; multi-writer load-balancing and production HA claims are not made |

## Done

1. Phase 0 slice P0.1: added an internal canonical serialization module for
   `ContextPack`, `VerificationReport`, and arbitrary JSON values.
2. Phase 0 slice P0.1: added explicit hashed-field allowlists for current
   canonical `ContextPack` and `VerificationReport` surfaces.
3. Phase 0 slice P0.1: added explicit telemetry exclusions for
   `elapsed_nanos`, `total_elapsed_nanos`, `Instant`, and `SystemTime`.
4. Phase 0 slice P0.1: added `make canonical-serialization-check` and
   `make accountability-canonical-check` targets.
5. Phase 0 slice P0.2: restored live `docs/ENGINE_DETERMINISM.md` and linked
   the canonical accountability surface into the determinism contract.
6. Phase 0 slice P0.3: added cross-process canonical byte identity coverage via
   `accountability_canonical_fixture`.
7. Phase 0 slice P0.4: upgraded `scripts/canonical_serialization_check.py` to
   fail when top-level `ContextPack` or `VerificationReport` fields are not
   classified as hashed, telemetry, or exported-only.
8. Phase 1 slice CP-1/CP-2: replaced HNSW cosine `dot.abs()` scoring with the
   shared signed-clamp `cosine_similarity_q16` helper.
9. Phase 1 slice CP-1/CP-2: made ContextPack redundancy and HNSW cosine use one
   shared implementation to prevent metric drift.
10. Phase 1 slice CP-1/CP-2: added `make cosine-metric-correctness-check` and
    `make hnsw-cosine-correctness-check` gates.
11. Phase 1 slice CP-3: routed memory, session, and feedback derived cell ids
    through one guarded `cell_ids` helper and removed silent 28-bit truncation
    from session/feedback constructors.
12. Phase 1 slice CP-3: documented the current top-nibble namespace layout as a
    28-bit agent-slot plus 32-bit sequence domain, with overflow returning a
    storage invariant instead of aliasing another agent.
13. Phase 1 slice CP-3: added `make cell-id-collision-check` to prove max-slot
    preservation, overflow rejection, and absence of old mask-truncation in the
    production paths.
14. Phase 1 slice CP-4: wired ContextPack conflict visibility through the
    existing `verification/numeric` parser and comparator so normalized-equal
    values no longer create false `VisibleConflict` groups.
15. Phase 1 slice CP-4: extended integer-only numeric normalization for currency
    symbols, suffixed units, and compatible unit classes without adding floats
    or new dependencies.
16. Phase 1 slice CP-4: broadened project/metric/value extraction to
    case-insensitive `key=value` and `key: value`, and carries `currency`/`unit`
    context into numeric value comparison.
17. Phase 1 slice CP-4: added `make conflict-normalization-check` with labeled
    magnitude/currency/unit fixtures, string fallback coverage, determinism
    coverage, and a JSON report.
18. Phase 1 slice CP-5: added `ContextPackAnomalyCode::RetrievalIncomplete`
    and plumbed ANN `VisitBudgetExceeded` reports into `ContextPack` via the
    search-outcome packing path.
19. Phase 1 slice CP-5: verified the anomaly is pack-level, scope-byte-free,
    and exported through JSON, prompt, and markdown ContextPack formats.
20. Phase 1 slice CP-5: added `make ann-budget-disclosure-check`, updated the
    ContextPack JSON Schema, OpenAPI enum, and generated SDK enum literals for
    `retrieval_incomplete`.
21. Phase 1 slice CP-6: added `make correctness-prerequisites-check` as the
    aggregate P1 gate over cosine, cell-id, conflict normalization,
    ANN-disclosure, ANN metric matrix, conflict visibility, and engine
    determinism checks.
22. Phase 1 slice CP-6: wired `correctness-prerequisites-check` into
    `alpha-check` and the beta release evidence bundle so correctness
    prerequisites block release/receipt work instead of living as standalone
    optional checks.
23. Phase 1 slice CP-6: added
    `scripts/correctness_prerequisites_check.py` and
    `CORRECTNESS_PREREQUISITES_REPORT` to keep aggregate wiring auditable and
    prevent circular rework.
24. Phase 2 preflight: added feature-gated RustCrypto dependency onboarding
    for `accountability-receipt` in `cortex-engine`, `cortex-server`, and
    `cortex-cli` without moving the dependency declarations into workspace
    dependencies.
25. Phase 2 preflight: added `make crypto-deps-readiness-check` and
    `scripts/crypto_deps_readiness_check.py` to prove dependency/readiness
    wiring while explicitly reporting legacy backup/audit policy blockers.
26. Phase 2 preflight: updated `Cargo.lock` through
    `cargo check --workspace --all-features`, proving the new optional crypto
    dependency graph resolves with the current workspace.
27. Phase 2 slice CRY-2: added a dedicated `cortex-crypto` workspace crate as
    the single thin wrapper surface for SHA-256/BLAKE3 hashing,
    XChaCha20-Poly1305 AEAD, Argon2id KDF, HMAC-SHA-256, Ed25519, KeyId,
    constant-time compare, and zeroized secret byte wrappers.
28. Phase 2 slice CRY-2: added KAT/regression tests for SHA-256, BLAKE3,
    XChaCha20-Poly1305 seal/open/tamper rejection, Argon2id, HMAC-SHA-256,
    constant-time compare, and the RFC-8032 Ed25519 test vector.
29. Phase 2 slice CRY-2: added `make crypto-primitives-check` and
    `scripts/crypto_primitives_check.py` with a JSON evidence report for the
    shared primitive surface.
30. Phase 2 slice CRY-3: replaced encrypted backup v1 XOR/FNV routines with
    XChaCha20-Poly1305 using Argon2id-derived keys, random salt/nonce, and
    detached AEAD tags authenticated over v2 header AAD.
31. Phase 2 slice CRY-3: changed encrypted backup archive headers to
    `cortexdb.encrypted_backup.v2` /
    `cortexdb.xchacha20poly1305-argon2id.v2`, and added fail-closed refusal
    for legacy v1 archives before any decode path can trust them.
32. Phase 2 slice CRY-3: added encrypted-backup tamper coverage for
    ciphertext, tag, nonce, salt, and AAD/header mutation, plus
    `make encrypted-backup-legacy-refuse-check` with a JSON report.
33. Phase 2 slice CRY-4: added shared audit-chain primitives in
    `cortex-crypto` with the v2 audit chain domain, 64-hex zero hash,
    SHA-256 event hashing, HMAC-SHA-256 event MAC surface, and regression
    coverage for order sensitivity and keyed MAC output width.
34. Phase 2 slice CRY-4: moved server and CLI audit-chain hashing off the
    duplicated FNV implementations and onto `cortex_crypto::audit_chain`, so
    verifier and writer code share one deterministic v2 hash surface.
35. Phase 2 slice CRY-1 exit: added `make crypto-deps-policy-check` and
    `scripts/crypto_deps_policy_check.py` to prove the shared crypto crate is
    consumed by engine/server/CLI and production backup/audit paths no longer
    contain the legacy FNV/XOR backup or audit-chain markers.
36. Phase 2 slice CRY-4/CRY-6: upgraded persisted audit JSONL records to
    `cortexdb.audit.v2` with `mac_key_id` and `event_mac`, and made file audit
    sinks fail closed unless an explicit `AuditMacKey` is configured.
37. Phase 2 slice CRY-6: added runtime audit MAC key parsing from
    `CORTEXDB_AUDIT_MAC_KEY_HEX` plus optional `CORTEXDB_AUDIT_MAC_KEY_ID`,
    with redacted key debug output and no direct secret value in the CLI audit
    verifier.
38. Phase 2 slice CRY-6: extended CLI audit verification and SIEM export to
    preserve and verify keyed v2 audit chains via `--mac-key-file`, while old
    v1 hash-chain fixtures remain readable as legacy records.
39. Phase 2 slice CRY-6: added `make key-management-check` and
    `scripts/key_management_check.py` to gate keyed audit writer/verifier
    wiring and the key-source contract.
40. Phase 2 slice CRY-7: updated public security, backup/restore, auth, audit
    format, and CLI docs so shipped claims distinguish current
    XChaCha20-Poly1305/Argon2id backup archives and SHA-256/HMAC-SHA-256
    audit v2 records from receipt signatures, KMS custody, immutable ledgers,
    and rotation guarantees that are still future work.
41. Phase 2 slice CRY-7: added `make crypto-claims-honesty-check` and
    `scripts/crypto_claims_honesty_check.py` to fail if public crypto docs
    regress to stale XOR/FNV, v1-only audit, or overbroad production claims.
42. Phase 2 slice CRY-7: refreshed audit productization and security-hardening
    marker checks so existing release evidence follows `cortexdb.audit.v2`,
    `CORTEXDB_AUDIT_MAC_KEY_HEX`, and `--mac-key-file`.
43. Phase 2 slice CRY-6 receipt-key custody: added
    `cortex_crypto::ReceiptSigningKey`, `ReceiptPublicKey`,
    `ReceiptSignature`, and `ReceiptKeyRing` with domain-separated Ed25519
    signing, deterministic signature coverage, and dual-trust verification for
    current plus previous public keys.
44. Phase 2 slice CRY-6 receipt-key custody: added
    `cortexdb receipt-key generate`, `receipt-key export-public`, and
    `receipt-key rotate` to write `cortexdb.receipt_signing_key.v1`,
    `cortexdb.receipt_public_key.v1`, and `cortexdb.receipt_trust.v1` files
    without printing `signing_seed_hex` in command output.
45. Phase 2 slice CRY-6 receipt-key custody: added server startup parsing for
    `CORTEXDB_RECEIPT_SIGNING_KEY_FILE` or
    `CORTEXDB_RECEIPT_SIGNING_KEY_HEX` plus `CORTEXDB_RECEIPT_SIGNING_KEY_ID`,
    with redacted `ReceiptSigningKey` debug output and public-key self-checking
    for JSON key files.
46. Phase 2 slice CRY-6 receipt-key custody: extended `make
    key-management-check` to cover receipt key generation, public export,
    rotation dual-trust, server file/env parsing, and existing keyed audit MAC
    behavior; `make secrets-check` remains green.
47. Phase 3 slice AR-3: added a DB-computed accountability cell hash surface in
    `cortex-engine` that canonicalizes `cell_id`, raw `payload_hex`, and the
    materialized `CellDescriptor` before applying domain-separated BLAKE3.
48. Phase 3 slice AR-3: added regression coverage proving the hash is
    deterministic, changes on a one-byte payload mutation, changes on
    descriptor metadata mutation, is domain separated, and is not copied from
    payload/descriptor `content_hash` strings.
49. Phase 3 slice AR-3: added `make accountability-cell-hash-check` plus
    `scripts/accountability_cell_hash_check.py` and
    `ACCOUNTABILITY_CELL_HASH_REPORT` so the pre-receipt cell identity anchor
    can be rerun without starting receipt emission or signing.
50. Phase 3 slice AR-6/FC-5: added captured access decisions on the AQL retrieval
    path by threading a `CapturedAccessDecision` from `EngineAqlProvider`
    through `CandidateResolver`, `QualityFilter`, and `RetrievedCell`.
51. Phase 3 slice AR-6/FC-5: extended ContextPack access decisions with
    `policy_version` and deterministic `agent_view_digest`, and made AQL
    ContextPack assembly consume captured enforcement evidence instead of
    relying only on pack-time re-derivation.
52. Phase 3 slice AR-6/FC-5: updated ContextPack JSON export, schema,
    OpenAPI/SDK generated types, server/SDK response models, and focused
    regression coverage so successful AQL packs do not emit admitted cells as
    `NotRecorded`.
53. Phase 2 slice CRY-8: added `make crypto-foundation-check` as an aggregate
    gate over crypto dependency policy, shared primitives, AEAD backup, legacy
    backup refusal, keyed audit chain, key management, secrets hygiene, and
    crypto public-claim honesty.
54. Phase 2 slice CRY-8: wired `crypto-foundation-check` into
    `security-gate-v2-check` and the security release report lane, with
    `scripts/crypto_foundation_check.py` consuming sub-reports instead of only
    checking target names.
55. Phase 2 slice CRY-8 support: corrected stale compliance/future-epic gate
    references from the old root enterprise RBAC compliance doc path to the
    existing archived design document so `security-release-report-check` can
    complete.
56. Phase 3 slice AR-6/FC-5 denied access: added bounded hash-only
    `CapturedAccessDenial` evidence and a `CapturedAccessDenialSet` on
    `RetrieveExecutionReport`, without adding forbidden payload bytes,
    descriptor scope strings, or raw denied `CellId` values to the report.
57. Phase 3 slice AR-6/FC-5 denied access: capture now records candidates
    excluded by AQL `PushAgentAllowed` bitmap filtering by comparing normal
    bitmap execution with an `agent_allowed=universe` bypass provider, while
    keeping the existing runtime `PermissionFilter` capture path for defense in
    depth.
58. Phase 3 slice AR-6/FC-5 denied access: extended
    `make context-access-decision-capture-check` and
    `scripts/context_access_decision_capture_check.py` to cover both admitted
    `Allowed` decisions and bounded denied/excluded access evidence.
59. Phase 3 slice AR-2: added frozen
    `docs/schemas/accountability_receipt.v1.json` plus a golden fixture that
    validates the receipt header, root commitments, six leaf sets, BLAKE3 hash
    roots, and Ed25519 signature material without enabling runtime emission.
60. Phase 3 slice AR-2: reserved `accountability_receipt` as an additive
    optional field on `context_pack.v1`, updated OpenAPI and generated Python /
    TypeScript SDK type artifacts, and added Rust SDK deserialization coverage
    proving v1 consumers can read the optional field.
61. Phase 3 slice AR-2: added `make accountability-receipt-schema-check` and
    `scripts/accountability_receipt_schema_check.py` to validate schema,
    golden fixture, ContextPack additive wiring, OpenAPI, docs, SDK coverage,
    and make/phony wiring.
62. Phase 3 slice AR-4: added an internal deterministic
    `accountability_receipt_body` builder that assembles access,
    provenance, cell-set, verification, budget, and conflict leaf sets into
    domain-separated BLAKE3 Merkle roots plus `pack_root` and
    `determinism_hash`.
63. Phase 3 slice AR-4: made receipt body assembly consume the Phase 0
    canonical pack/verification bytes, AR-3 DB-computed cell content hashes,
    captured admitted access decisions, and bounded hash-only denied access
    evidence without enabling public receipt emission or Ed25519 signing.
64. Phase 3 slice AR-4: split accountability receipt code into bounded
    modules and added `make accountability-receipt-determinism-check` with
    deterministic root, payload-mutation, determinism-input mutation, and
    fail-closed access-evidence coverage.
65. Phase 3 slice AR-5: added an internal signed receipt header over the
    deterministic receipt root commitments with frozen schema/hash/signature
    algorithm markers, `db_instance_id`, `key_id`, and `created_unix_seconds`.
66. Phase 3 slice AR-5: bound receipt header signing and verification to the
    existing `ReceiptSigningKey`, `ReceiptKeyRing`, `ReceiptSignature`, and
    `RECEIPT_SIGNING_DOMAIN` custody path without enabling public receipt
    emission.
67. Phase 3 slice AR-5: added deterministic signature, root mutation,
    keyring verification, rotated-key rejection, and public-key mismatch
    coverage plus `make accountability-receipt-sign-check`.
68. Phase 3 slice AR-7: added a standalone `cortex-receipt-verify` workspace
    crate and binary that does not link `cortex-engine`, `cortex-storage`,
    `cortex-aql`, or `cortex-server`.
69. Phase 3 slice AR-7: implemented independent verifier-side canonical JSON,
    canonical signed-header bytes, root recomputation, Ed25519 signature
    verification, admitted-cell public hash checks, span bounds checks, budget
    consistency checks, and verification-leaf admitted-cell reference checks.
70. Phase 3 slice AR-7: added a signed public verifier fixture plus
    `make accountability-receipt-verify-check` with genuine acceptance,
    signature/budget tamper rejection, and dependency-graph assertions.
71. Phase 3 slice AR-8: added `make accountability-receipt-tamper-check`
    covering budget mutation, admitted access mutation, provenance span shift,
    dropped visible conflict, swapped verification status, replayed
    determinism input, and signature-byte mutation.
72. Phase 3 slice AR-8: added umbrella `make accountability-receipt-check`
    over schema, determinism, sign, standalone verify, tamper, and no-FNV/XOR
    receipt-integrity source checks, then wired it into `alpha-check`.
73. Phase 3 runtime emission: added signed `accountability_receipt.v1` JSON
    emission for `/v1/context` and `/v1/verify` when
    `receipt_signing_key` custody is configured, while preserving absent-field
    compatibility when no key is configured.
74. Phase 3 runtime emission: made configured emission fail closed by routing
    server signing through `ReceiptEmissionContext`; signing/build failures now
    return route errors instead of silently dropping a configured receipt.
75. Phase 3 runtime emission: added evidence-producing ContextPack and VERIFY
    paths so emitted receipts are built from actual retrieval/verification
    evidence rather than response-only JSON.
76. Phase 3 runtime emission contract: updated OpenAPI, generated SDK types,
    manual Python/TypeScript SDK models, receipt/schema/security docs, and
    receipt contract checks for the emitted optional field.
77. Phase 2 slice CRY-5: added `accountability_receipt_hash` to local audit v2
    records and included it in the deterministic `event_hash`/`event_mac`
    surface, without storing the receipt body in the audit log.
78. Phase 2 slice CRY-5: added live `/v1/context` coverage proving the audit
    log hash equals the returned configured-key `accountability_receipt.v1`
    hash, plus CLI verification coverage rejecting tampered receipt hashes.
79. Phase 2 slice CRY-5: added `make audit-receipt-binding-check` with a JSON
    evidence report and wired it into `crypto-foundation-check`.
80. Phase 2 slice CRY-6 re-anchor: extended `cortexdb receipt-key rotate` with
    optional `--reanchor-file`, `--audit-chain-head`, and `--audit-sequence`
    so key rotation can emit a signed `cortexdb.receipt_audit_reanchor.v1`
    record.
81. Phase 2 slice CRY-6 re-anchor: added `cortexdb receipt-key
    verify-reanchor` to verify the re-anchor body hash, trust manifest hash,
    previous-key signature, current-key signature, and audit-chain head shape.
82. Phase 2 slice CRY-6 re-anchor: extended `make key-management-check` and
    public crypto docs so receipt key rotation now has a gated re-anchor
    artifact instead of only a dual-trust manifest.
83. Phase 3 receipt isolation hardening: replaced tenant-derived receipt
    header identity with a durable database-instance identity loaded from
    `cortexdb.database_instance_identity.v1` JSON in the database root.
84. Phase 3 receipt isolation hardening: server startup now creates or reads
    `cortexdb.database_instance_identity.json` when local or external receipt
    signing is configured; invalid or missing prepared identity fails receipt
    emission closed instead of silently dropping configured receipts.
85. Phase 3 receipt isolation hardening: added
    `database-instance-identity-check` with tenant-independence, persistence,
    invalid-file rejection, code-marker, docs-marker, and make-wiring evidence.
86. Phase 5 boundary slice: added a physical skipped-segment parity boundary
    gate so full persisted ANN/lexical segment-pruning parity remains assigned
    to `ann-scope-parity-check` and is not claimed by
    `context-access-decision-capture-check`.
87. Phase 5 FC-2: added `ann-scope-parity-check` for bound bitmap program
    derived persisted ANN/lexical allowed-set parity. The bound-plan search
    entrypoint computes cells from `eval_bitmap_program(plan.bitmap_program)`
    and constrains persisted keyword/vector/hybrid search; no-plan `/v1/search`
    remains scope-only because it has no AQL `WHERE` or `BoundRetrievePlan`.
88. Phase 5 FC-3: added `ann-sparse-scope-recall-check` and
    `SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES` so fallback-enabled ANN
    search with an explicit visit budget routes small sparse-relative-to-graph
    allowed sets to exact top-k before scope-blind HNSW traversal can exhaust
    budget on out-of-scope nodes. The ANN report surfaces
    `fallback_reason=sparse_allowed_set`; dense allowed sets keep the normal
    `visit_budget_exceeded` path, and full per-scope ANN subgraph partitioning
    plus fallback-disabled production eligibility remain explicitly unclaimed.
89. Phase 5 FC-6: added `scope-leak-bench-check` over >=200 combinations of
    agent, query shape, ContextPack export format, persistence state, and
    budget mode. The benchmark plants `PRIVATE_SCOPE_SHOULD_NOT_LEAK` plus
    forbidden source/citation/provenance markers in private-scope cells and
    scans ContextPack payload/citation/source_ref/explain/anomaly surfaces,
    VERIFY evidence/numeric conflicts, verification exports, and safe errors.
    This closes the benchmark surface, not the FC-7 formal invariant model or
    FC-8 aggregate fail-closed release lane.
90. Phase 5 FC-7: added `fail-closed-invariant-model-check` with a
    deterministic property harness over 128 AQL catalog/view/status/type/WHERE
    cases and 32 engine cases. The engine property checks ContextPack AQL plus
    persisted keyword/vector/hybrid search with the bound plan, and
    `accountability::fail_closed_invariant_model_hash()` exports pinned
    `model_hash=cb0e81f8fd07d20769e27ea3f8bd3b4e7459e72504a939b393470d433df40e79`.
    This closes the formal invariant model gate, not the FC-8 aggregate
    fail-closed release lane.
91. Phase 5 FC-8: added `fail-closed-end-to-end-check` as the aggregate
    release blocker over FC-1 through FC-7 plus
    `context-pack-private-scope-check` and `engine-determinism-check`. The gate
    writes `target/fail-closed-end-to-end/report.json`, registers
    `FAIL_CLOSED_END_TO_END_REPORT`, is listed in `.PHONY`, and is wired into
    `scripts/beta_release_bundle.py` with its artifact included in beta
    evidence. This closes physical fail-closed parity, not production-grade
    transparency anchoring, KMS/HSM custody, or compliance immutability.
92. Order 6 reconciliation: current `crypto-foundation-check` proves the
    roadmap Phase 4 crypto-hardening surface is already implemented in this
    checkout. It covers CRY-3 AEAD backup plus legacy-refuse, CRY-4 keyed
    audit chain, CRY-5 audit receipt-hash binding, CRY-6 key management and
    re-anchor coverage, CRY-7 honest crypto claims, database-instance identity,
    and secrets hygiene. This status correction prevents re-running the
    already-green CRY block as the next ordered item.
93. Order 7 DV2: added `verify-numeric-normalization-check` for the
    integer-only numeric normalizer. Cross-unit values normalize through
    u128-backed base-unit conversion, natural unit aliases such as `hours`,
    `minutes`, `meters`, and `grams` parse to canonical units, cross-class
    units remain `Incomparable`, and cross-currency conflicts are labeled as
    `CurrencyMismatch` while still counting as conflicts for VERIFY and
    ContextPack conflict visibility.
94. Order 7 DV1: extended `context-pack-conflict-visibility-check` so the
    ContextPack conflict visibility gate itself proves numeric-normalized
    equality and cross-path agreement with real `VERIFY FACT` numeric
    conflicts. The gate now covers `$1.2M`, `1,200,000 USD`, and `1.2 million`
    as zero visible conflicts, plus a shared-corpus assertion where
    ContextPack `visible_conflict_count` matches `VerificationReport`
    `numeric_conflicts.len()` for both agreement and conflict cases.
95. Order 7 DV3: added `verify-multivalue-extraction-check` and changed the
    numeric fact claim index from one record per cell to deterministic
    multi-record storage. `records_from_payload` now extracts contextual
    currency/unit values from multi-number bodies, keeps explicit multi-value
    `value=` fields indexed, and filters per-record comparability after cell-id
    bucket selection. Live derived stores and conflict-index rebuild/reopen
    paths now use all numeric records for a cell, while the conflict index skips
    same-cell numeric pairs to avoid self-conflicts. The regression fixture
    proves `VERIFY FACT "Solar Plant budget is 1.2B KZT"` detects a conflict
    from evidence text containing both the year `2025` and `1.4B KZT`.
96. Order 7 DV4: added `verify-temporal-conflict-check`, removed the temporal
    early-return from numeric fact claim lookup, and attached `TemporalValidity`
    to each `NumericFactRecord`. Dated `VERIFY FACT` queries now still produce
    structured numeric conflicts when evidence windows overlap the query date,
    stale/non-overlap evidence remains guarded instead of contradictory, and the
    conflict index only flags same `(scope, project, metric)` numeric
    contradictions when validity windows overlap. Full-workspace regressions
    also forced two guarded fixes: explicit `currency=` metadata now applies to
    raw `value=` numerics, and `contradicts=` marker lines are not indexed as
    positive numeric fact claims.
97. Order 7 DV5: added `verify-citation-conflict-check`, added typed
    `VerificationNumericConflictKind`, and surfaced same structured
    `source_ref` value disagreement as `kind="citation"` in VERIFY
    `numeric_conflicts`. `NumericFactRecord` now carries parsed `SourceRef`
    data, same-source comparison requires structured source specificity beyond
    a bare source id, and the conflict index labels same-source numeric
    disagreement as a citation conflict while same-source equal values remain
    non-conflicting. API/CLI/SDK/OpenAPI responses now include additive
    `numeric_conflicts[].kind` with backward-compatible SDK decode defaults.
    The kind surface now distinguishes ordinary numeric conflicts from
    temporal-window conflicts and same-citation conflicts.
98. Order 7 DV6: added `verify-determinism-check`, wired it into
    `engine-determinism-check` and `verification-quality-check`, and proved
    canonical VERIFY conflict serialization is byte-repeatable on a fixed store
    and after checkpoint. The canonical verification hash surface includes
    `numeric_conflicts[].kind`, changes when the conflict kind changes, and the
    gate rejects wall-clock markers such as `elapsed_nanos`,
    `total_elapsed_nanos`, `Instant`, and `SystemTime` inside the hashed
    verification/conflict serializer.
99. Order 7 DV7-DV8: added `verify-conflict-recall-check`,
    `verify-docs-claims-check`, and `docs-claims-check`. The DV7 measured
    corpus runs 180 real `Database::verify_fact_aql` cases: 150 expected
    conflicts across magnitude/unit/currency/temporal/citation/format and 30
    `must_not_conflict` controls. The latest measured report is
    `recall_q16=65535`, `precision_q16=65535`, and
    `false_conflict_rate_q16=0`. The implementation also fixed VERIFY numeric
    candidate selection so unit-class aliases such as `60 min`, `1h`, and
    `2 h` are compared by the existing integer-only normalizer. `VERIFY_FACT`
    now replaces the old alpha limitations section with measured conflict
    coverage, supported classes, honest scope notes, and a docs-claims gate that
    cross-checks the published numbers against the DV7 report.
100. Order 7 RANK-1: added the frozen ranking weights artifact
    `crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json`, generated
    crate-local module `crates/cortex-engine/src/search/frozen_weights.rs`, and
    `ranking-frozen-weights-check` with the compact
    `scripts/ranking_frozen_weights_gate_spec.py` marker spec. Existing Q16
    ranking weights for hybrid RRF, reranker defaults/calibration,
    evidence-overlap scoring, metadata rerank, search route policies,
    query-understanding term/anchor weights, ContextPack redundancy penalty,
    and value-per-token ordering now reference the frozen module instead of
    local bare literals. This is a behavior-identical refactor only; learned
    training, drift gates, and lift claims stay deferred to RANK-2/RANK-3.
101. Order 7 RANK-2: extended
    `scripts/learned_ranking_calibration_check.py` with a deterministic
    `--compiled-artifact` output and added
    `scripts/ranking_weights_drift_check.py` plus
    `make ranking-weights-drift-check`. The trainer-selected profiles now
    update the checked-in frozen artifact for `basic`, `semantic`,
    `project_related`, and `completeness`; the generated artifact is
    byte-identical to `crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json`,
    shares content hash
    `d67c97a93c97d34f501ecdb7da103faec92af38d62925514bfcb764e0d5fe947`,
    and the module check proves the engine-loaded constants match. The offline
    calibration floor remains `heldout_mrr_lift_bps=3750` and
    `heldout_win_rate_pct=75`. Engine-side heldout lift reproduction is not
    done here; that is the next RANK-3 slice.
102. Order 7 RANK-3: added `make ranking-learned-lift-check` and the
    engine-side regression `crates/cortex-engine/tests/ranking_learned_lift.rs`.
    The test loads
    `fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl`, compares
    `WeightedScoreReranker::fixed_default()` against
    `WeightedScoreReranker::enterprise_rag_calibrated()`, and writes
    `target/ranking/learned-lift/report.json`. The final report is
    `status=passed` with heldout baseline MRR `6250`, learned MRR `10000`,
    lift `3750` bps, win-rate `75`, and no policy regressions. The fixture's
    semantic/project/completeness question text now matches the engine's
    text-only intent classifier, while the trainer-controlled `question_type`
    labels, candidate scores, and expected documents remain unchanged.
103. Order 7 REPRO-2: added `crates/cortex-engine/src/determinism_hash.rs`
    and `make weights-version-binding-check`. `AccountabilityDeterminismInput`
    now carries both `frozen_weights_version` and `frozen_weights_hash`, with
    ContextPack/VERIFY receipt evidence using the checked-in
    `crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json` bytes and
    `frozen_weights::VERSION`. The standalone verifier now uses the same
    `cortexdb.determinism_hash.v1` domain and
    `cortexdb.determinism_hash.input.v1` input shape, including
    `frozen_weights.version` and `frozen_weights.artifact_hash`. The binding
    report proves mutating `/calibration/basic/0` changes artifact hash
    `f8678f66c6946c6dc74156cb078cf0762ad820589377ef84925ce3ad97dd6493`
    to
    `e23b1ef81e8c6a8f4affc2836e59315568c3645e53158126223b936ff9b7ebdf`,
    changes `determinism_hash`, and reverting restores the original hash.
104. Order 7 REPRO-3: added `make pack-determinism-hash-check`, the
    cross-process fixture binary
    `crates/cortex-engine/src/bin/pack_determinism_hash_fixture.rs`, and the
    integration test `crates/cortex-engine/tests/pack_determinism_hash.rs`.
    The harness seeds a deterministic database, captures canonical
    ContextPack and VERIFY bytes before checkpoint, then checkpoint/reopens
    and compares the post-reopen bytes plus receipt determinism hashes. It
    runs the fixture in two separate processes and writes
    `target/pack-determinism/report.json`; the report is `status=passed` with
    `same_process_checkpoint_reopen_stable=true`,
    `cross_process_stable=true`, context determinism hash
    `d9a6963d3fccf3ad367e1d7c19056d3db19bf733dc2f283f78be3575442bf001`,
    and VERIFY determinism hash
    `c74fb921a4c0fc8338fbb28ed268e12d91ef53ab7f69f391bfc718f62f631b2c`.
105. Order 7 RANK-4: added `make ranking-explain-faithfulness-check` with a
    dependency on `ranking-frozen-weights-check`, plus the integration test
    `crates/cortex-engine/tests/ranking_explain_faithfulness.rs`. The fixture
    builds a real ContextPack under frozen ranking weights and exercises
    `base_bm25`, `source_trust_bonus`, `source_freshness_bonus`,
    `redundancy_penalty`, and positive/negative `feedback_bonus` components.
    The gate asserts each selected cell's `explain.score` equals the sum of
    `explain.score_components[].contribution`, while also checking component
    values match the dedicated explain fields. The report
    `target/ranking/explain-faithfulness/report.json` is `status=passed`,
    covers two cells, and binds the check to frozen weights artifact hash
    `f8678f66c6946c6dc74156cb078cf0762ad820589377ef84925ce3ad97dd6493`.
106. Order 7 RANK-5: added `make pack-completeness-signal-check` with a real
    dependency on `ann-budget-disclosure-check`, plus the integration test
    `crates/cortex-engine/tests/pack_completeness_signal.rs`. `ContextPack`
    now has an additive optional top-level `grounding_report` field, with
    engine JSON export, canonical serialization, server response mapping,
    OpenAPI, JSON Schema, manual SDK models, generated SDK types, and docs kept
    aligned. The gate proves both completeness signals are part of the hashed
    pack surface: `retrieval_incomplete_hashed=true`,
    `grounding_report_hashed=true`,
    `grounding_changes_canonical_bytes=true`, and
    `grounding_result_changes_canonical_bytes=true` in
    `target/pack-completeness/report.json`. This closes Order 7.
107. Order 8 SPEC-1: published the normative
    `docs/spec/GCE_CONTRACT.md` contract for the Governed Context Engine
    category. The spec defines the `ContextPack` result type, verification
    output, six GCE invariants, conformance obligations, current gate evidence,
    and claim boundaries while keeping the scope to single-node beta and
    avoiding production-grade transparency/KMS/compliance claims. Added
    `make gce-spec-doc-check` and `scripts/gce_spec_doc_check.py`, which cross
    checks required sections, the six invariants, ContextPack fields from
    `context/mod.rs`, VERIFY fields from `verification/types.rs`,
    fail-closed binder markers, receipt/schema markers, and make/phony wiring.
    The report `target/gce-spec/doc-report.json` is `status=passed`.
108. Order 8 SPEC-3: published `docs/spec/RECEIPT_VERIFIER.md` as the
    normative offline verifier algorithm and threat model for
    `accountability_receipt.v1`. The doc enumerates seven verifier steps,
    seven named forgery/tamper classes, defending fields, current gate
    evidence, and explicit out-of-scope boundaries for factual truth,
    equivocation, KMS/HSM custody, and compliance immutability. Added
    `make receipt-threat-model-check` and
    `scripts/receipt_threat_model_check.py`, which cross-check the doc against
    verifier functions, receipt schema fields, AR-8 tamper mutation names,
    existing verifier/tamper/determinism gate names, `GCE_CONTRACT.md`, and
    make/phony wiring. The report
    `target/gce-spec/receipt-threat-model-report.json` is `status=passed`.
109. Order 8 CONF-1: published `docs/spec/GCE_CONFORMANCE.md` plus the
    CI-safe thin-wrapper reference fixture
    `fixtures/gce_conformance/thin_wrapper_reference.json`, and added
    `make aab-conformance-check` via `scripts/aab_conformance_check.py`. The
    gate runs the GCE spec, receipt threat model, standalone receipt verify,
    receipt tamper, pack determinism, fail-closed end-to-end, cosine metric,
    verification quality, and audit receipt binding evidence before checking
    the conformance fixture. The report `target/gce-conformance/report.json`
    is `status=passed`: CortexDB passes all conformance axes, while the
    documented pgvector/policy/RAG thin-wrapper reference fails the required
    `receipt_verifiability`, `determinism`, and `plan_binding` axes. This
    closes Order 8 without moving the four-competitor AAB matrix into the
    release-critical path.
110. Order 9 transparency/equivocation anchor: added local append-only
    transparency log records (`cortexdb.transparency.log.record.v1`) for
    signed receipt emission, chained by record hash and binding `pack_root`,
    `determinism_hash`, `db_instance_id`, key id, timestamp, and receipt
    signature. Server emission is opt-in via
    `CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE` and fails closed if
    append/chain/equivocation checks fail. Added
    `make transparency-anchor-check` via
    `scripts/transparency_anchor_check.py`, with engine tests for append-chain,
    same-determinism/different-pack-root rejection, and tamper detection plus
    docs updates in the GCE contract, verifier threat model, and security
    model. The report `target/transparency-anchor/report.json` is
    `status=passed`. This is intentionally a local anti-equivocation anchor,
    not external witnessed transparency, KMS/HSM custody, or compliance
    immutability.
111. Order 9 SCALE-3 receipt/audit-head slice: added signed
    `audit_chain_head` to `accountability_receipt.v1` headers, schema,
    standalone verifier model/canonical header bytes, golden verifier fixture,
    and receipt docs. Server receipt emission now reads the persisted audit
    JSONL chain tail before signing when audit logging has a file path, or
    binds the audit-chain zero hash when no persisted audit log is configured.
    Added engine tests proving same committed receipt inputs produce
    byte-identical signed headers and that changing the audit-chain head
    changes the signature. Added `make receipt-replica-invariance-check` via
    `scripts/receipt_replica_invariance_check.py`, which aggregates schema,
    verifier, and transparency gates and verifies standalone rejection of
    `audit_chain_head` tampering. The report
    `target/receipt-replica-invariance/report.json` is `status=passed`. This
    is a CI-safe SCALE-3 receipt/audit-head slice, not yet a live multi-node
    Raft/follower/failover proof.
112. Order 9 SCALE-3 replicated snapshot receipt invariance: added
    `crates/cortex-engine/tests/receipt_replica_invariance.rs`, which opens
    leader and follower `Database` instances, writes committed cells on the
    leader, installs `replication_snapshot_segment()` into the follower, and
    runs the same AQL ContextPack+receipt evidence query on both replicas. The
    test asserts byte-identical canonical ContextPack bytes, identical
    `determinism_hash`, byte-identical canonical signed
    `accountability_receipt.v1` bytes for the same receipt inputs, signed
    `audit_chain_head`, and matching local transparency-log commitments. The
    `receipt-replica-invariance-check` target and report script now run and
    document this evidence. This closes the local replicated-snapshot
    invariance slice, not networked Raft arbitrary-node serving,
    follower-read/failover/partition-heal behavior, external witnessed
    transparency, or KMS/HSM custody.
113. Order 9 SCALE-1 cluster fail-closed binder slice: added
    `crates/cortex-engine/tests/cluster_fail_closed.rs` plus
    `crates/cortex-engine/tests/cluster_fail_closed/support.rs`, covering
    follower reads after TCP peer snapshot install, leader failover where a
    stale old-leader scope-widening entry is rejected at the higher term, and
    partition/partition-heal where minority writes are not served before
    commit and post-heal reads still exclude private scope. Each scenario
    asserts the binder seed contains `PushAgentAllowed`, `PushLive`, and `And`;
    emitted ContextPack cells have captured `allowed` access decisions; and
    committed follower/failover reads preserve canonical ContextPack bytes,
    `determinism_hash`, and signed receipt bytes. Added
    `make consensus-failover-binder-check` with
    `scripts/consensus_failover_binder_check.py`; report
    `target/consensus/failover-binder.json` is `status=passed`. This is a
    CI-safe consensus/DB harness gate, not full HTTP/Raft arbitrary-node
    request routing, SCALE-2 consistency, external witnessed transparency,
    KMS/HSM custody, or N-run soak promotion.
114. Order 9 SCALE-2 multi-agent cluster consistency slice: added
    `crates/cortex-engine/tests/multi_agent_cluster_consistency.rs` plus
    `crates/cortex-engine/tests/multi_agent_cluster_consistency/support.rs`.
    The new cluster test commits agent transactions on a leader, verifies
    read-your-writes from a follower after TCP peer snapshot install, promotes
    that follower and commits a later write, then verifies the promoted state
    on another follower. A partition-heal scenario records a `last_seen_seq`,
    proves a stale follower rejects a future `SharedSequenced` handoff before
    catch-up, heals the partition, installs the caught-up snapshot, and proves
    monotonic reads advance to the later sequence with `SharedImmediate` and
    `SharedSequenced` reports. Added
    `make multi-agent-cluster-consistency-check`, which runs the existing
    single-node `multi-agent-consistency-check`, the cluster regression, and
    `scripts/multi_agent_cluster_consistency_check.py`; report
    `target/multi-agent-cluster-consistency/report.json` is `status=passed`.
    This is a CI-safe consensus/DB harness gate, not full HTTP/Raft
    arbitrary-node request routing, external witnessed transparency, KMS/HSM
    custody, linearizable reads without explicit catch-up, or N-run soak
    promotion.
115. Order 9 HTTP/Raft arbitrary-node routing evidence slice: added
    `crates/cortex-server/src/tests/http_raft_routing_tests.rs`. The new
    server regression builds production `AppState`/Axum route handling for a
    leader root and two follower roots, installs follower state through
    authenticated TCP Raft snapshot-install frames, and calls `/v1/context`
    against each node root. It proves the HTTP route serves the same committed
    cells, same `pack_root`, same `determinism_hash`, same `audit_chain_head`,
    and same stable receipt commitments from any replicated node root while
    preserving fail-closed filtering for a replicated private cell. Added
    `make http-raft-routing-accountability-check`, which runs the targeted
    server regression and
    `scripts/http_raft_routing_accountability_check.py`; report
    `target/http-raft-routing-accountability/report.json` is `status=passed`.
    This is a CI-safe HTTP/Axum plus TCP Raft snapshot-install gate, not a
    live production ingress/load-balancer proof, not byte-identical HTTP
    receipt signatures across wall-clock signing times, and not external
    witnessed transparency, KMS/HSM custody, or N-run soak promotion.
116. Order 9 live Raft ingress production guard slice: added
    `crates/cortex-server/src/cluster.rs` and
    `crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs`. The
    server can now carry an explicit `ClusterConfig` through `ServerOptions`
    (loaded by production `cortex-server` from
    `CORTEXDB_CLUSTER_CONFIG_FILE`) and expose that topology through
    `/v1/cluster/status` instead of silently reporting only single-node state.
    For configured multi-node topologies, both legacy sync tests and the
    production Axum handler now fail closed on `/v1/context` and
    `/v1/context/trace` with `service_unavailable` when a non-primary node
    cannot reach the configured primary. Added
    `make raft-ingress-production-guard-check`,
    which runs the focused server regression and
    `scripts/raft_ingress_production_guard_check.py`; report
    `target/raft-ingress-production-guard/report.json` is `status=passed`.
    This is an honest production guard and status-surface gate, not a
    production load balancer, not leader discovery, not linearizable multi-node
    reads through ingress, and not external witnessed transparency, KMS/HSM
    custody, or N-run soak promotion.
117. Order 9 fixed-primary live ingress forwarding slice: replaced the
    blanket multi-node accountable-context guard with typed ingress decisions
    in `crates/cortex-server/src/cluster.rs`. The first configured cluster node
    is now treated as the fixed primary for `/v1/context` and
    `/v1/context/trace`: that node serves locally, while non-primary nodes
    forward the live HTTP request body and selected headers to the primary and
    return the primary JSON body/status. The production Axum path performs the
    blocking TCP forward on `spawn_blocking`, and the legacy sync harness uses
    the same forward helper. Added
    `non_primary_context_route_forwards_to_live_primary`, which starts two
    real `serve_with_options` loopback servers, seeds the primary, calls
    `/v1/context` through the follower, and verifies the forwarded response
    contains the primary cell and signed `accountability_receipt`. Added
    `make raft-ingress-forwarding-check` and
    `scripts/raft_ingress_forwarding_check.py`; report
    `target/raft-ingress-forwarding/report.json` is `status=passed`. This
    proves fixed-primary live HTTP forwarding, not automatic Raft leader
    discovery, not load-balancer routing, not linearizable arbitrary-node reads
    through leadership change, and not external witnessed transparency,
    KMS/HSM custody, or N-run soak promotion.
118. Order 9 operator leader-hint ingress slice: added
    `ServerOptions.cluster_ingress_leader` and production env parsing for
    `CORTEXDB_CLUSTER_INGRESS_LEADER_ID`. Context ingress now chooses the
    operator-provided leader id when present and otherwise falls back to the
    first configured node. Startup validation rejects a leader hint without
    `cluster_config` or a hint that is not present in the configured topology;
    request routing also fails closed on unknown hinted leaders instead of
    falling back to local or first-node reads. Added
    `crates/cortex-server/src/tests/cluster_ingress_leader_hint_tests.rs`,
    where node 1 forwards `/v1/context` to a live hinted leader on node 2 and
    verifies the returned primary cell plus signed receipt, and where an
    unknown hinted leader returns `503`. Added
    `make raft-ingress-leader-hint-check` and
    `scripts/raft_ingress_leader_hint_check.py`; report
    `target/raft-ingress-leader-hint/report.json` is `status=passed`. This
    proves operator-controlled leader-target forwarding, not automatic Raft
    leader discovery from durable election state, not health-aware load
    balancing, not linearizable arbitrary-node reads through leadership change,
    and not external witnessed transparency, KMS/HSM custody, or N-run soak
    promotion.
119. Order 9 automatic Raft STATUS ingress discovery slice: extended
    `ClusterNode` with optional `ingress_address` while preserving legacy
    two-column cluster config records, and added a read-only Raft `STATUS`
    frame that reports current term, local node, role, and known leader. For
    separate Raft/HTTP topologies, context ingress now queries configured Raft
    peer addresses, discovers a known leader, and forwards `/v1/context` or
    `/v1/context/trace` to that leader's HTTP ingress address; if no known
    leader is discovered, it fails closed with `503` instead of serving local
    data. Added
    `crates/cortex-server/src/tests/cluster_ingress_discovery_tests.rs`,
    where node 1 discovers node 2 through a live Raft STATUS peer, forwards to
    node 2 HTTP ingress, and verifies the returned cell plus signed receipt;
    the companion regression covers separate-ingress fail-closed discovery
    unavailability. Added `make raft-ingress-auto-discovery-check` and
    `scripts/raft_ingress_auto_discovery_check.py`; report
    `target/raft-ingress-auto-discovery/report.json` is `status=passed`. This
    proves one-request Raft STATUS based leader discovery in an explicit
    separate-ingress topology, not health-aware/load-aware balancing, not a
    long-running production cluster-status monitor, not linearizable
    arbitrary-node reads through leadership changes and partitions, and not
    external witnessed transparency, KMS/HSM custody, or N-run soak promotion.
120. Order 9 health-aware ingress routing slice: automatic Raft leader
    discovery now probes a discovered remote leader's `/v1/health` endpoint
    before returning a forwarding target. If a Raft peer reports a stale leader
    whose HTTP ingress is unavailable, discovery continues to the next Raft
    peer instead of failing later in request forwarding. Added
    `crates/cortex-server/src/tests/cluster_ingress_health_tests.rs`, where
    node 1 first sees stale Raft status for node 2, node 2 HTTP ingress is
    down, node 2 Raft status reports node 1 as the current leader, and node 1
    serves the accountable context path locally with the expected signed
    receipt. Added `make raft-ingress-health-routing-check` and
    `scripts/raft_ingress_health_routing_check.py`; report
    `target/raft-ingress-health-routing/report.json` is `status=passed`.
    This proves health-aware skip-through for an unhealthy stale leader target
    and a simulated leadership-change view across Raft peers, not load-aware
    distribution, not a long-running production lifecycle monitor with cached
    peer health, not partition-linearizable arbitrary-node reads, and not
    external witnessed transparency, KMS/HSM custody, or N-run soak promotion.
121. Order 9 cached lifecycle monitor slice: split the ingress monitor into
    `crates/cortex-server/src/cluster/monitor.rs`, added
    `ClusterIngressMonitor` with a cached `ClusterIngressSnapshot`, wired it
    into `AppState`, and made production `serve_with_options` perform an
    initial refresh plus a background refresh loop. The Axum handler now calls
    `context_ingress_decision_with_monitor` and uses cached leader state when
    a monitor exists; the legacy sync harness keeps the direct request-time
    discovery path for compatibility. Added regression
    `production_monitor_uses_cached_leader_after_status_peer_exits`, which
    starts a one-shot Raft STATUS peer, proves the pre-monitor behavior fails
    once that peer exits, and now proves production routing can still forward
    to the cached healthy leader. Added
    `make raft-ingress-lifecycle-monitor-check` and
    `scripts/raft_ingress_lifecycle_monitor_check.py`; report
    `target/raft-ingress-lifecycle-monitor/report.json` is `status=passed`.
    This proves startup/background cached monitor wiring and last-known
    healthy leader use through the Axum path, not load-aware distribution,
    not exported monitor metrics or tunable intervals, not
    partition-linearizable arbitrary-node reads, and not external witnessed
    transparency, KMS/HSM custody, or N-run soak promotion.
122. Order 9 load-aware ingress policy slice: added per-leader in-flight
    admission to `ClusterIngressMonitor`, with forwarding decisions holding a
    `ClusterIngressRoutePermit` until the selected route is dropped. Added
    regression
    `load_policy_rejects_second_route_until_first_permit_drops`, which sets a
    one-route cached-monitor limit, proves the first non-leader route acquires
    a forwarding permit, proves a second concurrent route fails closed with an
    over-load-limit message, then proves routing resumes after the first
    permit is dropped. Added `make raft-ingress-load-policy-check` and
    `scripts/raft_ingress_load_policy_check.py`; report
    `target/raft-ingress-load-policy/report.json` is `status=passed`. This
    proves load-aware admission for cached forwarded ingress to the current
    leader, not load balancing across multiple writable Raft leaders, not
    operator-tunable production limits or exported load metrics, not
    partition-linearizable arbitrary-node reads, and not external witnessed
    transparency, KMS/HSM custody, or N-run soak promotion.
123. Order 9 external transparency witness mirror slice: added
    `crates/cortex-engine/src/accountability/transparency_witness.rs` with
    `TransparencyWitnessRecord`, `TransparencyWitnessSigningKey`,
    `witness_transparency_log`, and `verify_transparency_witness_record`. The
    witness record signs the already verified local transparency log head,
    sequence range, first/last receipt identities, and witness metadata with a
    separate Ed25519 key under
    `cortexdb.transparency.witness.record.v1`. Added regressions
    `transparency_witness_signs_log_head_and_verifies` and
    `transparency_witness_detects_tampered_head_after_hash_recompute`, proving
    both a valid off-database mirror witness and rejection after body tampering
    even when the witness hash is recomputed. Added
    `make transparency-witness-check` and
    `scripts/transparency_witness_check.py`; report
    `target/transparency-witness/report.json` is `status=passed`. This proves
    CI-safe external mirror evidence for the local `pack_root` chain head, not
    public transparency service availability, CT-style gossip, KMS/HSM
    custody, compliance immutability, or release-lane soak stability.
124. Order 9 transparency witness quorum slice: added
    `crates/cortex-engine/src/accountability/transparency_quorum.rs` with
    `TransparencyWitnessQuorumEvidence`,
    `verify_transparency_witness_quorum`, and
    `transparency_witness_quorum_hash`. The quorum evidence requires multiple
    individually valid witness records to agree on the same verified local log
    head, sequence range, and first/last receipt identities, while rejecting
    duplicate witness ids, duplicate key ids, duplicate public keys, and split
    log heads. Added regressions
    `transparency_witness_quorum_accepts_independent_log_head_witnesses`,
    `transparency_witness_quorum_rejects_duplicate_public_key`, and
    `transparency_witness_quorum_rejects_split_log_heads`. Added
    `make transparency-witness-quorum-check` and
    `scripts/transparency_witness_quorum_check.py`; report
    `target/transparency-witness-quorum/report.json` is `status=passed`.
    This proves CI-safe independent witness quorum evidence for the local
    `pack_root` chain head, not public transparency service availability,
    CT-style gossip, KMS/HSM custody, compliance immutability, or release-lane
    soak stability.
125. Order 9 transparency inclusion proof slice: added
    `crates/cortex-engine/src/accountability/transparency_inclusion.rs` with
    `TransparencyInclusionProof`, `TransparencyInclusionSibling`,
    `build_transparency_inclusion_proof`,
    `verify_transparency_inclusion_proof`, and
    `transparency_inclusion_root_hash`. The proof binds a local transparency
    record's `(sequence, record_hash)` leaf to a Merkle root for the published
    log snapshot and carries `log_record_count` plus `log_head_hash` for the
    snapshot boundary. Added regressions
    `transparency_inclusion_proof_accepts_middle_record`,
    `transparency_inclusion_proof_rejects_wrong_record_hash`, and
    `transparency_inclusion_proof_rejects_wrong_path_hash`. Added
    `make transparency-inclusion-check` and
    `scripts/transparency_inclusion_check.py`; report
    `target/transparency-inclusion/report.json` is `status=passed`. This
    proves a CI-safe Merkle inclusion proof primitive, not public
    transparency service availability, CT-style gossip/consistency exchange,
    KMS/HSM custody, compliance immutability, or release-lane soak stability.
126. Order 9 transparency consistency evidence slice: added
    `crates/cortex-engine/src/accountability/transparency_consistency.rs` with
    `TransparencyConsistencyEvidence`,
    `build_transparency_consistency_evidence`, and
    `verify_transparency_consistency_evidence`. The evidence carries old/new
    published snapshot record hashes, log heads, Merkle roots, and a monitor id,
    then verifies exact prefix consistency so append-only snapshots pass while
    divergent prefixes and truncated newer snapshots fail. Added regressions
    `transparency_consistency_accepts_append_only_snapshot`,
    `transparency_consistency_rejects_divergent_prefix`, and
    `transparency_consistency_rejects_truncated_snapshot`. Added
    `make transparency-consistency-check` and
    `scripts/transparency_consistency_check.py`; report
    `target/transparency-consistency/report.json` is `status=passed`. This
    proves a CI-safe public-monitor consistency primitive, not public
    transparency service availability, network gossip fanout, independent
    monitor uptime, KMS/HSM custody, compliance immutability, or release-lane
    soak stability.
127. Order 9 transparency availability evidence slice: added
    `crates/cortex-engine/src/accountability/transparency_availability.rs`
    with `TransparencyAvailabilityPolicy`,
    `TransparencyAvailabilityObservation`,
    `TransparencyAvailabilityEvidence`,
    `build_transparency_availability_evidence`, and
    `verify_transparency_availability_evidence`. The evidence requires fresh
    HTTPS monitor observations from independent monitor ids and URLs, available
    2xx service status, minimum monitor uptime, policy freshness, and agreement
    on the published log count, log head hash, and Merkle root. Added
    regressions `transparency_availability_accepts_fresh_independent_monitors`,
    `transparency_availability_rejects_stale_observation`,
    `transparency_availability_rejects_duplicate_monitor_identity`,
    `transparency_availability_rejects_low_monitor_uptime`, and
    `transparency_availability_rejects_split_log_heads`. Added
    `make transparency-availability-check` and
    `scripts/transparency_availability_check.py`; report
    `target/transparency-availability/report.json` is `status=passed`. This
    proves CI-safe public transparency service availability and independent
    monitor uptime evidence, not network gossip fanout, KMS/HSM custody,
    compliance immutability, or release-lane soak stability.
128. Order 9 transparency gossip fanout slice: added
    `crates/cortex-engine/src/accountability/transparency_gossip.rs` and
    `crates/cortex-engine/src/accountability/transparency_gossip/types.rs`
    with `TransparencyGossipPolicy`, `TransparencyGossipExchange`,
    `TransparencyGossipEvidence`, `build_transparency_gossip_evidence`, and
    `verify_transparency_gossip_evidence`. The evidence requires fresh
    delivered HTTPS monitor-to-monitor exchanges, a minimum per-monitor fanout,
    and agreement on the published log count, log head hash, and Merkle root.
    Added regressions `transparency_gossip_accepts_required_monitor_fanout`,
    `transparency_gossip_rejects_insufficient_fanout`,
    `transparency_gossip_rejects_stale_exchange`, and
    `transparency_gossip_rejects_split_log_head`. Added
    `make transparency-gossip-check` and
    `scripts/transparency_gossip_check.py`; report
    `target/transparency-gossip/report.json` is `status=passed`.
    This proves CI-safe network gossip fanout evidence, not continuous
    production SLO compliance, Byzantine monitor key custody, KMS/HSM custody,
    compliance immutability, or release-lane soak stability.
129. Order 9 transparency SLO evidence slice: added
    `crates/cortex-engine/src/accountability/transparency_slo.rs`,
    `crates/cortex-engine/src/accountability/transparency_slo/types.rs`, and
    `crates/cortex-engine/src/accountability/transparency_slo/validation.rs`
    with `TransparencySloPolicy`, `TransparencySloWindow`,
    `TransparencySloEvidence`, `build_transparency_slo_evidence`, and
    `verify_transparency_slo_evidence`. The evidence requires ordered
    operations windows that cover the declared period without gaps, meet the
    declared availability percentage, carry monitor quorum and gossip fanout
    summaries, report append-only consistency status, and keep log counts
    monotonic. Added regressions
    `transparency_slo_accepts_continuous_operational_windows`,
    `transparency_slo_rejects_gap_between_windows`,
    `transparency_slo_rejects_below_availability_slo`,
    `transparency_slo_rejects_log_count_regression`, and
    `transparency_slo_rejects_split_root_for_same_log_count`. Added
    `make transparency-slo-check` and `scripts/transparency_slo_check.py`;
    report `target/transparency-slo/report.json` is `status=passed`. This
    proves CI-safe continuous public transparency operations/SLO evidence, not
    live production deployment, Byzantine monitor key custody, KMS/HSM custody,
    compliance immutability, or release-lane soak stability.
130. Order 9 ingress load metrics/tunable limit slice: added
    `DEFAULT_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE`, server option and
    `CORTEXDB_CLUSTER_INGRESS_MAX_IN_FLIGHT_PER_NODE` parsing for the cached
    monitor per-leader in-flight limit. `ClusterIngressMonitor::from_options`
    now uses the operator-tunable cached ingress load limit instead of a
    hard-coded value, and `ClusterIngressMonitor::load_metrics` exposes cached
    leader id, configured limit, current in-flight routes, and remaining
    permits. `crates/cortex-server/src/http_metrics/cluster_ingress.rs` maps
    the monitor snapshot into Prometheus gauge values. Prometheus output now
    includes `cortexdb_cluster_ingress_configured`,
    `cortexdb_cluster_ingress_cached_leader_id`,
    `cortexdb_cluster_ingress_max_in_flight_per_node`,
    `cortexdb_cluster_ingress_in_flight`, and
    `cortexdb_cluster_ingress_available_permits`. Added regressions
    `load_policy_uses_operator_configured_limit_from_options`,
    `load_policy_metrics_report_cached_leader_limit_and_in_flight`,
    `parse_cluster_ingress_max_in_flight_accepts_positive_integer`,
    `parse_cluster_ingress_max_in_flight_rejects_zero_and_invalid_values`, and
    Prometheus contract coverage. Added `make raft-ingress-load-metrics-check`
    and `scripts/raft_ingress_load_metrics_check.py`; report
    `target/raft-ingress-load-metrics/report.json` is `status=passed`. This
    proves operator-tunable fail-closed cached ingress admission and exported
    monitor load gauges, not adaptive scheduling across multiple writable
    leaders, release-lane soak stability, partition-linearizable arbitrary-node
    reads, or production receipt guarantees.
131. Order 9 consensus release-lane promotion slice: added
    `make consensus-release-lane-check` and
    `scripts/consensus_release_lane_check.py`. The gate runs N consecutive
    soak-green iterations, defaulting to `CONSENSUS_RELEASE_LANE_RUNS=3`, and
    each iteration validates partition soak, failover SLO, rejoin, SCALE-1
    fail-closed binder, SCALE-2 multi-agent cluster consistency, and SCALE-3
    receipt replica invariance. `release-check` now invokes
    `consensus-release-lane-check` after `replication-lifecycle-check`, so the
    promoted consensus lane is exercised by the release target instead of only
    `distributed-consensus-research-check`. `docs/STATUS.md`,
    `docs/COMMUNITY_ROADMAP.md`, and `docs/SECURITY_MODEL.md` now describe
    this as release-lane CI evidence, not a live production HA guarantee. The
    report `target/consensus/release-lane/report.json` is `status=passed`
    with 3 of 3 consecutive runs green. This closes SCALE-4 release-lane
    wiring without claiming production distributed consensus correctness,
    operator lifecycle SLOs from live multi-process deployments, KMS/HSM
    custody, compliance immutability, or adaptive multi-leader scheduling.
132. CP-3 migration-boundary closure: documented the current agent-scoped
    persisted cell-id layout as
    `agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence` in
    `crates/cortex-engine/src/cell_ids.rs` and `docs/spec/GCE_CONTRACT.md`.
    Extended `scripts/cell_id_collision_check.py` so
    `make cell-id-collision-check` now proves the documented v1 domain is
    28-bit agent slot plus 32-bit sequence, rejects over-width session,
    feedback, and memory/remember agent ids instead of truncating them, and
    records `requires_schema_migration_for_31_bit_slots=true` in
    `target/cell-id-collision/report.json`. This closes the conditional
    31-bit remaining item without changing the persisted encoding: a true
    31-bit session/feedback slot remains future work only behind a new
    persisted schema version with explicit migration or refuse-to-read guard.
133. Production-grade public receipt readiness slice: added
    `make receipt-production-readiness-check` and
    `scripts/receipt_production_readiness_check.py`. The gate runs
    `accountability-receipt-check`, `transparency-slo-check`,
    `key-management-check`, and `security-release-report-check`, then writes
    `target/receipt-production-readiness/report.json` with component status,
    readiness booleans, and blocker ids. The report is intentionally
    `production_ready=false` while KMS/HSM-backed receipt key custody and
    compliance certification/immutability gates are absent. `release-check`
    now invokes this readiness inventory after `consensus-release-lane-check`,
    so release evidence preserves the no-production-grade-public-receipt claim
    instead of leaving it as an untested note.
134. Receipt KMS/HSM custody boundary slice: added
    `make receipt-kms-hsm-custody-check` and
    `scripts/receipt_kms_hsm_custody_check.py`. The gate validates the
    external signer/KMS-HSM custody contract in
    `docs/spec/ACCOUNTABILITY_RECEIPT_V1.md`, records the current runtime as
    local seed backed, and writes
    `target/receipt-kms-hsm-custody/report.json` with
    `kms_hsm_custody=false`,
    `custody_mode=external_signer_contract_only`, and blockers
    `runtime_external_receipt_signer_not_implemented` plus
    `operator_kms_hsm_custody_evidence_not_implemented`. The
    production-readiness aggregator now consumes this report as its own
    component instead of inferring KMS/HSM custody from the local
    key-management report.
135. Receipt external signer runtime slice: added engine-level
    `AccountabilityReceiptHeaderSigner` and
    `sign_accountability_receipt_header_with_signer`, plus server-side
    `ReceiptExternalSigner` command execution. Server receipt emission now
    routes through `ReceiptSigner::Local` or `ReceiptSigner::External`; external
    signer mode uses
    `CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND`,
    `CORTEXDB_RECEIPT_EXTERNAL_SIGNER_KEY_ID`,
    `CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX`, and optional
    `CORTEXDB_RECEIPT_EXTERNAL_SIGNER_REF`. The server sends
    `cortexdb.receipt_external_sign_request.v1`, verifies the returned
    `cortexdb.receipt_external_signature.v1` signature against the configured
    public key, and fails closed without local-seed fallback. The KMS/HSM
    custody gate now reports `external_signer_runtime_supported=true` while
    keeping `kms_hsm_custody=false` because operator KMS/HSM custody evidence
    remains absent.
136. Receipt KMS/HSM custody evidence validator slice: added
    `scripts/receipt_kms_hsm_evidence.py` plus optional
    `RECEIPT_KMS_HSM_CUSTODY_EVIDENCE`,
    `RECEIPT_KMS_HSM_EXPECTED_KEY_ID`,
    `RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX`, and
    `RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF` make wiring. The custody gate can now
    validate `cortexdb.receipt_kms_hsm_custody_evidence.v1` evidence that binds
    provider key reference, signer reference, key id, public key, signing
    domain, non-exportable key policy, disabled local-seed fallback, operator
    controls, and hashed custody artifacts. Default runs still report
    `kms_hsm_custody=false` until real operator evidence is supplied.
137. Compliance certification/immutability evidence validator slice: added
    `scripts/compliance_certification_evidence.py` plus optional
    `COMPLIANCE_CERTIFICATION_EVIDENCE` and
    `COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK` make wiring. The
    compliance-boundary gate can now validate
    `cortexdb.compliance_certification_evidence.v1` evidence that binds an
    external reviewer, framework, report reference, CortexDB
    `accountability_receipt.v1` scope, reviewed controls, operator
    responsibilities, external immutable store, append-only export, retention
    policy, tamper-evidence reference, and hashed artifacts. Default runs still
    report `supported_certified_frameworks=[]`,
    `external_certification.valid=false`, and `compliance_immutability=false`
    until real operator evidence is supplied.
138. Order 9 adaptive ingress scheduling slice: added
    `ClusterIngressMonitor::try_acquire_adaptive_leader_node` and routed cached
    monitor context ingress through it. When the cached leader is over the
    per-node in-flight limit, the monitor performs one Raft status refresh and
    retries admission against the refreshed healthy leader. Added regression
    `adaptive_ingress_refreshes_leader_when_cached_route_is_over_limit`, which
    keeps a permit live on remote node 2, changes the simulated Raft leader
    report to local node 3, and proves the second route uses node 3 instead of
    failing on the saturated cached leader. Added
    `make raft-ingress-adaptive-scheduling-check` and
    `scripts/raft_ingress_adaptive_scheduling_check.py`; report
    `target/raft-ingress-adaptive-scheduling/report.json` is `status=passed`.
    This proves adaptive refresh-on-overload for the cached Raft leader path,
    not load balancing across multiple writable Raft leaders, weighted
    scheduling, partition-linearizable arbitrary-node reads, or production HA.
139. Production-grade public receipt strict gate slice: added
    `--require-production-ready` to
    `scripts/receipt_production_readiness_check.py` and
    `make receipt-production-ready-check` with
    `RECEIPT_PRODUCTION_READY_REPORT`. The existing
    `receipt-production-readiness-check` remains a report-only inventory that
    can pass while `production_ready=false`; the strict gate writes a separate
    report and fails if KMS/HSM custody or compliance certification blockers
    remain. This proves the production receipt claim can be enforced by
    CI/operator automation, not that real operator KMS/HSM custody or external
    compliance certification evidence has been supplied.
140. Production evidence origin guard slice: added synthetic evidence origin
    classification to KMS/HSM custody and compliance certification evidence
    validators, and changed `receipt-production-readiness-check` so
    `production_ready=true` requires operator-origin evidence. Schema-valid
    fixture files remain useful validator coverage, but they now leave the
    production KMS/HSM and compliance blockers in place and make
    `receipt-production-ready-check` fail.
141. Production operator evidence preflight slice: added
    `receipt-production-evidence-preflight-check` as a fail-fast helper before
    the full readiness chain. It requires `RECEIPT_KMS_HSM_CUSTODY_EVIDENCE`,
    `RECEIPT_KMS_HSM_EXPECTED_KEY_ID`,
    `RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX`,
    `RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF`,
    `COMPLIANCE_CERTIFICATION_EVIDENCE`, and
    `COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK`; it rejects missing inputs,
    invalid evidence, and schema-valid synthetic fixtures. This records exactly
    what operator evidence is still needed without treating the preflight as a
    replacement for `receipt-production-ready-check`.
142. Production evidence generated-artifact origin guard slice: extended
    evidence-origin classification so generated local artifacts under `target/`
    are not operator-origin evidence. This prevents parser-positive temporary
    JSON from `target/codex-verification` from clearing production blockers;
    operator evidence must come from an operator-managed evidence location
    outside generated build/test output.
    Component `kms_hsm_custody` and `production_safe` booleans also remain
    false for schema-valid non-operator evidence.
143. Production evidence temporary-local origin guard slice: extended
    evidence-origin classification so `/tmp`, `/var/tmp`, and `/dev/shm`
    evidence paths are `temporary_local_artifact`, not operator-origin
    evidence. This prevents parser-positive scratch files from clearing
    production blockers while still leaving real operator-managed evidence
    paths available.
144. Production evidence local-reference origin guard slice: extended
    evidence-origin classification so nested evidence references pointing to
    local/generated artifacts (`file://`, `fixtures/`, `target/`, `/tmp`,
    `/var/tmp`, or `/dev/shm`) are `local_reference_artifact`, not
    operator-origin evidence. This prevents a schema-valid external JSON file
    from clearing production blockers while its referenced artifacts are still
    local test/build outputs.
145. Production evidence local path-variant reference guard slice: strengthened
    nested-reference classification so path-like variants such as `./target`,
    `../target`, absolute `.../target/...`, and equivalent `fixtures` path
    segments are rejected as `local_reference_artifact`, while non-file URI
    schemes such as `s3://`, `https://`, and `arn:` remain eligible for parser
    validation.
146. Production evidence component-origin guard slice: tightened
    `receipt-kms-hsm-custody-check` and `compliance-boundary-check` so
    schema-valid synthetic, fixture, generated, local, or local-reference
    evidence remains useful validator coverage but cannot set component-level
    production booleans. KMS/HSM component reports now require valid
    operator-origin evidence before `kms_hsm_custody=true` or
    `production_safe=true`; compliance component reports now require valid
    operator-origin evidence before publishing `supported_certified_frameworks`
    or `compliance_immutability=true`.
147. Production evidence loopback-reference guard slice: extended nested
    reference classification so local service references such as `localhost`,
    `127.*`, `0.0.0.0`, `[::1]`, and schemeless host references are rejected as
    `local_reference_artifact`, while non-local URI schemes such as `s3://`,
    `https://`, and `arn:` remain eligible for parser validation.
148. Production evidence resolved-path guard slice: evidence origin now checks
    both the supplied path and the resolved filesystem path, so an
    external-looking symlink back into `target/`, `fixtures/`, `/tmp`,
    `/var/tmp`, or `/dev/shm` remains non-operator evidence. This prevents
    generated local parser artifacts from clearing KMS/HSM or compliance
    blockers through symlink indirection.
149. Production evidence file-scheme reference guard slice: nested reference
    classification now treats all `file:` URI variants, including `file:/...`,
    `file:target/...`, and percent-encoded file paths, as local/generated
    evidence references. This prevents local files from clearing production
    evidence blockers through alternate file URI syntax.
150. Production evidence loopback-equivalent reference guard slice: nested
    reference classification now uses IP address parsing for loopback and
    unspecified hosts, so expanded IPv6 loopback, IPv4-mapped IPv6 loopback,
    `[::]`, and `http://0/...` refs are local/generated evidence references.
    This prevents local listener aliases from clearing production evidence
    blockers through alternate address syntax.
151. Production evidence recursive-encoded reference guard slice: nested
    reference classification now repeatedly decodes percent-encoding and
    normalizes encoded path separators before origin checks. This prevents
    double-encoded `file:`, `target/`, `/tmp`, and loopback refs from clearing
    production evidence blockers.
152. Production evidence legacy IPv4 alias guard slice: nested reference
    classification now rejects legacy IPv4 loopback and unspecified aliases in
    decimal dword, hexadecimal dword, octal dword, dotted hexadecimal, dotted
    octal, and short dotted forms. This prevents local listeners from clearing
    production evidence blockers through non-canonical IPv4 host syntax.
153. Production evidence Windows absolute-path guard slice: nested reference
    classification now rejects Windows drive absolute paths after backslash
    normalization and repeated percent-decoding. This prevents local filesystem
    refs such as `C:\...`, `D:/...`, or encoded drive paths from clearing
    production evidence blockers.
154. Production evidence UNC/scheme-relative path guard slice: nested reference
    classification now rejects UNC and scheme-relative path refs after
    backslash normalization and repeated percent-decoding. This prevents
    filesystem/network path refs such as `\\server\share`, `//server/share`,
    or encoded variants from clearing production evidence blockers.
155. Production evidence local transport URI guard slice: nested reference
    classification now rejects local transport URI schemes before the generic
    non-file URI parser allows remote schemes. This prevents `unix:`, `npipe:`,
    `pipe:`, and encoded variants from clearing production evidence blockers.
156. Production evidence shell-local expansion guard slice: nested reference
    classification now rejects shell/user-local expansion refs after repeated
    percent-decoding. This prevents `~/...`, `~user/...`, `$HOME/...`,
    `${USERPROFILE}/...`, `%USERPROFILE%/...`, `%TEMP%/...`, and encoded
    variants from clearing production evidence blockers.
157. Production evidence generic path reference guard slice: nested reference
    classification now rejects relative and absolute POSIX-style filesystem
    refs after repeated percent-decoding. This prevents
    `operator-evidence/report.pdf`, `./operator-evidence/report.pdf`,
    `../operator-evidence/report.pdf`, `/home/operator/evidence/report.pdf`,
    and encoded variants from clearing production evidence blockers.
158. Production evidence origin classifier split slice: moved nested reference
    parsing and local-reference helpers into `scripts/evidence_origin_references.py`
    while keeping `scripts/evidence_origin.py` focused on origin decisions and
    report-facing classification. This preserves the existing production
    evidence boundary behavior and restores room for future guards without
    growing the orchestration file past the local size limit.
159. Production evidence origin regression gate slice: added
    `make production-evidence-origin-check` and
    `scripts/evidence_origin_check.py` as a repeatable classifier/origin
    regression gate with a JSON report at
    `target/production-evidence-origin/report.json`. The gate locks in local
    reference detection, operator-reference allowances, path-origin classes,
    and symlink-to-generated artifact handling without supplying or simulating
    real operator KMS/HSM custody or compliance evidence.
160. Production evidence preflight origin-prerequisite slice: wired
    `receipt-production-evidence-preflight-check` to run
    `production-evidence-origin-check` first. Missing-input preflight still
    fails closed on the six required operator inputs, but now the reusable
    origin classifier regression report is refreshed before any operator
    evidence is accepted or rejected.
161. Production evidence operator handoff slice: extended
    `receipt-production-evidence-preflight-check` reports with a
    machine-readable `operator_handoff` block. The block records the required
    KMS/HSM custody and compliance evidence inputs, accepted schema versions,
    runtime signer bindings, rejected non-operator origin classes, and the
    exact preflight command shape operators must satisfy. This is report-only
    handoff metadata and does not synthesize or replace real operator evidence.
162. Production evidence standalone handoff slice: added
    `make receipt-production-evidence-handoff-check` and
    `scripts/receipt_production_evidence_handoff.py` so operators can generate
    the same machine-readable KMS/HSM custody and compliance evidence checklist
    without running the fail-closed preflight path. The target is report-only:
    it emits no readiness status and does not weaken
    `receipt-production-evidence-preflight-check` or
    `receipt-production-ready-check`.
163. Production evidence field-level handoff slice: moved the handoff payload
    builder into `scripts/receipt_production_evidence_handoff_payload.py` and
    extended the standalone/preflight handoff report with
    `evidence_field_checklist`. The checklist exposes the exact JSON fields,
    required values, required controls, artifact digest requirements, and
    forbidden secret field names enforced by the KMS/HSM custody and compliance
    evidence validators. The handoff remains report-only and does not create
    production evidence.
164. Production evidence handoff consistency gate slice: added
    `make receipt-production-evidence-handoff-consistency-check` and
    `scripts/receipt_production_evidence_handoff_check.py` to detect drift
    between the machine-readable handoff checklist and the validator constants.
    The gate verifies required preflight inputs, schema names, runtime signer
    schema bindings, validator controls, forbidden secret field names, artifact
    digest requirements, rejected origin classes, and confirms the handoff
    remains report-only with no readiness status.
165. Production evidence preflight consistency-prerequisite slice: wired
    `receipt-production-evidence-preflight-check` to run
    `receipt-production-evidence-handoff-consistency-check` before validating
    operator evidence. Missing-input preflight still fails closed on the six
    required inputs, but now both the origin-classifier and handoff-consistency
    reports are refreshed before any supplied evidence can be accepted.
166. Production readiness handoff-consistency inventory slice: wired
    `receipt-production-readiness-check` to run
    `receipt-production-evidence-handoff-consistency-check` and pass its report
    into `scripts/receipt_production_readiness_check.py`. The aggregate
    readiness report now records
    `production_evidence_handoff_consistency` in `component_status`, `reports`,
    and `readiness`, while `production_ready` remains false without real
    operator-origin KMS/HSM custody and external compliance evidence.
167. Production ready preflight fail-fast slice: changed
    `receipt-production-ready-check` from a direct readiness prerequisite into
    an ordered recipe that runs `receipt-production-evidence-preflight-check`
    before the full readiness chain. Without the six required operator evidence
    inputs, the strict production claim path now fails at preflight, refreshes
    origin and handoff-consistency evidence reports, and does not run the full
    readiness/strict aggregators.
168. Production ready origin-proof preflight slice: split the parser-coverage
    preflight from the strict production claim preflight. The strict
    `receipt-production-ready-check` now runs
    `receipt-production-evidence-production-preflight-check`, which requires
    both KMS/HSM custody and compliance evidence JSON files to carry a
    `production_origin_proof` object with schema
    `cortexdb.operator_evidence_origin_proof.v1`, external proof references,
    hashed signed-statement binding, and `external_control_plane=true`.
    `receipt_production_readiness_check.py --require-production-ready` also
    requires a production preflight report with
    `production_origin_proof_required=true`, so component reports generated
    from local parser-only JSON cannot bypass the strict production claim gate.
    The non-strict preflight remains available for schema/parser coverage, but
    it is no longer sufficient for `production_ready=true`.
169. Production origin-proof content-binding slice: tightened
    `production_origin_proof` from a proof-shaped field bundle into a
    content-bound proof contract. The strict production preflight now requires
    `issuer_ref`, `signed_statement_sha256_hex`, `evidence_sha256_hex`, and
    `expires_at`; parses `issued_at`/`expires_at`; rejects local/generated refs
    in `issuer_ref`; and checks that `evidence_sha256_hex` equals the SHA-256 of
    the evidence JSON object after removing `production_origin_proof` and
    serializing with sorted keys and compact separators. This prevents an
    unbound proof-shaped local JSON flag from satisfying the strict production
    preflight. It is still not a substitute for real external KMS/HSM custody or
    compliance evidence.
170. Production origin-proof signed-statement binding slice: tightened
    `production_origin_proof` again so `proof_sha256_hex` must match the
    canonical proof object after removing `proof_sha256_hex`, and
    `signed_statement_sha256_hex` must match an embedded
    `cortexdb.operator_evidence_origin_statement.v1` object. The statement must
    match the enclosing proof and evidence body across schema, evidence digest,
    proof reference, issuer reference, issuer key id, issuer public-key
    reference, detached signature reference, signature digest, reviewer, timing,
    and external-control-plane fields. The strict production preflight now also
    requires `issuer_key_id`, `issuer_public_key_ref`, `signature_algorithm`,
    `signature_ref`, and `signature_sha256_hex`. This closes the previous
    parser-positive path where a content-bound proof could carry an arbitrary
    `signed_statement_sha256_hex` without a locally checkable statement body. It
    remains parser/contract coverage only, not proof that a real external
    issuer produced the detached signature or that real KMS/HSM/compliance
    evidence exists.
171. Production origin-proof signature-verification slice: added a
    `cortex-crypto` Ed25519 helper and wired strict production evidence
    preflight to verify `production_origin_proof.signature_hex` against
    `issuer_public_key_hex` over
    `cortexdb.operator_evidence_origin_statement.sign.v1 || 0x00 ||
    canonical_json(signed_statement)`. The strict proof contract now requires
    `issuer_public_key_hex` and `signature_hex`; checks
    `signature_sha256_hex` against the raw signature bytes; requires
    `signature_algorithm=ed25519`; removes the self-referential
    `signature_sha256_hex` field from signed statement bytes; and fails closed
    when a proof only carries `signature_ref`/`signature_sha256_hex` metadata.
    This proves the supplied statement bytes are signed by the supplied issuer
    public key, but it still does not prove that the issuer key or evidence
    artifacts are controlled by a real external KMS/HSM/compliance authority.
172. Production origin-proof issuer-key-attestation slice: tightened
    `production_origin_proof` so the issuer public key is no longer accepted as
    a bare self-asserted field. Strict production preflight now requires
    `issuer_key_attestation_ref`, `issuer_key_attestation_sha256_hex`, an
    embedded `cortexdb.operator_evidence_origin_key_attestation.v1` object,
    `key_attestor_ref`, `key_attestor_key_id`, `key_attestor_public_key_ref`,
    `key_attestor_public_key_hex`, `key_attestation_signature_algorithm`,
    `key_attestation_signature_ref`, `key_attestation_signature_hex`, and
    `key_attestation_signature_sha256_hex`. The key attestation must bind the
    issuer key, issuer references, statement signing domain, attestor key, proof
    timing, and external-control-plane fields, and its Ed25519 signature must
    verify over
    `cortexdb.operator_evidence_origin_key_attestation.sign.v1 || 0x00 ||
    canonical_json(issuer_key_attestation)`. This proves the supplied issuer key
    is attested by a second supplied key, but it still does not prove that the
    attestor key itself is a public external trust anchor.
173. Production origin-proof key-attestor trust-anchor slice: tightened strict
    production preflight so the key attestor is no longer accepted only from the
    evidence JSON. `receipt-production-evidence-production-preflight-check` now
    requires `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF`; both
    KMS/HSM custody and compliance evidence proofs must match those separately
    supplied expected trust-anchor values. This prevents a proof from inventing
    its own attestor key in-band, while still leaving real external trust-anchor
    publication and real operator KMS/HSM/compliance evidence as production
    blockers.
174. Production origin-proof trust-anchor publication slice: added
    `cortexdb.operator_evidence_origin_trust_anchor.v1` validation and required
    `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE` for strict production
    preflight. The trust-anchor evidence must be operator-origin, non-local,
    schema-valid, control-backed, artifact-hashed, and must bind
    `key_attestor_key_id`, `key_attestor_public_key_hex`,
    `key_attestor_ref`, and `key_attestor_public_key_ref` to the separately
    supplied expected trust-anchor inputs. Strict preflight now rejects missing,
    generated-local, or mismatched trust-anchor publications while non-strict
    parser/inventory preflight remains available. This still does not supply a
    real external trust registry or real KMS/HSM/compliance operator evidence.
175. Production origin-proof signed trust-anchor publication slice: tightened
    `cortexdb.operator_evidence_origin_trust_anchor.v1` so the publication
    artifact must be signed by a separately expected publisher key. Strict
    preflight now requires
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID`,
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF`, and
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF`.
    The evidence must carry `publisher_key_id`, `publisher_public_key_ref`,
    `publisher_public_key_hex`, `signature_algorithm=ed25519`,
    `signature_ref`, `signature_hex`, and `signature_sha256_hex`, and
    `signature_hex` must verify over
    `cortexdb.operator_evidence_origin_trust_anchor.sign.v1 || 0x00 ||
    canonical_json(trust_anchor_evidence_without_signature_hex_and_signature_sha256_hex)`.
    This prevents unsigned trust-anchor publications and expected-publisher
    mismatches from satisfying strict preflight. It remains parser/contract
    coverage only, not real external trust-registry evidence.
176. Receipt KMS/HSM runtime signing probe slice: tightened
    `cortexdb.receipt_kms_hsm_custody_evidence.v1` so custody evidence must
    include a `runtime_signing_probe` signed by the same runtime public key.
    The probe must bind the external signer request/response schemas,
    `key_id`, `public_key_hex`, `signer_ref`, signing domain, a
    `canonical_header_hex` challenge, canonical request/response SHA-256
    digests, `signature_hex`, `signature_sha256_hex`, and `signed_at`;
    `signature_hex` must verify over
    `cortexdb.accountability_receipt.sign.v1 || 0x00 ||
    canonical_header_hex bytes`. `receipt-kms-hsm-custody-check` now rejects
    missing probes, bad probe signatures, and probe/key binding mismatches. The
    fixture remains synthetic parser coverage and still does not prove real
    operator KMS/HSM custody.
177. Receipt KMS/HSM component production-origin proof slice: tightened
    `receipt-kms-hsm-custody-check` so the standalone component can no longer
    set `kms_hsm_custody=true` from operator-shaped custody JSON that only has
    runtime signer metadata and a valid `runtime_signing_probe`. When
    `RECEIPT_KMS_HSM_CUSTODY_EVIDENCE` is supplied, the component gate now
    requires a valid `production_origin_proof`, requires that proof to satisfy
    the same issuer statement/key-attestation signature checks as strict
    production preflight, and requires separately supplied
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*` inputs. Missing
    expected attestor inputs, missing proof, or fixture evidence keep
    `kms_hsm_custody=false` and `production_safe=false`; the proof-bound
    operator-shaped artifact remains parser coverage only, not real KMS/HSM
    operator evidence.
178. Compliance component production-origin proof slice: tightened
    `compliance-boundary-check` so the standalone component can no longer set
    `supported_certified_frameworks` or `compliance_immutability=true` from
    operator-shaped certification JSON that only has external reviewer,
    immutability, controls, and artifact metadata. When
    `COMPLIANCE_CERTIFICATION_EVIDENCE` is supplied, the component gate now
    requires a valid `production_origin_proof`, requires that proof to satisfy
    the same issuer statement/key-attestation signature checks as strict
    production preflight, and requires separately supplied
    `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*` inputs. Missing
    expected attestor inputs, missing proof, or fixture evidence keep
    `supported_certified_frameworks=[]` and `compliance_immutability=false`;
    the proof-bound operator-shaped artifact remains parser coverage only, not
    real external compliance certification evidence.
179. Production origin-proof independent trust-anchor publisher slice:
    tightened `cortexdb.operator_evidence_origin_trust_anchor.v1` validation so
    a signed trust-anchor publication can no longer be self-published by the
    same key-attestor identity that it is supposed to anchor. The publisher key
    id, publisher public key, publisher ref, and publisher public-key ref must
    all be distinct from the corresponding `key_attestor_*` fields while still
    matching the separately supplied expected publisher inputs. A self-published,
    correctly signed, operator-shaped JSON artifact is now rejected; an
    independently signed parser artifact with a separate publisher key still
    validates. This remains parser/contract coverage only, not real external
    trust-registry evidence.
180. Production evidence distinct artifacts slice: tightened the shared KMS/HSM
    custody, external compliance certification, and production-origin
    trust-anchor validators so `evidence_artifacts` cannot satisfy the
    two-artifact requirement by duplicating the same entry. The validator now
    requires at least two entries, at least two distinct artifact URIs, and at
    least two distinct artifact SHA-256 digests; duplicate `{kind, uri,
    sha256_hex}` entries are rejected. This improves parser/contract evidence
    strength while still leaving real external KMS/HSM, compliance, and
    trust-registry evidence as production blockers.
181. Production evidence top-level expiry slice: tightened the KMS/HSM custody,
    external compliance certification, and production-origin trust-anchor
    validators so top-level validity windows must still be active at validation
    time. `operator_attestation.valid_until`, `external_review.valid_until`,
    and trust-anchor `valid_until` now fail if they are in the past even when
    the evidence is otherwise schema-valid, signed, and ordered after the
    corresponding `issued_at`/`published_at`. Timestamp parsing for KMS/HSM and
    compliance evidence now uses the shared normalizing parser so timezone
    forms are handled consistently.
182. Production origin-proof expiry slice: tightened
    `production_origin_proof` validation so its `expires_at` window must still
    be active at validation time, not merely after `issued_at`. Expired
    proof-bound KMS/HSM and compliance parser artifacts now fail with
    `production_origin_proof.expires_at must be in the future`, while fresh
    proof-bound parser artifacts remain valid. This remains parser/contract
    coverage only and does not supply real external KMS/HSM custody,
    compliance certification, or trust-registry evidence.
183. Receipt KMS/HSM runtime signing probe freshness slice: tightened
    `runtime_signing_probe.signed_at` validation so a live signer probe must be
    recent, not merely timestamp-shaped. The validator rejects probes older than
    24 hours at validation time and probes more than 300 seconds in the future;
    stale proof-shaped parser evidence now fails while fresh parser evidence
    remains valid. This does not turn external command signing into real
    KMS/HSM custody evidence.
184. Production evidence future timestamp slice: tightened the KMS/HSM custody,
    external compliance certification, production-origin trust-anchor, and
    production-origin proof validators so issue/publication timestamps cannot
    be arbitrarily future-dated. `operator_attestation.issued_at`,
    `external_review.issued_at`, trust-anchor `published_at`, and
    `production_origin_proof.issued_at` now fail if they are more than 300
    seconds in the future at validation time. Future-dated parser artifacts
    that were accepted before the guard now fail, while current parser
    artifacts and synthetic parser fixtures remain valid. This remains
    parser/contract coverage only and does not supply real external KMS/HSM
    custody, compliance certification, or trust-registry evidence.
185. Production origin-proof detached statement signature slice: tightened
    `production_origin_proof.signed_statement` validation so detached statement
    signature bytes cannot be duplicated inside the signed statement body.
    `signed_statement.signature_hex` and `signed_statement.signature_sha256_hex`
    are now both rejected, keeping the statement signature and its digest
    outside the canonical signed bytes. A proof-bound parser artifact with an
    embedded `signed_statement.signature_hex` was accepted before the guard and
    is now rejected, while clean detached-signature parser artifacts remain
    valid. This remains parser/contract coverage only and does not supply real
    external KMS/HSM custody, compliance certification, or trust-registry
    evidence.
186. Production origin-proof nested digest detachment slice: tightened
    nested `issuer_key_attestation` and `signed_statement` validation so
    detached digest fields cannot be duplicated inside the canonical signed
    objects. `issuer_key_attestation.issuer_key_attestation_sha256_hex` and
    `signed_statement.signed_statement_sha256_hex` are now rejected, keeping
    each nested object's digest at the enclosing proof level. Proof-bound
    parser artifacts with nested digest fields were accepted before the guard
    and are now rejected, while clean detached-digest parser artifacts remain
    valid. This remains parser/contract coverage only and does not supply real
    external KMS/HSM custody, compliance certification, or trust-registry
    evidence.
187. Production origin-proof nested closed-shape slice: tightened nested
    `issuer_key_attestation` and `signed_statement` validation so the signed v1
    objects cannot carry extra fields that are not part of their schemas.
    Proof-bound parser artifacts with signed `unreviewed_extension` fields were
    accepted before the guard and are now rejected, while clean parser artifacts
    with only the required v1 fields remain valid. This prevents signed but
    unvalidated operator claims from entering nested production-origin proof
    objects, and remains parser/contract coverage only rather than real
    external KMS/HSM custody, compliance certification, or trust-registry
    evidence.
188. Production origin-proof top-level closed-shape slice: tightened
    `production_origin_proof` validation so the signed top-level proof object
    cannot carry extra fields outside the v1 proof schema. Proof-bound parser
    artifacts with a signed `production_origin_proof.unreviewed_extension`
    field were accepted before the guard and are now rejected, while clean
    parser artifacts with only required v1 proof fields remain valid. This
    prevents signed but unvalidated proof-level claims from entering strict
    production evidence, and remains parser/contract coverage only rather than
    real external KMS/HSM custody, compliance certification, or trust-registry
    evidence.
189. Receipt KMS/HSM custody top-level closed-shape slice: tightened
    `cortexdb.receipt_kms_hsm_custody_evidence.v1` validation so the top-level
    custody evidence object cannot carry extra fields outside the documented
    v1 schema plus `production_origin_proof`. A schema-valid KMS/HSM parser
    artifact with `unreviewed_extension` was accepted before the guard and is
    now rejected, while the clean KMS/HSM fixture remains valid. This prevents
    unvalidated custody-level claims from sitting beside validated KMS/HSM
    fields, and remains parser/contract coverage only rather than real
    external KMS/HSM custody evidence.
190. Compliance certification top-level closed-shape slice: tightened
    `cortexdb.compliance_certification_evidence.v1` validation so the
    top-level certification evidence object cannot carry extra fields outside
    the documented v1 schema plus `production_origin_proof`. A schema-valid
    compliance parser artifact with `unreviewed_extension` was accepted before
    the guard and is now rejected, while the clean compliance fixture remains
    valid. This prevents unvalidated certification or immutability claims from
    sitting beside validated compliance fields, and remains parser/contract
    coverage only rather than real external compliance certification evidence.
191. Compliance certification nested closed-shape slice: tightened
    `external_review`, `scope`, and `immutability_evidence` validation so
    nested compliance evidence objects cannot carry extra fields outside their
    documented v1 shapes. A schema-valid compliance parser artifact with
    nested `unreviewed_extension` fields was accepted before the guard and is
    now rejected, while the clean compliance fixture remains valid. This
    prevents unvalidated nested review, scope, or immutability claims from
    sitting beside validated compliance evidence fields, and remains
    parser/contract coverage only rather than real external compliance
    certification evidence.
192. Receipt KMS/HSM custody nested closed-shape slice: tightened
    `runtime_binding`, `runtime_signing_probe`, and `operator_attestation`
    validation so nested KMS/HSM custody evidence objects cannot carry extra
    fields outside their documented v1 shapes. A schema-valid KMS/HSM parser
    artifact with nested `unreviewed_extension` fields was accepted before the
    guard and is now rejected, while the clean KMS/HSM fixture remains valid.
    This prevents unvalidated runtime, probe, or operator-control claims from
    sitting beside validated KMS/HSM custody evidence fields, and remains
    parser/contract coverage only rather than real external KMS/HSM custody
    evidence.
193. Production evidence artifact item closed-shape slice: tightened the
    shared KMS/HSM custody, compliance certification, and production-origin
    trust-anchor artifact validator so each `evidence_artifacts[]` item is a
    closed v1 shape. Parser artifacts whose artifact entries carried
    `unreviewed_extension` fields were accepted before the guard and are now
    rejected, while clean KMS/HSM and compliance fixtures plus a clean signed
    trust-anchor parser artifact remain valid. This prevents unvalidated
    artifact-level claims from sitting beside hashed evidence references, and
    remains parser/contract coverage only rather than real external KMS/HSM,
    compliance, or trust-registry evidence.
194. Production-origin trust-anchor top-level closed-shape slice: tightened
    `cortexdb.operator_evidence_origin_trust_anchor.v1` validation so the
    top-level signed trust-anchor publication object cannot carry extra fields
    outside the documented v1 schema. A signed trust-anchor parser artifact
    with `unreviewed_extension` was accepted before the guard and is now
    rejected, while a clean signed trust-anchor parser artifact remains valid.
    This prevents signed but unvalidated trust-registry publication claims from
    sitting beside validated trust-anchor fields, and remains parser/contract
    coverage only rather than real external trust-registry evidence.
195. Production evidence controls closed-set slice: tightened KMS/HSM custody,
    compliance certification, and production-origin trust-anchor validators so
    required controls lists cannot carry extra or duplicate entries. Parser
    artifacts whose controls contained `unreviewed_control_claim` plus a
    duplicate required control were accepted before the guard and are now
    rejected, while clean KMS/HSM and compliance fixtures plus a clean signed
    trust-anchor parser artifact remain valid. This prevents unvalidated
    operator, reviewer, or trust-registry control claims from sitting beside
    validated required controls, and remains parser/contract coverage only
    rather than real external KMS/HSM, compliance, or trust-registry evidence.
196. Compliance operator responsibilities closed-set slice: tightened
    `operator_responsibilities` validation so compliance certification evidence
    must carry exactly the reviewed responsibility set without extra or
    duplicate entries. Parser evidence containing an
    `unreviewed operator responsibility claim` plus a duplicate required
    responsibility was accepted before the guard and is now rejected, while the
    clean compliance fixture remains valid. This prevents unvalidated reviewer
    responsibility claims from sitting beside the required compliance boundary,
    and remains parser/contract coverage only rather than real external
    compliance certification evidence.
197. Production evidence non-local reference slice: tightened shared artifact
    URI validation plus KMS/HSM and compliance top-level evidence refs so
    local/generated references cannot remain parser-valid as external evidence.
    KMS/HSM parser evidence with a `target/...` `signer_ref` and compliance
    parser evidence with `target/...` `report_ref`, retention-policy ref, and
    tamper-evidence ref were accepted before the guard and are now rejected,
    while clean KMS/HSM and compliance fixtures remain valid. This prevents
    local build/test paths from sitting in the KMS signer or compliance report
    reference fields, and remains parser/contract coverage only rather than
    real external KMS/HSM or compliance evidence.
198. Production trust-anchor non-local reference slice: tightened top-level
    trust-anchor reference validation so signed trust-anchor publications cannot
    carry local/generated `key_attestor_ref`, `key_attestor_public_key_ref`,
    `publisher_ref`, `publisher_public_key_ref`, `publication_ref`, or
    `signature_ref` values as parser-valid external trust-registry evidence. A
    signed trust-anchor parser artifact with `target/...` top-level refs was
    accepted before the guard and is now rejected, while a clean signed
    trust-anchor parser artifact remains valid. This prevents local build/test
    paths from sitting in trust-registry publication fields, and remains
    parser/contract coverage only rather than real external trust-anchor
    evidence.
199. Production evidence artifact-kind closed-set slice: tightened shared
    `evidence_artifacts[]` validation so KMS/HSM custody, compliance
    certification, and trust-anchor evidence cannot carry arbitrary artifact
    `kind` values beside otherwise valid `uri`/`sha256_hex` references. Parser
    evidence with `unreviewed_*` artifact kinds was accepted before the guard
    and is now rejected, while the clean KMS/HSM fixture, compliance fixture,
    and signed trust-anchor parser artifact remain valid. This prevents
    unsupported artifact classification claims from sitting in production
    evidence objects, and remains parser/contract coverage only rather than
    real external KMS/HSM, compliance, or trust-anchor evidence.
200. Production-origin reviewer independence slice: tightened
    `production_origin_proof` validation so `reviewed_by` cannot equal the
    issuer or key-attestor identity fields (`issuer_ref`, `issuer_key_id`,
    `issuer_public_key_ref`, `key_attestor_ref`, `key_attestor_key_id`, or
    `key_attestor_public_key_ref`). A signed compliance parser artifact with
    `reviewed_by == issuer_ref` was accepted before the guard and is now
    rejected, while the same signed artifact with an independent reviewer
    remains valid. This prevents self-reviewed production-origin proofs from
    sitting beside valid proof digests and signatures, and remains
    parser/contract coverage only rather than real external KMS/HSM,
    compliance, or trust-anchor evidence.
201. Production-origin issuer-attestor independence slice: tightened
    `production_origin_proof` validation so the issuer identity cannot equal
    the key-attestor identity across the corresponding ref, key-id, public-key
    ref, or public-key hex fields. A signed compliance parser artifact whose
    issuer and key-attestor were the same key and reference was accepted before
    the guard and is now rejected, while a signed artifact with separate issuer
    and key-attestor identities remains valid. This prevents self-attested
    issuer keys from sitting beside valid proof digests and signatures, and
    remains parser/contract coverage only rather than real external KMS/HSM,
    compliance, or trust-anchor evidence.
202. Production evidence artifact digest lowercase slice: tightened the shared
    `evidence_artifacts[].sha256_hex` validator so artifact digests must be
    exactly 64 lowercase hex characters instead of being normalized with
    `.lower()` before validation. KMS/HSM and compliance parser artifacts with
    uppercase SHA-256 digests were accepted before the guard and are now
    rejected, while clean lowercase fixtures remain valid. This prevents
    non-canonical artifact hash claims from sitting beside production evidence
    references, and remains parser/contract coverage only rather than real
    external KMS/HSM, compliance, or trust-anchor evidence.
203. Production-origin proof key-id canonicalization slice: tightened
    `production_origin_proof` validation so `issuer_key_id` and
    `key_attestor_key_id` cannot contain whitespace anywhere in the signed proof
    object. A signed KMS/HSM parser artifact whose proof key ids contained
    internal spaces was accepted before the guard and is now rejected, while a
    clean signed proof-bound parser artifact remains valid. This aligns the
    production-origin proof key identifiers with the KMS/HSM `key_id`
    no-whitespace contract and remains parser/contract coverage only rather
    than real external KMS/HSM, compliance, or trust-anchor evidence.
204. Production-origin proof reference canonicalization slice: tightened
    `production_origin_proof` validation so external proof reference fields
    cannot contain raw whitespace anywhere in the signed proof object. A signed
    KMS/HSM parser artifact whose `proof_ref` contained an internal space was
    accepted before the guard and is now rejected, while a clean signed
    proof-bound parser artifact remains valid. This keeps proof references as
    non-local external identifiers rather than signed but non-canonical URI-like
    strings, and remains parser/contract coverage only rather than real
    external KMS/HSM, compliance, or trust-anchor evidence.
205. Receipt KMS/HSM runtime reference canonicalization slice: tightened
    KMS/HSM custody validation so top-level `provider_key_ref` and `signer_ref`
    plus nested `runtime_signing_probe.signer_ref` cannot contain raw
    whitespace. A KMS/HSM parser artifact whose signer reference contained an
    internal space and whose runtime request digest was recomputed for that
    value was accepted before the guard and is now rejected, while the clean
    custody fixture remains valid. This keeps runtime signer/provider
    references as canonical external identifiers and remains parser/contract
    coverage only rather than real external KMS/HSM custody evidence.
206. Compliance evidence reference canonicalization slice: tightened compliance
    certification validation so `report_ref`,
    `immutability_evidence.retention_policy_ref`, and
    `immutability_evidence.tamper_evidence_ref` cannot contain raw whitespace.
    A compliance parser artifact whose `report_ref` contained an internal space
    was accepted before the guard and is now rejected, while the clean
    compliance fixture remains valid. This keeps compliance report and
    immutability references as canonical external identifiers and remains
    parser/contract coverage only rather than real external compliance
    certification evidence.
207. Production evidence artifact URI canonicalization slice: tightened shared
    `evidence_artifacts[]` validation so artifact `uri` values cannot contain
    raw whitespace. KMS/HSM and compliance parser artifacts with internal spaces
    in `evidence_artifacts[0].uri` were accepted before the guard and are now
    rejected, while the clean KMS/HSM and compliance fixtures remain valid.
    Because this uses the shared artifact validator, the same URI canonicality
    guard applies to KMS/HSM custody, compliance certification, and trust-anchor
    evidence artifact lists. This remains parser/contract coverage only rather
    than real external KMS/HSM, compliance, or trust-registry evidence.
208. Trust-anchor reference canonicalization slice: tightened
    `cortexdb.operator_evidence_origin_trust_anchor.v1` validation so
    top-level trust-anchor reference fields cannot contain raw whitespace. A
    signature-valid trust-anchor parser artifact whose `publication_ref`
    contained an internal space was accepted before the guard and is now
    rejected, while a clean re-signed trust-anchor parser artifact remains
    valid. This keeps signed trust-registry publication references canonical
    and remains parser/contract coverage only rather than real external
    trust-registry evidence.
209. Trust-anchor key-id canonicalization slice: tightened
    `cortexdb.operator_evidence_origin_trust_anchor.v1` validation so
    `key_attestor_key_id` and `publisher_key_id` cannot contain whitespace. A
    signature-valid trust-anchor parser artifact whose `key_attestor_key_id`
    contained an internal space was accepted before the guard and is now
    rejected, while a clean re-signed trust-anchor parser artifact remains
    valid. This keeps trust-registry key identifiers canonical and remains
    parser/contract coverage only rather than real external trust-registry
    evidence.
210. Production-origin proof reviewer canonicalization slice: tightened
    `production_origin_proof` validation so `reviewed_by` cannot contain
    whitespace anywhere in the signed proof, signed statement, or issuer key
    attestation identity binding. A signed compliance parser artifact whose
    proof reviewer identity was `external reviewer` was accepted before the
    guard and is now rejected, while a clean signed proof-bound parser artifact
    with a whitespace-free reviewer identity remains valid. This keeps the
    independent reviewer identity canonical and remains parser/contract
    coverage only rather than real external KMS/HSM, compliance, or
    trust-registry evidence.
211. Production evidence timezone-aware timestamp slice: tightened shared
    production evidence timestamp parsing so KMS/HSM operator attestations,
    runtime signing probes, compliance external reviews, trust-anchor
    publications, and production-origin proofs must use timezone-aware
    ISO-8601 timestamps. Compliance, KMS/HSM, and signed trust-anchor parser
    artifacts with timezone-less timestamps were accepted before the guard, and
    a signed production-origin proof with timezone-less timestamps crashed the
    validator with a naive/aware datetime comparison; all now fail closed with
    explicit timezone-aware timestamp errors, while timezone-aware fixtures and
    signed parser artifacts remain valid. This prevents ambiguous evidence
    timing from being silently interpreted as UTC and remains parser/contract
    coverage only rather than real external KMS/HSM, compliance, or
    trust-registry evidence.
212. Production evidence normalized secret-field guard slice: tightened shared
    forbidden secret field detection across KMS/HSM custody, compliance
    certification, and trust-anchor evidence so recursive field-name matching is
    case-insensitive and normalized across snake_case, camelCase, kebab-case,
    and compact aliases. KMS/HSM and compliance parser evidence with optional
    non-strict `production_origin_proof.privateKey` / `apiToken` fields was
    previously accepted as valid parser coverage because optional proof failures
    were not promoted outside strict production-origin mode; those mixed-case
    secret fields now fail closed before any component can report valid evidence.
    Clean KMS/HSM and compliance fixtures remain valid. This prevents inline
    secret material from hiding in optional production-origin metadata and
    remains parser/contract coverage only rather than real external KMS/HSM,
    compliance, or trust-registry evidence.
213. Production readiness aggregate proof-binding guard slice: tightened
    `receipt-production-readiness-check` so strict production readiness no
    longer trusts component summary booleans alone. The aggregate now requires
    the production evidence preflight report to carry consistent nested
    readiness plus operator evidence, and it independently requires both
    KMS/HSM custody and compliance component reports to include
    `production_origin_proof_required=true` and
    `production_origin_proof_valid=true` on operator-origin evidence before
    `production_ready=true` can be reported. A forged/weak aggregate fixture
    that previously passed strict readiness with proof flags set false is now
    rejected, while a proof-bound positive aggregate fixture still passes. This
    keeps the production claim gate tied to proof-bound component evidence and
    remains parser/contract coverage only rather than real external KMS/HSM,
    compliance, or trust-registry evidence.
214. Production component trust-anchor publication guard slice: tightened
    standalone `receipt-kms-hsm-custody-check` and
    `compliance-boundary-check` so proof-bound component evidence is no longer
    sufficient by itself to set `kms_hsm_custody=true`,
    `production_safe=true`, `supported_certified_frameworks`, or
    `compliance_immutability=true`. When component evidence is supplied, those
    gates now also require `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`
    plus expected key-attestor and trust-anchor publisher inputs, and the
    trust-anchor publication must validate as operator-origin evidence before
    component production booleans can be set. Missing or invalid trust-anchor
    publication evidence keeps the component reports in blocker state, while a
    trust-anchor-bound positive component path remains available. This aligns
    standalone component reports with the strict preflight trust-anchor
    boundary and remains parser/contract coverage only rather than real
    external KMS/HSM, compliance, or trust-registry evidence.
215. Production readiness aggregate trust-anchor binding guard slice: tightened
    `receipt-production-readiness-check` so strict production readiness can no
    longer accept forged component reports that set `kms_hsm_custody=true`,
    `production_safe=true`, `supported_certified_frameworks`, or
    `compliance_immutability=true` from proof-bound evidence while omitting the
    component-level `production_origin_trust_anchor` validation. The aggregate
    now requires both KMS/HSM and compliance component reports to carry an
    operator-origin trust-anchor validation before `production_ready=true` can
    be reported. A forged aggregate fixture without component trust-anchor
    evidence now remains blocked, while a positive aggregate fixture with
    operator-origin component trust-anchor evidence still passes. The current
    real-operator path remains honest: the handoff report is generated, but the
    strict production evidence preflight fails until external KMS/HSM custody,
    compliance certification, trust-anchor, key-attestor, and publisher inputs
    are supplied.

## Verification Log

| Date | Scope | Command | Result |
|---|---|---|---|
| 2026-06-28 | P0.1 | `cargo fmt --check` | passed |
| 2026-06-28 | P0.1 | `make canonical-serialization-check` | passed |
| 2026-06-28 | P0.1 | `cargo check -p cortex-engine` | passed |
| 2026-06-28 | P0.1 | `cargo clippy -p cortex-engine --all-targets -- -D warnings` | passed |
| 2026-06-28 | P0.1 | `git diff --check` | passed |
| 2026-06-28 | workspace | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | P0.2 pre-fix | `make engine-determinism-check` | failed: `docs/ENGINE_DETERMINISM.md` was missing |
| 2026-06-28 | P0.2 | `make engine-determinism-check` | passed |
| 2026-06-28 | P0.3/P0.4 | `cargo test -p cortex-engine canonical` | passed |
| 2026-06-28 | P0.3/P0.4 | `python3 scripts/canonical_serialization_check.py --report target/canonical-serialization/dev-report.json` | passed |
| 2026-06-28 | P0 final | `cargo fmt --check` | passed |
| 2026-06-28 | P0 final | `make canonical-serialization-check` | passed |
| 2026-06-28 | P0 final | `make engine-determinism-check` | passed |
| 2026-06-28 | P0 final | `git diff --check` | passed |
| 2026-06-28 | workspace after P0 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after P0 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CP-1/CP-2 | `make cosine-metric-correctness-check` | passed |
| 2026-06-28 | CP-1/CP-2 | `cargo test -p cortex-engine test_deterministic_cosine_similarity` | passed |
| 2026-06-28 | CP-1/CP-2 | `cargo test -p cortex-engine context_pack_can_reduce_dense_vector_redundancy` | passed |
| 2026-06-28 | CP-1/CP-2 | `make ann-metric-matrix-check` | passed |
| 2026-06-28 | CP-1/CP-2 final | `cargo fmt --check` | passed |
| 2026-06-28 | CP-1/CP-2 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CP-1/CP-2 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-1/CP-2 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CP-3 preflight | `make cell-id-collision-check` | failed: direct 31-bit slot cannot fit the current top-nibble namespace plus 32-bit sequence layout |
| 2026-06-28 | CP-3 | `make cell-id-collision-check` | passed |
| 2026-06-28 | CP-3 final | `cargo fmt --check` | passed |
| 2026-06-28 | CP-3 final | `git diff --check` | passed |
| 2026-06-28 | CP-3 final | `make canonical-serialization-check` | passed |
| 2026-06-28 | CP-3 final | `make cosine-metric-correctness-check` | passed |
| 2026-06-28 | CP-3 final | `make engine-determinism-check` | passed |
| 2026-06-28 | workspace after CP-3 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-3 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CP-4 | `make conflict-normalization-check` | passed |
| 2026-06-28 | CP-4 final | `cargo fmt --check` | passed |
| 2026-06-28 | CP-4 final | `git diff --check` | passed |
| 2026-06-28 | CP-4 final | `make canonical-serialization-check` | passed |
| 2026-06-28 | CP-4 final | `make engine-determinism-check` | passed |
| 2026-06-28 | CP-4 final | `make cosine-metric-correctness-check` | passed |
| 2026-06-28 | CP-4 final | `make cell-id-collision-check` | passed |
| 2026-06-28 | workspace after CP-4 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-4 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CP-5 | `cargo test -p cortex-engine --test ann_budget_disclosure --all-features` | passed |
| 2026-06-28 | CP-5 | `make ann-budget-disclosure-check` | passed |
| 2026-06-28 | CP-5 contract | `make context-pack-schema-contract-check` | passed |
| 2026-06-28 | CP-5 contract | `make openapi-contract-check` | passed |
| 2026-06-28 | CP-5 final | `cargo fmt --check` | passed |
| 2026-06-28 | CP-5 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CP-5 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-5 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CP-6 | `make correctness-prerequisites-check` | passed |
| 2026-06-28 | CP-6 | `python3 scripts/correctness_prerequisites_check.py --root . --report target/correctness-prerequisites/direct-report.json` | passed |
| 2026-06-28 | CP-6 final | `cargo fmt --check` | passed |
| 2026-06-28 | CP-6 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CP-6 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-6 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Phase 2 preflight | `make crypto-deps-readiness-check` | passed; reports legacy policy blockers for `crypto-deps-policy-check` |
| 2026-06-28 | Phase 2 preflight | `cargo check --workspace --all-features` | passed |
| 2026-06-28 | Phase 2 preflight final | `cargo fmt --check` | passed |
| 2026-06-28 | Phase 2 preflight final | `git diff --check` | passed |
| 2026-06-28 | workspace after Phase 2 preflight | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Phase 2 preflight | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-2 | `make crypto-primitives-check` | passed |
| 2026-06-28 | CRY-2 final | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | CRY-2 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CRY-2 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-2 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-3 | `cargo test -p cortex-engine encrypted_backup` | passed |
| 2026-06-28 | CRY-3 | `make encrypted-backup-legacy-refuse-check` | passed |
| 2026-06-28 | CRY-3 | `make encrypted-backup-check` | passed |
| 2026-06-28 | CRY-3 | `make crypto-deps-readiness-check` | passed; actual legacy blockers now remain only in audit-chain |
| 2026-06-28 | CRY-3 final | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | CRY-3 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CRY-3 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-3 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-4 | `make crypto-primitives-check` | passed; audit-chain primitives are included in the evidence report |
| 2026-06-28 | CRY-4 | `make crypto-deps-policy-check` | passed; production backup/audit paths are free of legacy FNV/XOR markers |
| 2026-06-28 | CRY-4 | `make audit-chain-check` | passed |
| 2026-06-28 | CRY-4 final | `cargo fmt --check` | passed |
| 2026-06-28 | CRY-4 final | `git diff --check` | passed |
| 2026-06-28 | workspace after CRY-4 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-4 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-6 keyed audit | `make key-management-check` | passed |
| 2026-06-28 | CRY-6 keyed audit | `make audit-chain-check` | passed after v2 MAC wiring |
| 2026-06-28 | CRY-6 keyed audit | `make crypto-deps-policy-check` | passed after v2 MAC wiring |
| 2026-06-28 | CRY-6 keyed audit final | `cargo fmt --check` | passed |
| 2026-06-28 | CRY-6 keyed audit final | `git diff --check` | passed |
| 2026-06-28 | workspace after CRY-6 keyed audit | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-6 keyed audit | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-7 public claims | `make crypto-claims-honesty-check` | passed |
| 2026-06-28 | CRY-7 public claims | `make audit-productization-check` | passed after v2/MAC doc marker refresh |
| 2026-06-28 | CRY-7 public claims | `python3 scripts/security_hardening_check.py --report target/security-hardening/cry7-marker-report.json` | passed |
| 2026-06-28 | CRY-7 public claims final | `cargo fmt --check` | passed |
| 2026-06-28 | CRY-7 public claims final | `git diff --check` | passed |
| 2026-06-28 | CRY-7 public claims final | `python3 -m py_compile scripts/crypto_claims_honesty_check.py scripts/audit_productization_check.py scripts/security_hardening_check.py` | passed |
| 2026-06-28 | workspace after CRY-7 public claims | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-7 public claims | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-6 receipt-key custody | `cargo test -p cortex-crypto receipt` | passed |
| 2026-06-28 | CRY-6 receipt-key custody | `cargo test -p cortex-server parse_receipt_signing_key` | passed |
| 2026-06-28 | CRY-6 receipt-key custody | `cargo test -p cortex-cli receipt_key_generate_export_and_rotate_preserves_dual_trust` | passed |
| 2026-06-28 | CRY-6 receipt-key custody | `make key-management-check` | passed with receipt key custody plus keyed audit MAC coverage |
| 2026-06-28 | CRY-6 receipt-key custody | `make crypto-primitives-check` | passed with receipt keyring primitives |
| 2026-06-28 | CRY-6 receipt-key custody | `make crypto-claims-honesty-check` | passed after documenting key custody without claiming receipt emission |
| 2026-06-28 | CRY-6 receipt-key custody | `make secrets-check` | passed |
| 2026-06-28 | CRY-6 receipt-key custody final | `cargo fmt --check` | passed |
| 2026-06-28 | CRY-6 receipt-key custody final | `git diff --check` | passed |
| 2026-06-28 | CRY-6 receipt-key custody final | `python3 -m py_compile scripts/key_management_check.py scripts/crypto_primitives_check.py scripts/crypto_claims_honesty_check.py` | passed |
| 2026-06-28 | workspace after CRY-6 receipt-key custody | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-6 receipt-key custody | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-3 cell hash | `make accountability-cell-hash-check` | passed |
| 2026-06-28 | AR-3 cell hash final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-3 cell hash final | `git diff --check` | passed |
| 2026-06-28 | AR-3 cell hash final | `python3 -m py_compile scripts/accountability_cell_hash_check.py` | passed |
| 2026-06-28 | AR-3 cell hash final | `make canonical-serialization-check` | passed |
| 2026-06-28 | workspace after AR-3 cell hash | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-3 cell hash | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-6/FC-5 captured access | `make context-access-decision-capture-check` | passed |
| 2026-06-28 | AR-6/FC-5 captured access contract | `make context-pack-schema-contract-check` | passed |
| 2026-06-28 | AR-6/FC-5 captured access contract | `make openapi-contract-check` | passed |
| 2026-06-28 | AR-6/FC-5 captured access final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-6/FC-5 captured access final | `git diff --check` | passed |
| 2026-06-28 | AR-6/FC-5 captured access final | `python3 -m py_compile scripts/context_access_decision_capture_check.py scripts/accountability_cell_hash_check.py` | passed |
| 2026-06-28 | AR-6/FC-5 captured access final | `make canonical-serialization-check` | passed |
| 2026-06-28 | workspace after AR-6/FC-5 captured access | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-6/FC-5 captured access | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-8 aggregate | `python3 -m py_compile scripts/crypto_foundation_check.py scripts/security_gate_v2_check.py` | passed |
| 2026-06-28 | CRY-8 aggregate | `make crypto-foundation-check` | passed |
| 2026-06-28 | CRY-8 release lane | `make security-gate-v2-check` | passed |
| 2026-06-28 | CRY-8 release lane pre-fix | `make security-release-report-check` | failed: stale compliance gate path `docs/ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md` |
| 2026-06-28 | CRY-8 release lane | `python3 -m py_compile scripts/compliance_boundary_check.py scripts/future_epic_design_check.py` | passed |
| 2026-06-28 | CRY-8 release lane | `make compliance-boundary-check` | passed |
| 2026-06-28 | CRY-8 release lane | `python3 scripts/future_epic_design_check.py --epic enterprise-rbac --report target/future-epic-design/enterprise-rbac-report.json` | passed |
| 2026-06-28 | CRY-8 release lane | `make security-release-report-check` | passed |
| 2026-06-28 | CRY-8 final | `cargo fmt --check` | passed |
| 2026-06-28 | CRY-8 final | `git diff --check` | passed |
| 2026-06-28 | CRY-8 final | `python3 -m py_compile scripts/crypto_foundation_check.py scripts/security_gate_v2_check.py scripts/compliance_boundary_check.py scripts/future_epic_design_check.py scripts/context_access_decision_capture_check.py scripts/accountability_cell_hash_check.py` | passed |
| 2026-06-28 | workspace after CRY-8 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CRY-8 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-6 denied access pre-fix | `cargo test -p cortex-engine --lib retrieve_execution_report_captures_permission_denials_without_forbidden_payload` | failed: denied candidate was excluded by `PushAgentAllowed` before `PermissionFilter` |
| 2026-06-28 | AR-6 denied access | `cargo test -p cortex-engine --lib retrieve_execution_report_captures_permission_denials_without_forbidden_payload` | passed |
| 2026-06-28 | AR-6 denied access | `python3 scripts/context_access_decision_capture_check.py --root "." --report target/context-access-decision-capture/manual-report.json` | passed |
| 2026-06-28 | AR-6 denied access | `make context-access-decision-capture-check` | passed |
| 2026-06-28 | AR-6 denied access final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-6 denied access final | `git diff --check` | passed |
| 2026-06-28 | AR-6 denied access final | `python3 -m py_compile scripts/context_access_decision_capture_check.py` | passed |
| 2026-06-28 | workspace after AR-6 denied access | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-6 denied access | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-2 receipt schema | `python3 -m py_compile scripts/accountability_receipt_schema_check.py` | passed |
| 2026-06-28 | AR-2 receipt schema | `python3 scripts/accountability_receipt_schema_check.py --root "." --report target/accountability-receipt/manual-schema-report.json` | passed after fixing a malformed golden fixture hex digest |
| 2026-06-28 | AR-2 receipt schema | `make accountability-receipt-schema-check` | passed |
| 2026-06-28 | AR-2 receipt schema contract | `make context-pack-schema-contract-check` | passed |
| 2026-06-28 | AR-2 receipt schema contract | `make openapi-contract-check` | passed |
| 2026-06-28 | AR-2 receipt schema contract | `python3 scripts/generate_openapi_sdk_types.py --check` | passed |
| 2026-06-28 | AR-2 receipt schema final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-2 receipt schema final | `git diff --check` | passed |
| 2026-06-28 | AR-2 receipt schema final | `python3 -m py_compile scripts/accountability_receipt_schema_check.py scripts/context_access_decision_capture_check.py` | passed |
| 2026-06-28 | AR-2 receipt schema final | `cargo test -p cortexdb-sdk context_pack_v1` | passed |
| 2026-06-28 | workspace after AR-2 receipt schema | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-2 receipt schema | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-4 receipt body | `cargo test -p cortex-engine accountability_receipt_body --all-features` | passed |
| 2026-06-28 | AR-4 receipt body | `python3 scripts/accountability_receipt_determinism_check.py --root "." --report target/accountability-receipt/manual-determinism-report.json` | passed |
| 2026-06-28 | AR-4 receipt body | `make accountability-receipt-determinism-check` | passed |
| 2026-06-28 | AR-4 receipt body final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-4 receipt body final | `git diff --check` | passed |
| 2026-06-28 | AR-4 receipt body final | `python3 -m py_compile scripts/accountability_receipt_determinism_check.py scripts/accountability_receipt_schema_check.py scripts/context_access_decision_capture_check.py` | passed |
| 2026-06-28 | workspace after AR-4 receipt body | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-4 receipt body | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-5 signed header | `cargo test -p cortex-engine accountability_receipt_header --all-features` | passed |
| 2026-06-28 | AR-5 signed header | `python3 -m py_compile scripts/accountability_receipt_sign_check.py` | passed |
| 2026-06-28 | AR-5 signed header | `python3 scripts/accountability_receipt_sign_check.py --root "." --report target/accountability-receipt/manual-sign-report.json` | passed |
| 2026-06-28 | AR-5 signed header | `make accountability-receipt-sign-check` | passed |
| 2026-06-28 | AR-5 signed header regression | `make accountability-receipt-determinism-check` | passed |
| 2026-06-28 | AR-5 signed header regression | `make accountability-receipt-schema-check` | passed |
| 2026-06-28 | AR-5 signed header final | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | AR-5 signed header final | `git diff --check` | passed |
| 2026-06-28 | AR-5 signed header final | `python3 -m py_compile scripts/accountability_receipt_sign_check.py scripts/accountability_receipt_determinism_check.py scripts/accountability_receipt_schema_check.py scripts/context_access_decision_capture_check.py` | passed |
| 2026-06-28 | workspace after AR-5 signed header | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-5 signed header | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | AR-7 standalone verifier | `cargo test -p cortex-receipt-verify` | passed |
| 2026-06-28 | AR-7 standalone verifier | `cargo run -p cortex-receipt-verify -- --input fixtures/accountability_receipt/verify_input.golden.json` | passed |
| 2026-06-28 | AR-7 standalone verifier | `cargo tree -p cortex-receipt-verify --edges normal` | passed; dependency graph excludes `cortex-engine`, `cortex-storage`, `cortex-aql`, and `cortex-server` |
| 2026-06-28 | AR-7 standalone verifier | `python3 -m py_compile scripts/accountability_receipt_verify_check.py` | passed |
| 2026-06-28 | AR-7 standalone verifier | `python3 scripts/accountability_receipt_verify_check.py --root "." --fixture fixtures/accountability_receipt/verify_input.golden.json --report target/accountability-receipt/manual-verify-report.json` | passed |
| 2026-06-28 | AR-7 standalone verifier | `make accountability-receipt-verify-check` | passed |
| 2026-06-28 | AR-8 tamper suite | `python3 -m py_compile scripts/accountability_receipt_verify_check.py scripts/accountability_receipt_tamper_check.py scripts/accountability_receipt_check.py` | passed |
| 2026-06-28 | AR-8 tamper suite | `python3 scripts/accountability_receipt_tamper_check.py --root "." --fixture fixtures/accountability_receipt/verify_input.golden.json --report target/accountability-receipt/manual-tamper-report.json` | passed |
| 2026-06-28 | AR-8 receipt umbrella | `make accountability-receipt-check` | passed |
| 2026-06-28 | AR-7/AR-8 final | `cargo fmt --check` | passed |
| 2026-06-28 | AR-7/AR-8 final | `git diff --check` | passed |
| 2026-06-28 | AR-7/AR-8 final | `python3 -m py_compile scripts/accountability_receipt_verify_check.py scripts/accountability_receipt_tamper_check.py scripts/accountability_receipt_check.py scripts/accountability_receipt_sign_check.py scripts/accountability_receipt_determinism_check.py scripts/accountability_receipt_schema_check.py` | passed |
| 2026-06-28 | AR-7/AR-8 final | `make accountability-receipt-verify-check` | passed |
| 2026-06-28 | AR-7/AR-8 final | `make accountability-receipt-tamper-check` | passed |
| 2026-06-28 | AR-7/AR-8 final | `make accountability-receipt-check` | passed |
| 2026-06-28 | workspace after AR-7/AR-8 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after AR-7/AR-8 | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Phase 3 runtime emission pre-fix | `cargo test -p cortex-server emits_signed_accountability_receipt` | failed: configured-key responses still omitted `accountability_receipt` before runtime wiring |
| 2026-06-28 | Phase 3 runtime emission | `cargo test -p cortex-server emits_signed_accountability_receipt` | passed |
| 2026-06-28 | Phase 3 runtime emission contract pre-fix | `python3 scripts/generate_openapi_sdk_types.py --check` | failed: generated Python/TypeScript OpenAPI SDK types were stale |
| 2026-06-28 | Phase 3 runtime emission contract | `python3 scripts/generate_openapi_sdk_types.py --check` | passed after regeneration |
| 2026-06-28 | Phase 3 runtime emission contract | `make openapi-contract-check` | passed |
| 2026-06-28 | Phase 3 runtime emission contract pre-fix | `make accountability-receipt-schema-check` | failed: receipt docs were missing the exact configured-key fail-closed claim |
| 2026-06-28 | Phase 3 runtime emission contract | `make accountability-receipt-schema-check` | passed |
| 2026-06-28 | Phase 3 runtime emission contract | `make accountability-receipt-check` | passed |
| 2026-06-28 | Phase 3 runtime emission claims | `make crypto-claims-honesty-check` | passed |
| 2026-06-28 | Phase 3 runtime emission SDK | `python3 scripts/check_openapi_sdk_codegen_control.py` | passed |
| 2026-06-28 | Phase 3 runtime emission SDK | `python3 -m pytest sdk/python/test_cortexdb_client.py` | passed |
| 2026-06-28 | Phase 3 runtime emission SDK | `npm run typecheck --prefix sdk/typescript` | passed |
| 2026-06-28 | Phase 3 runtime emission final | `cargo fmt --check` | passed |
| 2026-06-28 | Phase 3 runtime emission final | `git diff --check` | passed |
| 2026-06-28 | Phase 3 runtime emission final | `python3 -m py_compile scripts/accountability_receipt_schema_check.py scripts/crypto_claims_honesty_check.py scripts/accountability_receipt_verify_check.py scripts/accountability_receipt_tamper_check.py scripts/accountability_receipt_check.py sdk/python/_cortexdb_client/model_types/verification.py` | passed |
| 2026-06-28 | workspace after Phase 3 runtime emission | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Phase 3 runtime emission | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | CRY-5 audit receipt binding pre-fix | `cargo test -p cortex-server receipt_hash` | failed: missing audit receipt hash helper/field before CRY-5 wiring |
| 2026-06-28 | CRY-5 audit receipt binding | `cargo test -p cortex-server receipt_hash` | passed |
| 2026-06-28 | CRY-5 audit receipt binding | `cargo test -p cortex-cli audit_review_verify_chain_rejects_receipt_hash_tampering` | passed |
| 2026-06-28 | CRY-5 audit receipt binding | `make audit-receipt-binding-check` | passed |
| 2026-06-28 | CRY-5 audit receipt binding regression | `make audit-chain-check` | passed |
| 2026-06-28 | CRY-5 audit receipt binding regression | `make crypto-foundation-check` | passed |
| 2026-06-28 | CRY-6 re-anchor pre-fix | `cargo test -p cortex-cli receipt_key_rotate_writes_verifiable_reanchor_record` | failed: `receipt-key rotate` did not accept `--reanchor-file` |
| 2026-06-28 | CRY-6 re-anchor | `cargo test -p cortex-cli receipt_key_rotate_writes_verifiable_reanchor_record` | passed |
| 2026-06-28 | CRY-6 re-anchor regression | `cargo test -p cortex-cli receipt_key_generate_export_and_rotate_preserves_dual_trust` | passed |
| 2026-06-28 | CRY-6 re-anchor | `make key-management-check` | passed with re-anchor writer/verifier coverage |
| 2026-06-28 | CRY-6 re-anchor claims | `make crypto-claims-honesty-check` | passed |
| 2026-06-28 | CRY-6 re-anchor regression | `make crypto-foundation-check` | passed |
| 2026-06-28 | database-instance identity pre-fix | `cargo test -p cortex-server configured_receipts_use_durable_database_instance_id_across_tenants` | failed: configured receipt headers used `local:default` and `local:alpha` |
| 2026-06-28 | database-instance identity | `cargo test -p cortex-server database_instance_id --all-features` | passed |
| 2026-06-28 | database-instance identity regression | `cargo test -p cortex-server emits_signed_accountability_receipt --all-features` | passed |
| 2026-06-28 | database-instance identity | `python3 -m py_compile scripts/database_instance_identity_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | database-instance identity | `make database-instance-identity-check` | passed |
| 2026-06-28 | database-instance identity regression | `make crypto-foundation-check` | passed |
| 2026-06-28 | database-instance identity final | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | database-instance identity final | `git diff --check` | passed |
| 2026-06-28 | workspace after database-instance identity | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after database-instance identity | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | physical skipped-segment parity boundary pre-fix | `python3 scripts/segment_pruning_parity_boundary_check.py --root "." --report target/segment-pruning-parity-boundary/manual-report.json` | failed: boundary docs/make markers were missing |
| 2026-06-28 | physical skipped-segment parity boundary | `python3 -m py_compile scripts/segment_pruning_parity_boundary_check.py scripts/context_access_decision_capture_check.py` | passed |
| 2026-06-28 | physical skipped-segment parity boundary | `make segment-pruning-parity-boundary-check` | passed |
| 2026-06-28 | physical skipped-segment parity boundary regression | `python3 scripts/context_access_decision_capture_check.py --root "." --report target/context-access-decision-capture/manual-after-boundary-report.json` | passed; AR-6 gate remains scoped to captured access evidence |
| 2026-06-28 | physical skipped-segment parity boundary final | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after physical skipped-segment parity boundary | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after physical skipped-segment parity boundary | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | ann-scope parity pre-fix | `cargo test -p cortex-engine search::database::tests::persisted_search_bound_plan_allowed_set_filters_status_and_where --all-features` | failed: bound-plan persisted search entrypoint did not exist |
| 2026-06-28 | ann-scope parity targeted | `cargo test -p cortex-engine search::database::tests::persisted_search_bound_plan_allowed_set_filters_status_and_where --all-features` | passed |
| 2026-06-28 | ann-scope parity prewire | `python3 scripts/ann_scope_parity_check.py --root "." --report target/ann-scope-parity/manual-prewire-report.json` | failed: make/report/status markers were missing |
| 2026-06-28 | ann-scope parity scripts | `python3 -m py_compile scripts/ann_scope_parity_check.py scripts/segment_pruning_parity_boundary_check.py` | passed |
| 2026-06-28 | ann-scope parity | `make ann-scope-parity-check` | passed |
| 2026-06-28 | ann-scope parity boundary regression | `make segment-pruning-parity-boundary-check` | passed |
| 2026-06-28 | ann-scope parity final | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | ann-scope parity final | `git diff --check` | passed |
| 2026-06-28 | workspace after ann-scope parity | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after ann-scope parity | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | ann-scope parity public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | ann sparse-scope recall pre-fix | `cargo test -p cortex-engine sparse_allowed_set_routes_to_exact_before_hnsw_budget --all-features` | failed: `SparseAllowedSet` report reason and sparse threshold were missing |
| 2026-06-28 | ann sparse-scope recall targeted | `cargo test -p cortex-engine sparse_allowed_set_routes_to_exact_before_hnsw_budget --all-features` | passed |
| 2026-06-28 | ann sparse-scope recall prewire | `python3 scripts/ann_sparse_scope_recall_check.py --root "." --report target/ann-sparse-scope-recall/manual-prewire-report.json` | failed: make/report/status markers were missing |
| 2026-06-28 | ann sparse-scope recall scripts | `python3 -m py_compile scripts/ann_sparse_scope_recall_check.py scripts/ann_scope_parity_check.py` | passed |
| 2026-06-28 | ann sparse-scope recall | `make ann-sparse-scope-recall-check` | passed |
| 2026-06-28 | ann sparse-scope recall regression | `make ann-scope-parity-check` | passed |
| 2026-06-28 | ann sparse-scope recall formatting | `cargo fmt --check` | passed after applying rustfmt |
| 2026-06-28 | workspace after ann sparse-scope recall first pass | `cargo test --workspace --all-features` | failed: sparse exact route intercepted dense budget-violation reports; fixed by requiring allowed set to be sparse relative to graph nodes |
| 2026-06-28 | ann sparse-scope recall density regression | `cargo test -p cortex-engine --lib search::ann --all-features` | passed |
| 2026-06-28 | workspace after ann sparse-scope recall | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after ann sparse-scope recall | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | ann sparse-scope recall public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | ann sparse-scope recall API contract sanity | `make openapi-contract-check` | passed |
| 2026-06-28 | ann sparse-scope recall final | `git diff --check` | passed |
| 2026-06-28 | scope-leak bench prewire | `python3 scripts/scope_leak_bench_check.py --root "." --report target/context-pack-quality/scope-leak-bench-prewire.json` | failed: Rust test, make/report/status markers were missing |
| 2026-06-28 | scope-leak bench targeted pre-fix | `cargo test -p cortex-engine --test scope_leak_bench --all-features` | failed: in-memory ANN reports `NoPersistedSegments`; budget exhaustion is asserted only after checkpoint/compact |
| 2026-06-28 | scope-leak bench targeted | `cargo test -p cortex-engine --test scope_leak_bench --all-features` | passed |
| 2026-06-28 | scope-leak bench split guard | `make scope-leak-bench-check` | failed: support-module split missed the `AgentView` import in the integration test; fixed before final verification |
| 2026-06-28 | scope-leak bench scripts | `python3 -m py_compile scripts/scope_leak_bench_check.py scripts/context_pack_explain_v2_check.py scripts/context_pack_private_scope_check.py` | passed |
| 2026-06-28 | scope-leak bench final targeted | `make scope-leak-bench-check` | passed |
| 2026-06-28 | context-pack quality aggregate after scope-leak bench | `make context-pack-quality-check` | passed |
| 2026-06-28 | workspace after scope-leak bench | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after scope-leak bench | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after scope-leak bench | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | fail-closed invariant model prewire | `python3 scripts/fail_closed_invariant_model_check.py --root "." --report target/fail-closed-invariant-model/prewire-report.json` | failed: make/report/status markers and pinned `model_hash` were missing |
| 2026-06-28 | fail-closed invariant AQL property | `cargo test -p cortex-aql --test fail_closed_invariant_model` | passed |
| 2026-06-28 | fail-closed invariant engine pre-fix | `cargo test -p cortex-engine --test fail_closed_invariant_model fail_closed_invariant_model_hash_is_stable --all-features -- --nocapture` | failed: integration test attempted private `bind_aql_cached`; persisted-path property moved into a crate unit test |
| 2026-06-28 | fail-closed invariant model hash pre-pin | `cargo test -p cortex-engine --test fail_closed_invariant_model fail_closed_invariant_model_hash_is_stable --all-features -- --nocapture` | failed: computed `model_hash=cb0e81f8fd07d20769e27ea3f8bd3b4e7459e72504a939b393470d433df40e79`; pinned constant was empty |
| 2026-06-28 | fail-closed invariant engine property | `cargo test -p cortex-engine fail_closed_invariant_model_tests::persisted_ann_and_lexical_paths_respect_fail_closed_model --all-features` | passed |
| 2026-06-28 | fail-closed invariant final targeted | `make fail-closed-invariant-model-check` | passed |
| 2026-06-28 | fail-closed invariant scripts | `python3 -m py_compile scripts/fail_closed_invariant_model_check.py scripts/scope_leak_bench_check.py` | passed |
| 2026-06-28 | workspace after fail-closed invariant model | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after fail-closed invariant model | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after fail-closed invariant model | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | fail-closed invariant public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | fail-closed end-to-end prewire | `python3 scripts/fail_closed_end_to_end_check.py --root "." --report target/fail-closed-end-to-end/prewire-report.json` | failed: aggregate make target, report var, `.PHONY`, beta suite, and beta artifact markers were missing |
| 2026-06-28 | fail-closed end-to-end scripts | `python3 -m py_compile scripts/fail_closed_end_to_end_check.py scripts/beta_release_bundle.py` | passed |
| 2026-06-28 | fail-closed end-to-end static report | `python3 scripts/fail_closed_end_to_end_check.py --root "." --report target/fail-closed-end-to-end/static-report.json` | passed |
| 2026-06-28 | fail-closed end-to-end beta bundle self-test | `python3 scripts/beta_release_bundle.py --self-test` | passed |
| 2026-06-28 | fail-closed end-to-end final targeted | `make fail-closed-end-to-end-check` | passed |
| 2026-06-28 | fail-closed end-to-end post-status report | `python3 scripts/fail_closed_end_to_end_check.py --root "." --report target/fail-closed-end-to-end/post-status-report.json` | passed |
| 2026-06-28 | fail-closed end-to-end file-size sanity | `wc -l scripts/fail_closed_end_to_end_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md mk/core-retrieval-context.mk scripts/beta_release_bundle.py` | passed: new script is 164 lines |
| 2026-06-28 | workspace after fail-closed end-to-end | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after fail-closed end-to-end | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after fail-closed end-to-end | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | workspace after fail-closed end-to-end | `git diff --check` | passed |
| 2026-06-28 | Order 6 crypto-hardening reconciliation | `make crypto-foundation-check` | passed |
| 2026-06-28 | DV2 numeric normalization prewire | `python3 scripts/verify_numeric_normalization_check.py --root "." --report target/verification-quality/numeric-normalization-prewire.json` | failed: `CurrencyMismatch`, unit aliases, `.is_conflict()`, make/report/status markers were missing |
| 2026-06-28 | DV2 numeric normalization scripts | `python3 -m py_compile scripts/verify_numeric_normalization_check.py scripts/conflict_normalization_check.py` | passed |
| 2026-06-28 | DV2 numeric normalization static report | `python3 scripts/verify_numeric_normalization_check.py --root "." --report target/verification-quality/numeric-normalization-static.json` | passed |
| 2026-06-28 | DV2 numeric normalization targeted | `cargo test -p cortex-engine numeric --all-features` | passed |
| 2026-06-28 | DV2 numeric normalization final targeted | `make verify-numeric-normalization-check` | passed |
| 2026-06-28 | DV2 conflict normalization regression | `make conflict-normalization-check` | passed |
| 2026-06-28 | DV2 verification quality aggregate | `make verification-quality-check` | passed |
| 2026-06-28 | DV2 numeric normalization file-size sanity | `wc -l scripts/verify_numeric_normalization_check.py crates/cortex-engine/src/verification/numeric/value.rs crates/cortex-engine/src/verification/numeric/parse.rs crates/cortex-engine/src/verification/numeric/tests.rs` | passed: new script is 132 lines |
| 2026-06-28 | DV2 numeric normalization formatting pre-fix | `cargo fmt --check` | failed: rustfmt wanted the `ml` unit alias match arm on one line |
| 2026-06-28 | DV2 numeric normalization formatting | `cargo fmt` | passed |
| 2026-06-28 | DV2 numeric normalization final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV2 numeric normalization final static report | `python3 scripts/verify_numeric_normalization_check.py --root "." --report target/verification-quality/numeric-normalization-post-fmt.json` | passed |
| 2026-06-28 | workspace after DV2 numeric normalization | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | DV2 numeric normalization public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | workspace after DV2 numeric normalization | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV1 context/verify agreement prewire | `python3 scripts/context_pack_conflict_visibility_check.py --root "." --report target/context-pack-quality/conflict-visibility-dv1-prewire.json` | failed: normalized-equal and ContextPack/VERIFY agreement test markers were missing |
| 2026-06-28 | DV1 context/verify agreement scripts | `python3 -m py_compile scripts/context_pack_conflict_visibility_check.py` | passed |
| 2026-06-28 | DV1 context/verify agreement targeted | `cargo test -p cortex-engine --test context_pack_conflict_visibility --all-features` | passed |
| 2026-06-28 | DV1 context/verify agreement static report | `python3 scripts/context_pack_conflict_visibility_check.py --root "." --report target/context-pack-quality/conflict-visibility-dv1-static.json` | passed |
| 2026-06-28 | DV1 context/verify agreement final targeted | `make context-pack-conflict-visibility-check` | passed |
| 2026-06-28 | DV1 conflict normalization regression | `make conflict-normalization-check` | passed |
| 2026-06-28 | DV1 context-pack aggregate | `make context-pack-quality-check` | passed |
| 2026-06-28 | DV1 context/verify agreement file-size sanity | `wc -l crates/cortex-engine/tests/context_pack_conflict_visibility.rs scripts/context_pack_conflict_visibility_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: modified Rust test remains 251 lines |
| 2026-06-28 | DV1 context/verify agreement formatting pre-fix | `cargo fmt --check` | failed: rustfmt wanted a one-line `map` closure in `context_pack_conflict_visibility.rs` |
| 2026-06-28 | DV1 context/verify agreement formatting | `cargo fmt` | passed |
| 2026-06-28 | DV1 context/verify agreement final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV1 context/verify agreement final targeted | `make context-pack-conflict-visibility-check` | passed after rustfmt |
| 2026-06-28 | DV1 context/verify agreement final static report | `python3 scripts/context_pack_conflict_visibility_check.py --root "." --report target/context-pack-quality/conflict-visibility-dv1-final-status.json` | passed |
| 2026-06-28 | DV1 context/verify agreement whitespace | `git diff --check` | passed |
| 2026-06-28 | workspace after DV1 context/verify agreement | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV1 context/verify agreement | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV3 multivalue extraction prewire | `python3 scripts/verify_multivalue_extraction_check.py --root "." --report target/verification-quality/multivalue-extraction-prewire.json` | failed: multi-record storage, contextual extraction, tests, make/report markers were missing and `single_numeric_value` was still present |
| 2026-06-28 | DV3 multivalue extraction scripts | `python3 -m py_compile scripts/verify_multivalue_extraction_check.py scripts/verify_numeric_normalization_check.py` | passed |
| 2026-06-28 | DV3 multivalue extraction first targeted | `cargo test -p cortex-engine numeric --all-features` | failed: contextual segment splitting used `.`, which broke decimal `1.4B` into separate numeric fragments |
| 2026-06-28 | DV3 multivalue extraction VERIFY regression | `cargo test -p cortex-engine --test verification_guards verify_fact_detects_conflict_from_multivalue_evidence_body --all-features` | passed |
| 2026-06-28 | DV3 multivalue extraction static report | `python3 scripts/verify_multivalue_extraction_check.py --root "." --report target/verification-quality/multivalue-extraction-static-after-fix.json` | passed |
| 2026-06-28 | DV3 multivalue extraction targeted | `cargo test -p cortex-engine numeric --all-features` | passed after removing `.` from contextual split |
| 2026-06-28 | DV3 multivalue extraction final targeted | `make verify-multivalue-extraction-check` | passed |
| 2026-06-28 | DV3 verification quality aggregate | `make verification-quality-check` | passed |
| 2026-06-28 | DV3 multivalue extraction file-size sanity | `wc -l scripts/verify_multivalue_extraction_check.py crates/cortex-engine/src/verification/numeric/fact_claim.rs crates/cortex-engine/src/verification/numeric/fact_claim/tests.rs crates/cortex-engine/tests/verification_guards.rs crates/cortex-engine/src/verification/conflict_index/store.rs` | passed: new script is 114 lines; touched legacy files remain oversized |
| 2026-06-28 | DV3 multivalue extraction formatting pre-fix | `cargo fmt --check` | failed: rustfmt wanted a wrapped `metadata.source` chain |
| 2026-06-28 | DV3 multivalue extraction formatting | `cargo fmt` | passed |
| 2026-06-28 | DV3 multivalue extraction final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV3 multivalue extraction post-fmt targeted | `make verify-multivalue-extraction-check` | passed |
| 2026-06-28 | DV3 multivalue extraction post-fmt static report | `python3 scripts/verify_multivalue_extraction_check.py --root "." --report target/verification-quality/multivalue-extraction-post-fmt.json` | passed |
| 2026-06-28 | DV3 multivalue extraction whitespace | `git diff --check` | passed |
| 2026-06-28 | workspace after DV3 multivalue extraction | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV3 multivalue extraction | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV3 multivalue extraction public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | DV4 temporal conflict prewire | `python3 scripts/verify_temporal_conflict_check.py --root "." --report target/verification-quality/temporal-conflict-prewire.json` | failed: temporal validity storage, overlap helpers, tests, make/report markers were missing and both temporal early-returns were still present |
| 2026-06-28 | DV4 temporal conflict script | `python3 -m py_compile scripts/verify_temporal_conflict_check.py` | passed |
| 2026-06-28 | DV4 temporal conflict first targeted | `cargo test -p cortex-engine numeric --all-features` | failed: `metadata.project` was partially moved before temporal validity extraction |
| 2026-06-28 | DV4 temporal conflict targeted | `cargo test -p cortex-engine numeric --all-features` | passed after moving temporal validity extraction before `metadata.project` ownership transfer |
| 2026-06-28 | DV4 temporal conflict static report | `python3 scripts/verify_temporal_conflict_check.py --root "." --report target/verification-quality/temporal-conflict-after-code.json` | passed |
| 2026-06-28 | DV4 temporal conflict final targeted | `make verify-temporal-conflict-check` | passed |
| 2026-06-28 | DV4 temporal conflict formatting pre-fix | `cargo fmt --check` | failed: rustfmt wanted import ordering and line wrapping |
| 2026-06-28 | DV4 temporal conflict formatting | `cargo fmt` | passed |
| 2026-06-28 | DV4 temporal conflict final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV4 temporal conflict post-fmt static report | `python3 scripts/verify_temporal_conflict_check.py --root "." --report target/verification-quality/temporal-conflict-post-fmt.json` | passed |
| 2026-06-28 | DV4 verification quality aggregate | `make verification-quality-check` | passed |
| 2026-06-28 | workspace after DV4 temporal conflict first full test | `cargo test --workspace --all-features` | failed: `context_verify_quality` saw one extra contradiction because `value=1200000000` plus `currency=KZT` was indexed without currency and compared against the year `2025` |
| 2026-06-28 | DV4 explicit currency regression | `cargo test -p cortex-engine --test context_verify_quality --all-features` | passed after applying explicit `currency=` metadata to raw `value=` numerics |
| 2026-06-28 | DV4 explicit currency numeric regression | `cargo test -p cortex-engine numeric --all-features` | passed with `explicit_currency_field_applies_to_numeric_value` |
| 2026-06-28 | DV4 temporal conflict strengthened static report | `python3 scripts/verify_temporal_conflict_check.py --root "." --report target/verification-quality/temporal-conflict-after-currency-fix.json` | passed |
| 2026-06-28 | workspace after DV4 temporal conflict second full test | `cargo test --workspace --all-features` | failed: determinism snapshot showed the same cell as both support and contradiction because `contradicts=` was indexed as a positive numeric claim |
| 2026-06-28 | DV4 contradicts marker numeric regression | `cargo test -p cortex-engine numeric --all-features` | passed with `contextual_numeric_values_ignore_contradicts_marker` |
| 2026-06-28 | DV4 determinism regression | `cargo test -p cortex-engine --test determinism verification_report_output_is_repeatable_and_snapshotted --all-features` | passed after skipping `contradicts=` marker segments in contextual numeric extraction |
| 2026-06-28 | DV4 temporal conflict final targeted | `make verify-temporal-conflict-check` | passed with explicit currency and `contradicts=` marker guards |
| 2026-06-28 | DV4 verification quality final aggregate | `make verification-quality-check` | passed |
| 2026-06-28 | workspace after DV4 temporal conflict | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV4 temporal conflict | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV4 temporal conflict public API sanity | `make engine-public-api-freeze-check` | passed |
| 2026-06-28 | DV4 temporal conflict whitespace | `git diff --check` | passed |
| 2026-06-28 | DV4 temporal conflict file-size sanity | `wc -l scripts/verify_temporal_conflict_check.py crates/cortex-engine/src/verification/numeric/fact_claim.rs crates/cortex-engine/src/verification/numeric/fact_claim/tests.rs crates/cortex-engine/src/verification/conflict_index/store.rs crates/cortex-engine/tests/verification_guards.rs crates/cortex-engine/tests/verification_conflict_numeric.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new script is 127 lines; touched legacy Rust files remain oversized |
| 2026-06-28 | DV5 citation conflict script | `python3 -m py_compile scripts/verify_citation_conflict_check.py` | passed |
| 2026-06-28 | DV5 citation conflict prewire | `python3 scripts/verify_citation_conflict_check.py --root "." --report target/verification-quality/citation-conflict-prewire.json` | failed as expected: typed kind, same-source helpers, conflict-index citation marker, API field, and tests were missing |
| 2026-06-28 | DV5 citation conflict first targeted | `make verify-citation-conflict-check` | failed: `VerificationNumericConflictKind` was not re-exported from the engine verification/root modules |
| 2026-06-28 | DV5 citation conflict targeted | `make verify-citation-conflict-check` | passed after re-exporting the conflict kind and adding VERIFY/conflict-index regression tests |
| 2026-06-28 | DV5 citation conflict formatting pre-fix | `cargo fmt --check` | failed: rustfmt wanted line wrapping in source-ref fallback and the new test helper |
| 2026-06-28 | DV5 citation conflict formatting | `cargo fmt` | passed |
| 2026-06-28 | DV5 citation conflict initial fmt check | `cargo fmt --check` | passed |
| 2026-06-28 | DV5 citation conflict engine guards | `cargo test -p cortex-engine --test verification_guards --all-features` | passed |
| 2026-06-28 | DV5 citation conflict index | `cargo test -p cortex-engine --test verification_conflict_numeric --all-features` | passed |
| 2026-06-28 | DV5 citation conflict numeric suite | `cargo test -p cortex-engine numeric --all-features` | passed |
| 2026-06-28 | DV5 citation conflict SDK helper | `cargo test -p cortexdb-sdk verification_report_helpers_surface_result_and_conflicts` | passed after using the correct package name (`cortexdb-sdk`) |
| 2026-06-28 | DV5 citation conflict server snapshots first run | `cargo test -p cortex-server response_snapshot_tests --all-features` | failed: verification response snapshot was missing additive `numeric_conflicts[].kind` |
| 2026-06-28 | DV5 citation conflict server snapshots | `cargo test -p cortex-server response_snapshot_tests --all-features` | passed after updating the verification response snapshot |
| 2026-06-28 | DV5 verification quality aggregate | `make verification-quality-check` | passed with `verify-citation-conflict-check` wired after DV4 |
| 2026-06-28 | DV5 OpenAPI contract first run | `make openapi-contract-check` | failed: generated Python/TypeScript OpenAPI SDK types were stale after adding `numeric_conflicts[].kind` |
| 2026-06-28 | DV5 generated OpenAPI SDK types | `python3 scripts/generate_openapi_sdk_types.py` | passed |
| 2026-06-28 | DV5 OpenAPI contract | `make openapi-contract-check` | passed |
| 2026-06-28 | workspace after DV5 citation conflict | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV5 citation conflict | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV5 temporal-kind correction first run | `make verify-temporal-conflict-check` | failed: an existing early numeric mismatch kept `kind="numeric"` instead of upgrading to `kind="temporal"` |
| 2026-06-28 | DV5 temporal-kind correction targeted | `make verify-temporal-conflict-check` | passed after upgrading existing conflicts for any non-numeric kind |
| 2026-06-28 | DV5 citation conflict final targeted | `make verify-citation-conflict-check` | passed after the temporal-kind correction |
| 2026-06-28 | DV5 verification quality final aggregate | `make verification-quality-check` | passed with DV2-DV5 gates wired in order |
| 2026-06-28 | DV5 OpenAPI final contract | `make openapi-contract-check` | passed |
| 2026-06-28 | workspace after DV5 temporal/citation conflict | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV5 temporal/citation conflict | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV5 citation conflict final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV5 citation conflict final static report | `python3 scripts/verify_citation_conflict_check.py --root "." --report target/verification-quality/citation-conflict-final.json` | passed |
| 2026-06-28 | DV5 citation conflict final whitespace | `git diff --check` | passed |
| 2026-06-28 | DV6 determinism script | `python3 -m py_compile scripts/verify_determinism_check.py` | passed |
| 2026-06-28 | DV6 determinism targeted | `make verify-determinism-check` | passed with canonical VERIFY conflict bytes repeatable, checkpoint-stable, `kind`-inclusive, and clock-free |
| 2026-06-28 | DV6 engine determinism lane | `make engine-determinism-check` | passed with `verify-determinism-check` wired before the existing static guard |
| 2026-06-28 | DV6 canonical serialization | `make canonical-serialization-check` | passed |
| 2026-06-28 | DV6 verification quality aggregate | `make verification-quality-check` | passed with DV2-DV6 gates wired in order |
| 2026-06-28 | workspace after DV6 verification determinism | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV6 verification determinism | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV6 verification determinism final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV6 verification determinism final static report | `python3 scripts/verify_determinism_check.py --root "." --report target/verification-quality/determinism-final.json` | passed |
| 2026-06-28 | DV6 verification determinism final whitespace | `git diff --check` | passed |
| 2026-06-28 | DV6 verification determinism file-size sanity | `wc -l scripts/verify_determinism_check.py crates/cortex-engine/tests/determinism.rs crates/cortex-engine/src/canonical.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new script is 145 lines; determinism test is 296 lines; touched legacy canonical/status docs remain oversized |
| 2026-06-28 | DV7/DV8 script compile | `python3 -m py_compile scripts/verify_conflict_recall_check.py scripts/verify_docs_claims_check.py` | passed |
| 2026-06-28 | DV7 conflict recall Rust benchmark | `cargo test -p cortex-engine --test verification_conflict_recall --all-features` | passed |
| 2026-06-28 | DV7 conflict recall first make target | `make verify-conflict-recall-check` | failed: Rust integration test cwd wrote the measured report under `crates/cortex-engine/target/...` while Python expected workspace `target/...` |
| 2026-06-28 | DV7 conflict recall make target | `make verify-conflict-recall-check` | passed after using `$(CURDIR)/$(VERIFY_CONFLICT_RECALL_REPORT)` for the Rust report path |
| 2026-06-28 | DV8 docs claims gate | `make docs-claims-check` | passed with `VERIFY_FACT.md` numbers matched to the DV7 report |
| 2026-06-28 | DV7-DV8 verification quality aggregate | `make verification-quality-check` | passed with DV2-DV8 gates wired in order |
| 2026-06-28 | DV7-DV8 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | DV7-DV8 final whitespace | `git diff --check` | passed |
| 2026-06-28 | workspace after DV7-DV8 conflict recall | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after DV7-DV8 conflict recall | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | DV8 public claims sanity | `make public-claims-check` | passed |
| 2026-06-28 | RANK-1 formatting | `cargo fmt` | passed |
| 2026-06-28 | RANK-1 frozen weights first gate | `make ranking-frozen-weights-check` | failed: gate found remaining bare candidate-depth/Q16 markers in search ranking consumers |
| 2026-06-28 | RANK-1 frozen weights gate | `make ranking-frozen-weights-check` | passed after moving hybrid candidate depth and metadata recency Q16 denominator to the frozen module |
| 2026-06-28 | RANK-1 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after RANK-1 first run | `cargo test --workspace --all-features` | failed once in unrelated `tenant_quota_50_tenant_load_smoke`: third cell returned 200 instead of quota rejection |
| 2026-06-28 | RANK-1 quota smoke rerun | `cargo test -p cortex-server tests::security_quota_tests::tenant_quota_50_tenant_load_smoke --all-features -- --exact --nocapture` | passed |
| 2026-06-28 | workspace after RANK-1 rerun | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after RANK-1 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | RANK-1 gate split compile | `python3 -m py_compile scripts/ranking_frozen_weights_check.py scripts/ranking_frozen_weights_gate_spec.py` | passed |
| 2026-06-28 | RANK-1 gate split file-size sanity | `wc -l scripts/ranking_frozen_weights_check.py scripts/ranking_frozen_weights_gate_spec.py crates/cortex-engine/src/search/frozen_weights.rs crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json` | passed: new files are 220, 154, 267, and 76 lines respectively |
| 2026-06-28 | RANK-1 final frozen weights gate | `make ranking-frozen-weights-check` | passed after splitting gate marker data into a compact spec module |
| 2026-06-28 | RANK-1 final fmt recheck | `cargo fmt --check` | passed |
| 2026-06-28 | RANK-1 final whitespace | `git diff --check` | passed |
| 2026-06-28 | RANK-2 script compile | `python3 -m py_compile scripts/learned_ranking_calibration_check.py scripts/ranking_weights_drift_check.py scripts/ranking_frozen_weights_check.py scripts/ranking_frozen_weights_gate_spec.py` | passed |
| 2026-06-28 | RANK-2 file-size sanity | `wc -l scripts/learned_ranking_calibration_check.py scripts/ranking_weights_drift_check.py scripts/ranking_frozen_weights_check.py scripts/ranking_frozen_weights_gate_spec.py crates/cortex-engine/src/search/frozen_weights.rs crates/cortex-engine/fixtures/ranking_frozen_weights_v1.json` | passed: files are 298, 138, 220, 154, 267, and 76 lines respectively |
| 2026-06-28 | RANK-2 calibration compatibility | `make learned-ranking-calibration-check` | passed with heldout baseline MRR 6250, learned MRR 10000, lift 3750 bps, win-rate 75% |
| 2026-06-28 | RANK-2 drift gate | `make ranking-weights-drift-check` | passed: generated artifact byte-identical to checked-in artifact and module check green; content hash `d67c97a93c97d34f501ecdb7da103faec92af38d62925514bfcb764e0d5fe947` |
| 2026-06-28 | RANK-2 frozen weights regression | `make ranking-frozen-weights-check` | passed after trainer-selected profile updates |
| 2026-06-28 | RANK-2 fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after RANK-2 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after RANK-2 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | RANK-2 whitespace | `git diff --check` | passed |
| 2026-06-28 | RANK-3 initial fmt | `cargo fmt --check` | failed: new `ranking_learned_lift.rs` needed rustfmt wrapping |
| 2026-06-28 | RANK-3 calibration compatibility | `make learned-ranking-calibration-check` | passed with heldout baseline MRR 6250, learned MRR 10000, lift 3750 bps, win-rate 75% |
| 2026-06-28 | RANK-3 drift compatibility | `make ranking-weights-drift-check` | passed after fixture question text updates; generated artifact remains byte-identical to checked-in artifact |
| 2026-06-28 | RANK-3 first Rust lift gate | `make ranking-learned-lift-check` | failed: engine-side win-rate was 50% because the semantic heldout query did not match the engine semantic intent signals |
| 2026-06-28 | RANK-3 final Rust lift gate | `make ranking-learned-lift-check` | passed with heldout baseline MRR 6250, learned MRR 10000, lift 3750 bps, win-rate 75%, and report `target/ranking/learned-lift/report.json` |
| 2026-06-28 | RANK-3 file-size sanity | `wc -l crates/cortex-engine/tests/ranking_learned_lift.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new Rust test is 177 lines |
| 2026-06-28 | RANK-3 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after RANK-3 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after RANK-3 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | RANK-3 whitespace | `git diff --check` | passed |
| 2026-06-28 | REPRO-2 initial fmt | `cargo fmt --check` | failed on new determinism hash files before targeted rustfmt |
| 2026-06-28 | REPRO-2 helper tests | `cargo test -p cortex-engine determinism_hash --all-features` | passed |
| 2026-06-28 | REPRO-2 binding gate | `make weights-version-binding-check` | passed with report `target/determinism-hash/binding-report.json` |
| 2026-06-28 | REPRO-2 receipt body regression | `cargo test -p cortex-engine accountability_receipt_body --all-features` | passed, including frozen weights hash mutation coverage |
| 2026-06-28 | REPRO-2 standalone verifier regression | `cargo test -p cortex-receipt-verify --all-features` | passed |
| 2026-06-28 | REPRO-2 receipt determinism gate | `make accountability-receipt-determinism-check` | passed |
| 2026-06-28 | REPRO-2 verifier first gate | `make accountability-receipt-verify-check` | failed once because `fixtures/accountability_receipt/verify_input.golden.json` still had the old determinism input shape/signature |
| 2026-06-28 | REPRO-2 verifier final gate | `make accountability-receipt-verify-check` | passed after regenerating the golden determinism input, determinism hash, and signature |
| 2026-06-28 | REPRO-2 tamper gate | `make accountability-receipt-tamper-check` | passed against the updated verifier fixture |
| 2026-06-28 | REPRO-2 file-size sanity | `wc -l crates/cortex-engine/src/determinism_hash.rs crates/cortex-engine/tests/weights_version_binding.rs crates/cortex-engine/src/accountability/receipt.rs crates/cortex-receipt-verify/src/receipt_hash.rs crates/cortex-receipt-verify/src/tests.rs` | passed: touched Rust files are 102, 85, 201, 87, and 193 lines |
| 2026-06-28 | REPRO-2 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after REPRO-2 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after REPRO-2 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | REPRO-2 whitespace | `git diff --check` | passed |
| 2026-06-28 | REPRO-3 initial fmt | `cargo fmt --check` | failed: new pack determinism fixture/test needed rustfmt wrapping |
| 2026-06-28 | REPRO-3 determinism hash gate | `make pack-determinism-hash-check` | passed with report `target/pack-determinism/report.json` |
| 2026-06-28 | REPRO-3 file-size sanity | `wc -l crates/cortex-engine/src/bin/pack_determinism_hash_fixture.rs crates/cortex-engine/tests/pack_determinism_hash.rs crates/cortex-engine/src/context/receipt_evidence.rs` | passed: touched Rust files are 145, 105, and 75 lines |
| 2026-06-28 | REPRO-3 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after REPRO-3 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after REPRO-3 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | REPRO-3 whitespace | `git diff --check` | passed |
| 2026-06-28 | RANK-4 initial fmt | `cargo fmt --check` | passed |
| 2026-06-28 | RANK-4 explain faithfulness gate | `make ranking-explain-faithfulness-check` | passed with `ranking-frozen-weights-check` dependency and report `target/ranking/explain-faithfulness/report.json` |
| 2026-06-28 | RANK-4 file-size sanity | `wc -l crates/cortex-engine/tests/ranking_explain_faithfulness.rs mk/core.mk mk/vars-core.mk mk/phony.mk` | passed: new Rust test is 187 lines |
| 2026-06-28 | RANK-4 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after RANK-4 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after RANK-4 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | RANK-4 whitespace | `git diff --check` | passed |
| 2026-06-28 | RANK-5 initial fmt | `cargo fmt --check` | failed: new grounding/canonical imports needed rustfmt wrapping |
| 2026-06-28 | RANK-5 completeness gate | `make pack-completeness-signal-check` | passed with `ann-budget-disclosure-check` dependency and report `target/pack-completeness/report.json` |
| 2026-06-28 | RANK-5 ContextPack schema contract | `make context-pack-schema-contract-check` | passed |
| 2026-06-28 | RANK-5 OpenAPI contract | `make openapi-contract-check` | passed with existing OpenAPI coverage warnings but live response validation, taxonomy, SDK generated types, and codegen-control checks green |
| 2026-06-28 | RANK-5 generated SDK type check | `python3 scripts/generate_openapi_sdk_types.py --check` | passed |
| 2026-06-28 | workspace after RANK-5 first run | `cargo test --workspace --all-features` | failed: three Rust SDK `ContextPackV1` test literals were missing explicit `grounding_report: None` |
| 2026-06-28 | RANK-5 file-size sanity | `wc -l crates/cortex-engine/tests/pack_completeness_signal.rs crates/cortex-engine/src/context/mod.rs crates/cortex-engine/src/context/export/json_export.rs crates/cortex-engine/src/canonical.rs crates/cortex-server/src/responses/context.rs crates/cortex-sdk/src/types/context.rs crates/cortex-sdk/src/context_pack_tests.rs sdk/python/_cortexdb_client/model_types/context.py sdk/typescript/cortexdb-client/types/context.ts docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new test is 105 lines; touched typed response/model files are under 300 lines; `canonical.rs` and status doc remain pre-existing oversized files |
| 2026-06-28 | RANK-5 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after RANK-5 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after RANK-5 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | RANK-5 final contract replay | `make pack-completeness-signal-check && make context-pack-schema-contract-check && make openapi-contract-check && python3 scripts/generate_openapi_sdk_types.py --check` | passed |
| 2026-06-28 | RANK-5 whitespace | `git diff --check` | passed |
| 2026-06-28 | RANK-5 docs precision recheck | `make context-pack-schema-contract-check && git diff --check` | passed |
| 2026-06-28 | SPEC-1 script compile | `python3 -m py_compile scripts/gce_spec_doc_check.py` | passed |
| 2026-06-28 | SPEC-1 first doc gate | `make gce-spec-doc-check` | failed: the spec was missing exact source markers for VERIFY type names, `BitmapOp::And`, `where_clause`, `cortex-receipt-verify`, `blake3-256`, and `ed25519` |
| 2026-06-28 | SPEC-1 doc gate | `make gce-spec-doc-check` | passed with report `target/gce-spec/doc-report.json` |
| 2026-06-28 | SPEC-1 file-size sanity | `wc -l docs/spec/GCE_CONTRACT.md scripts/gce_spec_doc_check.py mk/core.mk mk/vars-core.mk mk/phony.mk` | passed: new doc is 234 lines and new script is 228 lines; touched make files include pre-existing large target lists |
| 2026-06-28 | SPEC-1 fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after SPEC-1 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after SPEC-1 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | SPEC-1 whitespace | `git diff --check` | passed |
| 2026-06-28 | SPEC-3 script compile | `python3 -m py_compile scripts/receipt_threat_model_check.py` | passed |
| 2026-06-28 | SPEC-3 threat-model gate | `make receipt-threat-model-check` | passed with report `target/gce-spec/receipt-threat-model-report.json` |
| 2026-06-28 | SPEC-3 file-size sanity | `wc -l docs/spec/RECEIPT_VERIFIER.md docs/spec/GCE_CONTRACT.md scripts/receipt_threat_model_check.py scripts/gce_spec_doc_check.py mk/core.mk mk/vars-core.mk mk/phony.mk` | passed: new verifier spec is 141 lines and new script is 215 lines; touched make files include pre-existing large target lists |
| 2026-06-28 | SPEC-3 fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after SPEC-3 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after SPEC-3 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | SPEC-3 whitespace | `git diff --check` | passed |
| 2026-06-28 | CONF-1 script compile | `python3 -m py_compile scripts/aab_conformance_check.py` | passed |
| 2026-06-28 | CONF-1 direct report | `python3 scripts/aab_conformance_check.py --root "." --fixture "fixtures/gce_conformance/thin_wrapper_reference.json" --report "target/gce-conformance/dev-report.json"` | passed |
| 2026-06-28 | CONF-1 conformance gate | `make aab-conformance-check` | passed with report `target/gce-conformance/report.json` |
| 2026-06-28 | CONF-1 file-size sanity | `wc -l docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/spec/GCE_CONFORMANCE.md scripts/gce_spec_doc_check.py scripts/receipt_threat_model_check.py scripts/aab_conformance_check.py fixtures/gce_conformance/thin_wrapper_reference.json` | passed: new conformance doc is 84 lines, new script is 218 lines, and fixture is 35 lines |
| 2026-06-28 | Order 8 final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 8 | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 8 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 8 final whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency script compile | `python3 -m py_compile scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency gate | `make transparency-anchor-check` | passed with report `target/transparency-anchor/report.json` |
| 2026-06-28 | Order 9 transparency file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency.rs crates/cortex-engine/src/accountability/transparency_tests.rs scripts/transparency_anchor_check.py crates/cortex-server/src/receipt.rs docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md` | passed: new transparency module is 212 lines, tests are 120 lines, and script is 122 lines |
| 2026-06-28 | Order 9 transparency fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 transparency whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 receipt replica script compile | `python3 -m py_compile scripts/receipt_replica_invariance_check.py scripts/receipt_threat_model_check.py scripts/accountability_receipt_schema_check.py scripts/accountability_receipt_sign_check.py scripts/accountability_receipt_verify_check.py` | passed |
| 2026-06-28 | Order 9 receipt replica gate | `make receipt-replica-invariance-check` | passed with report `target/receipt-replica-invariance/report.json` |
| 2026-06-28 | Order 9 receipt aggregate after audit head | `make accountability-receipt-check receipt-threat-model-check` | passed |
| 2026-06-28 | Order 9 receipt replica file-size sanity | `wc -l crates/cortex-engine/src/accountability/receipt_header.rs crates/cortex-engine/src/accountability/receipt_sign_tests.rs crates/cortex-receipt-verify/src/model.rs crates/cortex-receipt-verify/src/receipt_hash.rs crates/cortex-receipt-verify/src/verifier.rs crates/cortex-server/src/receipt.rs scripts/receipt_replica_invariance_check.py docs/spec/ACCOUNTABILITY_RECEIPT_V1.md docs/spec/RECEIPT_VERIFIER.md` | passed: new script is 200 lines and touched Rust files remain under 300 lines |
| 2026-06-28 | Order 9 receipt replica fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 receipt replica | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 receipt replica clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 receipt replica OpenAPI | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 receipt replica whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance integration | `cargo test -p cortex-engine --test receipt_replica_invariance --all-features` | passed |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance script compile | `python3 -m py_compile scripts/receipt_replica_invariance_check.py` | passed |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance gate | `make receipt-replica-invariance-check` | passed with report `target/receipt-replica-invariance/report.json` |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance fmt | `cargo fmt && cargo fmt --check` | passed after formatting the new integration test |
| 2026-06-28 | workspace after Order 9 replicated snapshot receipt invariance | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 replicated snapshot receipt invariance clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance file-size sanity | `wc -l crates/cortex-engine/tests/receipt_replica_invariance.rs scripts/receipt_replica_invariance_check.py mk/core-contracts.mk docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new integration test is 164 lines, script is 213 lines, and makefile remains 183 lines |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | Order 9 replicated snapshot receipt invariance final whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 cluster fail-closed first targeted | `cargo test -p cortex-engine --test cluster_fail_closed --all-features` | failed: failover scenario initially modeled the new leader at the same term as the old leader; fixed by starting the new election at a higher term and splitting helpers under the line limit |
| 2026-06-28 | Order 9 cluster fail-closed targeted | `cargo test -p cortex-engine --test cluster_fail_closed --all-features` | passed |
| 2026-06-28 | Order 9 cluster fail-closed script compile | `python3 -m py_compile scripts/consensus_failover_binder_check.py` | passed |
| 2026-06-28 | Order 9 cluster fail-closed gate | `make consensus-failover-binder-check` | passed with report `target/consensus/failover-binder.json` |
| 2026-06-28 | Order 9 cluster fail-closed fmt | `cargo fmt && cargo fmt --check` | passed after formatting the new integration test/support module |
| 2026-06-28 | workspace after Order 9 cluster fail-closed | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 cluster fail-closed clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 cluster fail-closed whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 cluster fail-closed file-size sanity | `wc -l crates/cortex-engine/tests/cluster_fail_closed.rs crates/cortex-engine/tests/cluster_fail_closed/support.rs scripts/consensus_failover_binder_check.py mk/core-contracts.mk mk/vars-core.mk mk/phony.mk` | passed: new test is 163 lines, support module is 278 lines, and script is 115 lines |
| 2026-06-28 | Order 9 multi-agent cluster consistency targeted | `cargo test -p cortex-engine --test multi_agent_cluster_consistency --all-features` | passed |
| 2026-06-28 | Order 9 multi-agent cluster consistency script compile | `python3 -m py_compile scripts/multi_agent_cluster_consistency_check.py` | passed |
| 2026-06-28 | Order 9 multi-agent cluster consistency gate | `make multi-agent-cluster-consistency-check` | passed with report `target/multi-agent-cluster-consistency/report.json` |
| 2026-06-28 | Order 9 multi-agent cluster consistency fmt | `cargo fmt && cargo fmt --check` | passed after formatting the new integration test/support module |
| 2026-06-28 | workspace after Order 9 multi-agent cluster consistency | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 multi-agent cluster consistency clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 multi-agent cluster consistency whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 multi-agent cluster consistency file-size sanity | `wc -l crates/cortex-engine/tests/multi_agent_cluster_consistency.rs crates/cortex-engine/tests/multi_agent_cluster_consistency/support.rs scripts/multi_agent_cluster_consistency_check.py mk/core-contracts.mk mk/vars-core.mk mk/phony.mk docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new test is 130 lines, support module is 210 lines, and script is 114 lines |
| 2026-06-28 | Order 9 HTTP/Raft routing targeted | `cargo test -p cortex-server http_raft_arbitrary_node_context_receipts_use_replicated_snapshot --all-features` | passed |
| 2026-06-28 | Order 9 HTTP/Raft routing gate | `make http-raft-routing-accountability-check` | passed with report `target/http-raft-routing-accountability/report.json` |
| 2026-06-28 | Order 9 HTTP/Raft routing fmt | `cargo fmt && cargo fmt --check` | passed after formatting the new server test |
| 2026-06-28 | workspace after Order 9 HTTP/Raft routing | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 HTTP/Raft routing clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | Order 9 HTTP/Raft routing whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 HTTP/Raft routing script compile | `python3 -m py_compile scripts/http_raft_routing_accountability_check.py` | passed |
| 2026-06-28 | Order 9 HTTP/Raft routing file-size sanity | `wc -l crates/cortex-server/src/tests/http_raft_routing_tests.rs scripts/http_raft_routing_accountability_check.py mk/core-contracts.mk mk/vars-core.mk mk/phony.mk docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new test is 266 lines and script is 105 lines |
| 2026-06-28 | Order 9 live Raft ingress guard targeted | `cargo test -p cortex-server cluster_ingress_guard_tests --all-features` | passed |
| 2026-06-28 | Order 9 live Raft ingress guard gate | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 live Raft ingress guard fmt | `cargo fmt && cargo fmt --check` | passed after formatting the new server config/guard tests |
| 2026-06-28 | workspace after Order 9 live Raft ingress guard | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 live Raft ingress guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 live Raft ingress guard | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 live Raft ingress guard whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 live Raft ingress guard script compile | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py` | passed |
| 2026-06-28 | Order 9 live Raft ingress guard file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs scripts/raft_ingress_production_guard_check.py` | passed: cluster helper is 49 lines, test is 118 lines, and script is 118 lines |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding targeted | `cargo test -p cortex-server cluster_ingress_guard_tests --all-features` | passed |
| 2026-06-28 | Order 9 fixed-primary ingress guard refresh | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding gate | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py` | passed |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 fixed-primary ingress forwarding | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 fixed-primary ingress forwarding clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 fixed-primary ingress forwarding | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 fixed-primary ingress forwarding file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py` | passed: cluster helper is 163 lines, server ingress test is 262 lines, guard script is 119 lines, and forwarding script is 118 lines |
| 2026-06-28 | Order 9 ingress leader hint targeted | `cargo test -p cortex-server cluster_ingress_leader_hint_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress leader hint parser targeted | `cargo test -p cortex-server parse_cluster_ingress_leader_accepts_positive_node_id` | passed |
| 2026-06-28 | Order 9 ingress leader hint gate | `make raft-ingress-leader-hint-check` | passed with report `target/raft-ingress-leader-hint/report.json` |
| 2026-06-28 | Order 9 ingress guard refresh after leader hint | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary forwarding refresh after leader hint | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 ingress leader hint scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py` | passed |
| 2026-06-28 | Order 9 ingress leader hint fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 ingress leader hint | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress leader hint clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress leader hint | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 ingress leader hint whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress leader hint file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs crates/cortex-server/src/tests/cluster_ingress_leader_hint_tests.rs scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py` | passed: cluster helper is 185 lines, guard test is 262 lines, leader-hint test is 184 lines, guard script is 119 lines, forwarding script is 118 lines, and leader-hint script is 122 lines |
| 2026-06-28 | Order 9 ingress auto-discovery config targeted | `cargo test -p cortex-engine --test replication_cluster_config cluster_config_roundtrips_optional_ingress_addresses --all-features` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery Raft STATUS targeted | `cargo test -p cortex-engine --test replication_transport replication_status_frame_reports_known_leader_without_log_mutation --all-features` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery server targeted | `cargo test -p cortex-server cluster_ingress_discovery_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery gate | `make raft-ingress-auto-discovery-check` | passed with report `target/raft-ingress-auto-discovery/report.json` |
| 2026-06-28 | Order 9 ingress guard refresh after auto-discovery | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary forwarding refresh after auto-discovery | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 leader-hint refresh after auto-discovery | `make raft-ingress-leader-hint-check` | passed with report `target/raft-ingress-leader-hint/report.json` |
| 2026-06-28 | Order 9 ingress auto-discovery scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py scripts/raft_ingress_auto_discovery_check.py` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 ingress auto-discovery | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress auto-discovery clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress auto-discovery | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress auto-discovery file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/tests/cluster_ingress_discovery_tests.rs scripts/raft_ingress_auto_discovery_check.py scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py` | passed: cluster helper is 246 lines, discovery test is 224 lines, auto-discovery script is 128 lines, guard script is 119 lines, forwarding script is 118 lines, and leader-hint script is 122 lines |
| 2026-06-28 | Order 9 ingress health routing pre-fix targeted | `cargo test -p cortex-server cluster_ingress_health_tests --all-features` | failed as expected: stale node2 leader was selected and forward failed with `Connection refused` |
| 2026-06-28 | Order 9 ingress health routing targeted | `cargo test -p cortex-server cluster_ingress_health_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress health routing gate | `make raft-ingress-health-routing-check` | passed with report `target/raft-ingress-health-routing/report.json` |
| 2026-06-28 | Order 9 ingress auto-discovery refresh after health routing | `make raft-ingress-auto-discovery-check` | passed with report `target/raft-ingress-auto-discovery/report.json` |
| 2026-06-28 | Order 9 ingress guard refresh after health routing | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary forwarding refresh after health routing | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 leader-hint refresh after health routing | `make raft-ingress-leader-hint-check` | passed with report `target/raft-ingress-leader-hint/report.json` |
| 2026-06-28 | Order 9 ingress health routing scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py scripts/raft_ingress_auto_discovery_check.py scripts/raft_ingress_health_routing_check.py` | passed |
| 2026-06-28 | Order 9 ingress health routing fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 ingress health routing | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress health routing clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress health routing | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 ingress health routing whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress health routing file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/tests/cluster_ingress_discovery_tests.rs crates/cortex-server/src/tests/cluster_ingress_health_tests.rs scripts/raft_ingress_auto_discovery_check.py scripts/raft_ingress_health_routing_check.py` | passed: cluster helper is 288 lines, discovery test is 224 lines, health test is 191 lines, auto-discovery script is 128 lines, and health-routing script is 107 lines |
| 2026-06-28 | Order 9 ingress lifecycle monitor pre-fix targeted | `cargo test -p cortex-server cluster_ingress_health_tests --all-features` | failed as expected: after draining the one-shot Raft STATUS peer, routing returned `cached monitor route failed` / no known leader |
| 2026-06-28 | Order 9 ingress lifecycle monitor targeted | `cargo test -p cortex-server cluster_ingress_health_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress lifecycle monitor gate | `make raft-ingress-lifecycle-monitor-check` | passed with report `target/raft-ingress-lifecycle-monitor/report.json` |
| 2026-06-28 | Order 9 ingress auto-discovery refresh after lifecycle monitor | `make raft-ingress-auto-discovery-check` | passed with report `target/raft-ingress-auto-discovery/report.json` |
| 2026-06-28 | Order 9 ingress health routing refresh after lifecycle monitor | `make raft-ingress-health-routing-check` | passed with report `target/raft-ingress-health-routing/report.json` |
| 2026-06-28 | Order 9 ingress guard refresh after lifecycle monitor | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary forwarding refresh after lifecycle monitor | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 leader-hint refresh after lifecycle monitor | `make raft-ingress-leader-hint-check` | passed with report `target/raft-ingress-leader-hint/report.json` |
| 2026-06-28 | Order 9 ingress lifecycle monitor scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py scripts/raft_ingress_auto_discovery_check.py scripts/raft_ingress_health_routing_check.py scripts/raft_ingress_lifecycle_monitor_check.py` | passed |
| 2026-06-28 | Order 9 ingress lifecycle monitor fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 ingress lifecycle monitor | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress lifecycle monitor clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress lifecycle monitor | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 ingress lifecycle monitor whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress lifecycle monitor file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/cluster/monitor.rs crates/cortex-server/src/tests/cluster_ingress_health_tests.rs scripts/raft_ingress_lifecycle_monitor_check.py scripts/raft_ingress_health_routing_check.py scripts/raft_ingress_auto_discovery_check.py` | passed: cluster helper is 215 lines, monitor module is 147 lines, health test is 288 lines, lifecycle script is 128 lines, health-routing script is 111 lines, and auto-discovery script is 133 lines |
| 2026-06-28 | Order 9 ingress load policy targeted | `cargo test -p cortex-server cluster_ingress_load_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress load policy gate | `make raft-ingress-load-policy-check` | passed with report `target/raft-ingress-load-policy/report.json` |
| 2026-06-28 | Order 9 ingress auto-discovery refresh after load policy | `make raft-ingress-auto-discovery-check` | passed with report `target/raft-ingress-auto-discovery/report.json` |
| 2026-06-28 | Order 9 ingress health routing refresh after load policy | `make raft-ingress-health-routing-check` | passed with report `target/raft-ingress-health-routing/report.json` |
| 2026-06-28 | Order 9 ingress guard refresh after load policy | `make raft-ingress-production-guard-check` | passed with report `target/raft-ingress-production-guard/report.json` |
| 2026-06-28 | Order 9 fixed-primary forwarding refresh after load policy | `make raft-ingress-forwarding-check` | passed with report `target/raft-ingress-forwarding/report.json` |
| 2026-06-28 | Order 9 leader-hint refresh after load policy | `make raft-ingress-leader-hint-check` | passed with report `target/raft-ingress-leader-hint/report.json` |
| 2026-06-28 | Order 9 lifecycle monitor refresh after load policy | `make raft-ingress-lifecycle-monitor-check` | passed with report `target/raft-ingress-lifecycle-monitor/report.json` |
| 2026-06-28 | Order 9 ingress load policy scripts | `python3 -m py_compile scripts/raft_ingress_production_guard_check.py scripts/raft_ingress_forwarding_check.py scripts/raft_ingress_leader_hint_check.py scripts/raft_ingress_auto_discovery_check.py scripts/raft_ingress_health_routing_check.py scripts/raft_ingress_lifecycle_monitor_check.py scripts/raft_ingress_load_policy_check.py` | passed |
| 2026-06-28 | Order 9 ingress load policy fmt | `cargo fmt && cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 ingress load policy | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress load policy clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress load policy | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 ingress load policy whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress load policy file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/cluster/monitor.rs crates/cortex-server/src/tests/cluster_ingress_load_tests.rs scripts/raft_ingress_load_policy_check.py scripts/raft_ingress_lifecycle_monitor_check.py scripts/raft_ingress_auto_discovery_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: cluster helper is 239 lines, monitor module is 206 lines, load test is 149 lines, load-policy script is 116 lines, lifecycle script is 128 lines, auto-discovery script is 133 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency witness targeted | `cargo test -p cortex-engine transparency_witness --all-features` | passed |
| 2026-06-28 | Order 9 transparency witness gate | `make transparency-witness-check` | passed with report `target/transparency-witness/report.json` |
| 2026-06-28 | Order 9 receipt replica invariance refresh after transparency witness | `make receipt-replica-invariance-check` | passed with report `target/receipt-replica-invariance/report.json` |
| 2026-06-28 | workspace after Order 9 transparency witness first pass | `cargo test --workspace --all-features` | failed once in `tests::cluster_ingress_guard_tests::non_primary_context_route_forwards_to_live_primary`: response `seq` was `2` instead of `1` |
| 2026-06-28 | Order 9 transparency witness focused rerun | `cargo test -p cortex-server non_primary_context_route_forwards_to_live_primary --all-features -- --nocapture` | passed |
| 2026-06-28 | workspace after Order 9 transparency witness rerun | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | Order 9 transparency witness scripts | `python3 -m py_compile scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/receipt_replica_invariance_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency witness fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency witness clippy first pass | `cargo clippy --workspace --all-targets -- -D warnings` | failed on local `clippy::needless_borrows_for_generic_args` in `transparency_witness.rs`; fixed before final run |
| 2026-06-28 | workspace after Order 9 transparency witness clippy rerun | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | workspace after Order 9 transparency witness final test | `cargo test --workspace --all-features` | passed after final local clippy fix |
| 2026-06-28 | API after Order 9 transparency witness | `make openapi-contract-check` | passed |
| 2026-06-28 | Order 9 transparency witness whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency witness file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency.rs crates/cortex-engine/src/accountability/transparency_witness.rs crates/cortex-engine/src/accountability/transparency_tests.rs scripts/transparency_witness_check.py scripts/transparency_anchor_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md` | passed: transparency module is 212 lines, witness module is 262 lines, witness tests are 179 lines, witness script is 136 lines, anchor script is 122 lines, GCE spec is 260 lines, verifier spec is 156 lines, security model is 176 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency witness quorum targeted | `cargo test -p cortex-engine transparency_witness_quorum --all-features` | passed |
| 2026-06-28 | Order 9 transparency witness quorum gate first pass | `make transparency-witness-quorum-check` | failed after Rust tests passed because the gate marker expected literal `duplicate witness_public_key_hex`; fixed the script marker |
| 2026-06-28 | Order 9 transparency witness quorum gate second pass | `make transparency-witness-quorum-check` | failed after Rust tests passed because the `SECURITY_MODEL.md` marker was split by line wrapping; fixed the script marker |
| 2026-06-28 | Order 9 transparency witness quorum gate | `make transparency-witness-quorum-check` | passed with report `target/transparency-witness-quorum/report.json` |
| 2026-06-28 | Order 9 transparency witness quorum scripts | `python3 -m py_compile scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | workspace after Order 9 transparency witness quorum first pass | `cargo test --workspace --all-features` | failed once in `tests::cluster_ingress_guard_tests::non_primary_context_route_forwards_to_live_primary`: response `cell_id` was `Null` instead of `1` |
| 2026-06-28 | Order 9 ingress forwarding flaky focused rerun before stabilization | `cargo test -p cortex-server non_primary_context_route_forwards_to_live_primary --all-features -- --nocapture` | passed |
| 2026-06-28 | Order 9 ingress forwarding flaky stabilization | `crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs` | stabilized the test by proving primary `/v1/context` readiness and follower `/v1/health` readiness before asserting forwarded context receipt shape |
| 2026-06-28 | Order 9 ingress forwarding stabilized focused | `cargo test -p cortex-server non_primary_context_route_forwards_to_live_primary --all-features -- --nocapture` | passed |
| 2026-06-28 | Order 9 transparency witness quorum fmt | `cargo fmt --check` | passed after the local test helper type fix |
| 2026-06-28 | workspace after Order 9 transparency witness quorum clippy first pass | `cargo clippy --workspace --all-targets -- -D warnings` | failed on local `clippy::ptr_arg` in `transparency_quorum_tests.rs`; changed the helper from `&PathBuf` to `&Path` |
| 2026-06-28 | workspace after Order 9 transparency witness quorum final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency witness quorum clippy rerun | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 transparency witness quorum | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency witness quorum whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency witness quorum report | `python3 -m json.tool target/transparency-witness-quorum/report.json` | passed: report schema `cortexdb.transparency_witness_quorum.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency witness quorum file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_witness.rs crates/cortex-engine/src/accountability/transparency_quorum.rs crates/cortex-engine/src/accountability/transparency_quorum_tests.rs scripts/transparency_witness_quorum_check.py crates/cortex-server/src/tests/cluster_ingress_guard_tests.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md` | passed: witness module is 262 lines, quorum module is 198 lines, quorum tests are 158 lines, quorum script is 131 lines, ingress guard test is 292 lines, GCE spec is 267 lines, verifier spec is 164 lines, security model is 181 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency inclusion test-first | `cargo test -p cortex-engine transparency_inclusion --all-features` | failed as expected before implementation because `transparency_inclusion` module was missing |
| 2026-06-28 | Order 9 transparency inclusion targeted | `cargo test -p cortex-engine transparency_inclusion --all-features` | passed |
| 2026-06-28 | Order 9 transparency inclusion gate | `make transparency-inclusion-check` | passed with report `target/transparency-inclusion/report.json` |
| 2026-06-28 | Order 9 transparency inclusion scripts | `python3 -m py_compile scripts/transparency_inclusion_check.py scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency inclusion fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency inclusion clippy first pass | `cargo clippy --workspace --all-targets -- -D warnings` | failed on local `manual_is_multiple_of` and `manual_div_ceil` in `transparency_inclusion.rs`; fixed before final run |
| 2026-06-28 | Order 9 transparency inclusion focused after clippy fix | `cargo test -p cortex-engine transparency_inclusion --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency inclusion clippy rerun | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | workspace after Order 9 transparency inclusion final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | API after Order 9 transparency inclusion | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency inclusion whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency inclusion report | `python3 -m json.tool target/transparency-inclusion/report.json` | passed: report schema `cortexdb.transparency_inclusion.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency inclusion file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_inclusion.rs crates/cortex-engine/src/accountability/transparency_inclusion_tests.rs scripts/transparency_inclusion_check.py crates/cortex-engine/src/accountability.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md` | passed: inclusion module is 241 lines, inclusion tests are 127 lines, inclusion script is 131 lines, accountability exports are 145 lines, GCE spec is 276 lines, verifier spec is 172 lines, security model is 187 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency consistency test-first | `cargo test -p cortex-engine transparency_consistency --all-features` | failed as expected before implementation because `transparency_consistency` module was missing |
| 2026-06-28 | Order 9 transparency consistency targeted | `cargo test -p cortex-engine transparency_consistency --all-features` | passed |
| 2026-06-28 | Order 9 transparency consistency gate first pass | `make transparency-consistency-check` | failed after Rust tests passed because the `SECURITY_MODEL.md` marker was too literal for Markdown line wrapping; script marker fixed before final gate |
| 2026-06-28 | Order 9 transparency consistency gate | `make transparency-consistency-check` | passed with report `target/transparency-consistency/report.json` |
| 2026-06-28 | Order 9 transparency consistency scripts | `python3 -m py_compile scripts/transparency_consistency_check.py scripts/transparency_inclusion_check.py scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency consistency fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency consistency final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency consistency clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 transparency consistency | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency consistency whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency consistency report | `python3 -m json.tool target/transparency-consistency/report.json` | passed: report schema `cortexdb.transparency_consistency.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency consistency file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_consistency.rs crates/cortex-engine/src/accountability/transparency_consistency_tests.rs scripts/transparency_consistency_check.py crates/cortex-engine/src/accountability.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md` | passed: consistency module is 261 lines, consistency tests are 127 lines, consistency script is 130 lines, accountability exports are 153 lines, GCE spec is 284 lines, verifier spec is 180 lines, security model is 193 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency availability test-first | `cargo test -p cortex-engine transparency_availability --all-features` | failed as expected before implementation because `TransparencyAvailability*` types and verifier functions were missing |
| 2026-06-28 | Order 9 transparency availability first implementation run | `cargo test -p cortex-engine transparency_availability --all-features` | failed because test fixtures used non-hex `head/root` strings while the validator correctly required 64-char hex hashes; fixtures changed to hex prefixes |
| 2026-06-28 | Order 9 transparency availability file-size first pass | `wc -l crates/cortex-engine/src/accountability/transparency_availability.rs crates/cortex-engine/src/accountability/transparency_availability_tests.rs` | failed local size discipline: module was 338 lines; reduced to 279 lines before gate |
| 2026-06-28 | Order 9 transparency availability targeted | `cargo test -p cortex-engine transparency_availability --all-features` | passed |
| 2026-06-28 | Order 9 transparency availability scripts | `python3 -m py_compile scripts/transparency_availability_check.py scripts/transparency_consistency_check.py scripts/transparency_inclusion_check.py scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency availability gate | `make transparency-availability-check` | passed with report `target/transparency-availability/report.json` |
| 2026-06-28 | Order 9 transparency availability fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency availability first pass | `cargo test --workspace --all-features` | failed once in unrelated server tests `rate_limit_returns_typed_429_when_enabled` and `context_route_discovers_raft_leader_then_forwards_to_ingress_address`; focused reruns passed |
| 2026-06-28 | Order 9 transparency availability flaky focused reruns | `cargo test -p cortex-server rate_limit_returns_typed_429_when_enabled --all-features -- --nocapture`; `cargo test -p cortex-server context_route_discovers_raft_leader_then_forwards_to_ingress_address --all-features -- --nocapture` | passed |
| 2026-06-28 | workspace after Order 9 transparency availability final test | `cargo test --workspace --all-features` | passed on clean rerun |
| 2026-06-28 | workspace after Order 9 transparency availability clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 transparency availability | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency availability whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency availability report | `python3 -m json.tool target/transparency-availability/report.json` | passed: report schema `cortexdb.transparency_availability.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency availability file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_availability.rs crates/cortex-engine/src/accountability/transparency_availability_tests.rs scripts/transparency_availability_check.py crates/cortex-engine/src/accountability.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md scripts/crypto_foundation_check.py` | passed: availability module is 279 lines, availability tests are 188 lines, availability script is 137 lines, accountability exports are 162 lines, GCE spec is 292 lines, verifier spec is 188 lines, security model is 201 lines, crypto foundation script is 164 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency gossip test-first | `cargo test -p cortex-engine transparency_gossip --all-features` | failed as expected before implementation because `TransparencyGossip*` types and verifier functions were missing |
| 2026-06-28 | Order 9 transparency gossip targeted | `cargo test -p cortex-engine transparency_gossip --all-features` | passed |
| 2026-06-28 | Order 9 transparency gossip file-size first pass | `wc -l crates/cortex-engine/src/accountability/transparency_gossip.rs crates/cortex-engine/src/accountability/transparency_gossip_tests.rs crates/cortex-engine/src/accountability.rs` | failed local size discipline: module was 333 lines; split type definitions into `transparency_gossip/types.rs`, reducing validator to 289 lines |
| 2026-06-28 | Order 9 transparency gossip targeted after split | `cargo test -p cortex-engine transparency_gossip --all-features` | passed |
| 2026-06-28 | Order 9 transparency gossip scripts | `python3 -m py_compile scripts/transparency_gossip_check.py scripts/transparency_availability_check.py scripts/transparency_consistency_check.py scripts/transparency_inclusion_check.py scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency gossip gate | `make transparency-gossip-check` | passed with report `target/transparency-gossip/report.json` |
| 2026-06-28 | Order 9 transparency gossip fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 transparency gossip final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency gossip clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 transparency gossip | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency gossip whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 transparency gossip report | `python3 -m json.tool target/transparency-gossip/report.json` | passed: report schema `cortexdb.transparency_gossip.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency gossip file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_gossip.rs crates/cortex-engine/src/accountability/transparency_gossip/types.rs crates/cortex-engine/src/accountability/transparency_gossip_tests.rs scripts/transparency_gossip_check.py crates/cortex-engine/src/accountability.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md scripts/crypto_foundation_check.py` | passed: gossip validator is 289 lines, gossip types are 51 lines, gossip tests are 115 lines, gossip script is 139 lines, accountability exports are 171 lines, GCE spec is 301 lines, verifier spec is 196 lines, security model is 207 lines, crypto foundation script is 164 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency SLO test-first | `cargo test -p cortex-engine transparency_slo --all-features` | failed as expected before implementation because `TransparencySlo*` types and verifier functions were missing |
| 2026-06-28 | Order 9 transparency SLO targeted | `cargo test -p cortex-engine transparency_slo --all-features` | passed |
| 2026-06-28 | Order 9 transparency SLO file-size first pass | `wc -l crates/cortex-engine/src/accountability/transparency_slo.rs crates/cortex-engine/src/accountability/transparency_slo/types.rs crates/cortex-engine/src/accountability/transparency_slo_tests.rs crates/cortex-engine/src/accountability.rs` | failed local size discipline: module was 316 lines; split validation helpers into `transparency_slo/validation.rs`, reducing the validator to 292 lines after formatting |
| 2026-06-28 | Order 9 transparency SLO targeted after split | `cargo test -p cortex-engine transparency_slo --all-features` | passed |
| 2026-06-28 | Order 9 transparency SLO scripts | `python3 -m py_compile scripts/transparency_slo_check.py scripts/transparency_gossip_check.py scripts/transparency_availability_check.py scripts/transparency_consistency_check.py scripts/transparency_inclusion_check.py scripts/transparency_witness_quorum_check.py scripts/transparency_witness_check.py scripts/transparency_anchor_check.py scripts/crypto_foundation_check.py` | passed |
| 2026-06-28 | Order 9 transparency SLO fmt first pass | `cargo fmt --check` | failed on formatting in the new `transparency_slo.rs`; fixed with `cargo fmt` |
| 2026-06-28 | Order 9 transparency SLO gate first pass | `make transparency-slo-check` | failed after Rust tests passed because a Markdown line-wrap split the script marker `continuous public transparency operations`; marker fixed before final gate |
| 2026-06-28 | Order 9 transparency SLO gate | `make transparency-slo-check` | passed with report `target/transparency-slo/report.json` |
| 2026-06-28 | Order 9 transparency SLO post-status fmt | `cargo fmt --check` | passed |
| 2026-06-28 | Order 9 transparency SLO post-status gate | `make transparency-slo-check` | passed |
| 2026-06-28 | workspace after Order 9 transparency SLO final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 transparency SLO clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 transparency SLO | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 transparency SLO report | `python3 -m json.tool target/transparency-slo/report.json` | passed: report schema `cortexdb.transparency_slo.report.v1` with `status: passed` |
| 2026-06-28 | Order 9 transparency SLO file-size sanity | `wc -l crates/cortex-engine/src/accountability/transparency_slo.rs crates/cortex-engine/src/accountability/transparency_slo/types.rs crates/cortex-engine/src/accountability/transparency_slo/validation.rs crates/cortex-engine/src/accountability/transparency_slo_tests.rs scripts/transparency_slo_check.py crates/cortex-engine/src/accountability.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/spec/GCE_CONTRACT.md docs/spec/RECEIPT_VERIFIER.md docs/SECURITY_MODEL.md scripts/crypto_foundation_check.py` | passed: SLO validator is 292 lines, SLO types are 55 lines, SLO validation helpers are 35 lines, SLO tests are 110 lines, SLO script is 144 lines, accountability exports are 179 lines, GCE spec is 313 lines, verifier spec is 206 lines, security model is 214 lines, crypto foundation script is 164 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 transparency SLO whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 ingress load metrics test-first | `cargo test -p cortex-server cluster_ingress_load_tests --all-features` | failed as expected before implementation because `cluster_ingress_max_in_flight_per_node` and `ClusterIngressMonitor::load_metrics` were missing |
| 2026-06-28 | Order 9 ingress load metrics targeted | `cargo test -p cortex-server cluster_ingress_load_tests --all-features` | passed |
| 2026-06-28 | Order 9 ingress load metrics parser/prometheus targeted | `cargo test -p cortex-server parse_cluster_ingress_max_in_flight --all-features`; `cargo test -p cortex-server metrics_prometheus_output_contains_contract_series --all-features` | passed |
| 2026-06-28 | Order 9 ingress load metrics scripts | `python3 -m py_compile scripts/raft_ingress_load_metrics_check.py scripts/raft_ingress_load_policy_check.py` | passed |
| 2026-06-28 | Order 9 ingress load metrics fmt first pass | `cargo fmt --check` | failed on formatting in `cluster/monitor.rs`; fixed with `cargo fmt` |
| 2026-06-28 | Order 9 ingress load metrics gate first pass | `make raft-ingress-load-metrics-check` | failed after Rust tests passed because the older load-policy gate line limit for `cluster/monitor.rs` was still 230; raised to 250 for the new metrics method |
| 2026-06-28 | Order 9 ingress load metrics gate second pass | `make raft-ingress-load-metrics-check` | failed after Rust tests passed because the older load-policy gate test-file line limit was still 220; raised to 260 for the expanded regression tests |
| 2026-06-28 | Order 9 ingress load metrics gate | `make raft-ingress-load-metrics-check` | passed with report `target/raft-ingress-load-metrics/report.json` |
| 2026-06-28 | workspace after Order 9 ingress load metrics first pass | `cargo test --workspace --all-features` | failed once in `load_policy_uses_operator_configured_limit_from_options` because the loopback test peer was not ready before the 200 ms monitor retry budget elapsed |
| 2026-06-28 | Order 9 ingress load metrics flaky stabilization | `cargo test -p cortex-server cluster_ingress_load_tests --all-features -- --nocapture` | passed after increasing the local test peer request budget to 32 and monitor readiness retries to 1 second |
| 2026-06-28 | Order 9 ingress load metrics helper split | `wc -l crates/cortex-server/src/http_metrics/response.rs crates/cortex-server/src/http_metrics/cluster_ingress.rs scripts/raft_ingress_load_metrics_check.py` | passed: response formatter is 282 lines, cluster ingress metrics helper is 39 lines, and load metrics script is 147 lines |
| 2026-06-28 | Order 9 ingress load metrics post-helper gate | `make raft-ingress-load-metrics-check` | passed with report `target/raft-ingress-load-metrics/report.json` |
| 2026-06-28 | workspace after Order 9 ingress load metrics final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 ingress load metrics clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 ingress load metrics | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 ingress load metrics reports | `python3 -m json.tool target/raft-ingress-load-policy/report.json`; `python3 -m json.tool target/raft-ingress-load-metrics/report.json` | passed: load policy report schema `cortexdb.raft_ingress_load_policy_gate.v1` and load metrics report schema `cortexdb.raft_ingress_load_metrics_gate.v1` are both `status=passed` |
| 2026-06-28 | Order 9 ingress load metrics file-size sanity | `wc -l crates/cortex-server/src/config.rs crates/cortex-server/src/main.rs crates/cortex-server/src/cluster/monitor.rs crates/cortex-server/src/http_metrics/response.rs crates/cortex-server/src/http_metrics/cluster_ingress.rs crates/cortex-server/src/tests/cluster_ingress_load_tests.rs scripts/raft_ingress_load_metrics_check.py scripts/raft_ingress_load_policy_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: config is 297 lines, main is 578 lines, monitor is 237 lines, response formatter is 282 lines, cluster ingress metrics helper is 39 lines, load tests are 228 lines, metrics script is 147 lines, policy script is 116 lines, and status doc is intentionally oversized |
| 2026-06-28 | Order 9 ingress load metrics whitespace | `git diff --check` | passed |
| 2026-06-28 | Order 9 consensus release-lane test-first | `python3 scripts/consensus_release_lane_check.py --check-wiring-only --runs 1 --report target/consensus/release-lane/preflight-red.json` | failed as expected because `release-check`, vars, phony, and public docs did not yet include `consensus-release-lane-check` |
| 2026-06-28 | Order 9 consensus release-lane wiring preflight | `python3 scripts/consensus_release_lane_check.py --check-wiring-only --runs 1 --report target/consensus/release-lane/preflight-wiring.json` | passed after make/docs wiring |
| 2026-06-28 | Order 9 consensus release-lane gate first pass | `make consensus-release-lane-check` | failed after replication partition evidence passed because `consensus_gate_check.py` still pointed at archived consensus docs under old root paths |
| 2026-06-28 | Order 9 consensus release-lane stale marker fix | `python3 scripts/consensus_gate_check.py --gate partition-soak --evidence target/consensus/release-lane/run-01/replication-partition/report.json --report target/consensus/release-lane/run-01/partition-soak.json` | passed after moving consensus gate doc markers to `docs/archive/*` and target markers to `mk/core-security-ops.mk` |
| 2026-06-28 | Order 9 consensus release-lane gate | `make consensus-release-lane-check` | passed with report `target/consensus/release-lane/report.json`: 3 of 3 consecutive runs green across partition soak, failover SLO, rejoin, failover binder, multi-agent cluster consistency, and receipt replica invariance |
| 2026-06-28 | Order 9 consensus release-lane scripts | `python3 -m py_compile scripts/consensus_release_lane_check.py scripts/consensus_gate_check.py scripts/distributed_consensus_research_check.py scripts/replication_partition_check.py scripts/replication_lifecycle_check.py` | passed |
| 2026-06-28 | Order 9 consensus release-lane fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after Order 9 consensus release-lane first pass | `cargo test --workspace --all-features` | failed/interrupted after `context_route_discovers_raft_leader_then_forwards_to_ingress_address` failed once and `audit_log_file_redacts_ingestion_query_and_body` kept running past 60 seconds |
| 2026-06-28 | Order 9 consensus release-lane focused reruns | `cargo test -p cortex-server context_route_discovers_raft_leader_then_forwards_to_ingress_address --all-features -- --nocapture`; `cargo test -p cortex-server audit_log_file_redacts_ingestion_query_and_body --all-features -- --nocapture` | passed |
| 2026-06-28 | workspace after Order 9 consensus release-lane final test | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after Order 9 consensus release-lane clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after Order 9 consensus release-lane | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | Order 9 consensus release-lane report | `python3 -m json.tool target/consensus/release-lane/report.json` | passed: report schema `cortexdb.consensus.release_lane_gate.v1`, `release_ready=true`, `production_ready=false`, and 3 of 3 runs passed |
| 2026-06-28 | Order 9 consensus release-lane file-size sanity | `wc -l scripts/consensus_release_lane_check.py scripts/consensus_gate_check.py scripts/distributed_consensus_research_check.py mk/core-security-ops.mk mk/release.mk mk/vars-core.mk mk/phony.mk docs/STATUS.md docs/COMMUNITY_ROADMAP.md docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: release-lane script is 243 lines, consensus gate script is 186 lines, distributed consensus research script is 77 lines, core security ops makefile is 266 lines, public docs are 60/75/216 lines, and the central vars makefile plus status doc are intentionally oversized manifests |
| 2026-06-28 | Order 9 consensus release-lane whitespace | `git diff --check` | passed |
| 2026-06-28 | CP-3 migration boundary test-first | `rg -q "agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence" crates/cortex-engine/src/cell_ids.rs && rg -q "true 31-bit session/feedback agent slot" docs/spec/GCE_CONTRACT.md && rg -q "requires_schema_migration_for_31_bit_slots" scripts/cell_id_collision_check.py` | failed as expected because the collision gate did not yet make the v1 layout and 31-bit migration boundary explicit |
| 2026-06-28 | CP-3 migration boundary gate | `make cell-id-collision-check` | passed with session, feedback, remember, and script checks; report `target/cell-id-collision/report.json` records `requires_schema_migration_for_31_bit_slots=true` |
| 2026-06-28 | CP-3 migration boundary aggregate | `make correctness-prerequisites-check` | passed |
| 2026-06-28 | CP-3 migration boundary script | `python3 -m py_compile scripts/cell_id_collision_check.py` | passed |
| 2026-06-28 | CP-3 migration boundary fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after CP-3 migration boundary | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after CP-3 migration boundary clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after CP-3 migration boundary | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | CP-3 migration boundary report | `python3 -m json.tool target/cell-id-collision/report.json` | passed: report schema `cortexdb.cell_id_collision.report.v1`, `agent_slot_bits=28`, `sequence_bits=32`, and `requires_schema_migration_for_31_bit_slots=true` |
| 2026-06-28 | CP-3 migration boundary file-size sanity | `wc -l crates/cortex-engine/src/cell_ids.rs scripts/cell_id_collision_check.py docs/spec/GCE_CONTRACT.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: cell_ids helper is 43 lines, cell-id gate script is 150 lines, GCE contract is 333 lines, and status doc is intentionally oversized |
| 2026-06-28 | receipt production readiness test-first | `rg -q "receipt-production-readiness-check" mk scripts docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/SECURITY_MODEL.md && rg -q "RECEIPT_PRODUCTION_READINESS_REPORT" mk/vars-core.mk` | failed as expected because the production-grade public receipt readiness gate did not exist |
| 2026-06-28 | receipt production readiness scripts | `python3 -m py_compile scripts/receipt_production_readiness_check.py scripts/key_management_check.py scripts/compliance_boundary_check.py scripts/security_release_report_check.py scripts/transparency_slo_check.py` | passed |
| 2026-06-28 | receipt production readiness gate | `make receipt-production-readiness-check` | passed with report `target/receipt-production-readiness/report.json`: component reports passed, `production_ready=false`, blockers are `kms_hsm_receipt_key_custody` and `compliance_certification` |
| 2026-06-28 | receipt production readiness report | `python3 -m json.tool target/receipt-production-readiness/report.json` | passed: report schema `cortexdb.receipt_production_readiness.report.v1`, receipt/transparency/key-management/security/compliance components passed, and production readiness remains false |
| 2026-06-28 | receipt production readiness file-size sanity | `wc -l scripts/receipt_production_readiness_check.py mk/core-security-ops.mk mk/vars-core.mk mk/phony.mk mk/release.mk docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: readiness script is 176 lines, core security ops makefile is 273 lines, public security model is 222 lines, and central vars/status docs are intentionally oversized manifests |
| 2026-06-28 | receipt production readiness whitespace | `git diff --check` | passed |
| 2026-06-28 | receipt production readiness fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after receipt production readiness | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after receipt production readiness clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after receipt production readiness | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses and generated type contracts validated |
| 2026-06-28 | receipt KMS/HSM custody test-first | `rg -q "receipt-kms-hsm-custody-check" mk scripts docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md docs/SECURITY_MODEL.md docs/spec/ACCOUNTABILITY_RECEIPT_V1.md && rg -q "RECEIPT_KMS_HSM_CUSTODY_REPORT" mk/vars-core.mk` | failed as expected because the dedicated KMS/HSM custody boundary gate did not exist |
| 2026-06-28 | receipt KMS/HSM custody scripts | `python3 -m py_compile scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_readiness_check.py` | passed |
| 2026-06-28 | receipt KMS/HSM custody gate first run | `make receipt-kms-hsm-custody-check` | failed because the new spec markers for `signing_seed_hex` and fail-closed fallback were split across markdown line breaks |
| 2026-06-28 | receipt KMS/HSM custody gate | `make receipt-kms-hsm-custody-check` | passed with report `target/receipt-kms-hsm-custody/report.json`: `kms_hsm_custody=false`, `custody_mode=external_signer_contract_only`, and runtime external signer support remains false |
| 2026-06-28 | receipt KMS/HSM custody report | `python3 -m json.tool target/receipt-kms-hsm-custody/report.json` | passed: report schema `cortexdb.receipt_kms_hsm_custody.report.v1`, blockers are `runtime_external_receipt_signer_not_implemented` and `operator_kms_hsm_custody_evidence_not_implemented` |
| 2026-06-28 | receipt production readiness with KMS/HSM boundary | `make receipt-production-readiness-check` | passed with separate `receipt_kms_hsm_custody` component; aggregate report remains `production_ready=false` with blockers `kms_hsm_receipt_key_custody` and `compliance_certification` |
| 2026-06-28 | receipt production readiness with KMS/HSM boundary report | `python3 -m json.tool target/receipt-production-readiness/report.json` | passed: `component_status.receipt_kms_hsm_custody=passed`, `readiness.kms_hsm_receipt_key_custody=false`, and no production-grade public receipt claim is made |
| 2026-06-28 | receipt KMS/HSM custody file-size sanity | `wc -l scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_readiness_check.py mk/core-security-ops.mk mk/vars-core.mk mk/phony.mk docs/SECURITY_MODEL.md docs/spec/ACCOUNTABILITY_RECEIPT_V1.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: new custody script is 164 lines, readiness script is 183 lines, core security ops makefile is 277 lines, and status/vars docs are intentionally oversized manifests |
| 2026-06-28 | receipt KMS/HSM custody fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after receipt KMS/HSM custody | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after receipt KMS/HSM custody clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after receipt KMS/HSM custody | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but all live responses, error taxonomy, and generated SDK type contracts validated |
| 2026-06-28 | receipt KMS/HSM custody whitespace | `git diff --check` | passed |
| 2026-06-28 | receipt external signer runtime test-first | `rg -q "ReceiptExternalSigner\|CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND\|signed_receipt_value_with_signer" crates/cortex-server/src crates/cortex-engine/src docs/spec/ACCOUNTABILITY_RECEIPT_V1.md docs/SECURITY_MODEL.md` | failed as expected because the runtime external signer path did not exist |
| 2026-06-28 | receipt external signer engine | `cargo test -p cortex-engine accountability_receipt_header --all-features` | passed with external signer trait acceptance and invalid-signature rejection |
| 2026-06-28 | receipt external signer server focused | `cargo test -p cortex-server receipt --all-features` | passed with external signer request-shape capture, invalid-signature fail-closed behavior, and existing configured receipt emission |
| 2026-06-28 | receipt external signer key management | `make key-management-check` | passed with local receipt key custody, external command signer contract, keyed audit MAC, and CLI receipt rotation coverage |
| 2026-06-28 | receipt external signer KMS/HSM custody gate | `make receipt-kms-hsm-custody-check` | passed with `external_signer_runtime_supported=true`, `kms_hsm_custody=false`, `custody_mode=external_signer_runtime_no_kms_hsm_evidence`, and blocker `operator_kms_hsm_custody_evidence_not_implemented` |
| 2026-06-28 | receipt external signer KMS/HSM custody report | `python3 -m json.tool target/receipt-kms-hsm-custody/report.json` | passed: report schema `cortexdb.receipt_kms_hsm_custody.report.v1`; runtime external signer support is true and KMS/HSM custody remains unclaimed |
| 2026-06-28 | receipt external signer production readiness | `make receipt-production-readiness-check` | passed with `component_status.receipt_kms_hsm_custody=passed`, `production_ready=false`, and blockers `kms_hsm_receipt_key_custody` plus `compliance_certification` |
| 2026-06-28 | receipt external signer production readiness report | `python3 -m json.tool target/receipt-production-readiness/report.json` | passed: report schema `cortexdb.receipt_production_readiness.report.v1`; `readiness.kms_hsm_receipt_key_custody=false` remains the explicit production blocker |
| 2026-06-28 | receipt external signer file-size sanity | `wc -l crates/cortex-server/src/config.rs crates/cortex-server/src/receipt.rs crates/cortex-server/src/receipt_signer.rs crates/cortex-engine/src/accountability/receipt_header.rs crates/cortex-engine/src/context/receipt_evidence.rs docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: config is 296 lines, receipt runtime is 227 lines, external signer runtime is 206 lines, receipt header is 254 lines, receipt evidence is 98 lines, and status doc is intentionally oversized |
| 2026-06-28 | receipt external signer clippy first pass | `cargo clippy --workspace --all-targets -- -D warnings` | failed on local `clippy::ptr_arg` in `parse_receipt_external_signer`; fixed by changing helper inputs from `&PathBuf` to `&Path` |
| 2026-06-28 | receipt external signer parser fix | `cargo test -p cortex-server parse_receipt_external_signer --all-features` | passed after the `&Path` helper fix |
| 2026-06-28 | workspace after receipt external signer runtime | `cargo test --workspace --all-features` | passed after the final Rust helper fix |
| 2026-06-28 | API after receipt external signer runtime | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, and generated SDK type contracts validated |
| 2026-06-28 | receipt external signer production readiness first rerun | `make receipt-production-readiness-check` | failed after component tests because `database_instance_identity_check.py` still required the stale local-only marker `database instance identity is required when receipt_signing_key is set` |
| 2026-06-28 | receipt external signer database identity marker fix | `make database-instance-identity-check` | passed after updating durable identity markers for local or external receipt signing mode |
| 2026-06-28 | receipt external signer scripts | `python3 -m py_compile scripts/database_instance_identity_check.py scripts/receipt_production_readiness_check.py scripts/receipt_kms_hsm_custody_check.py scripts/key_management_check.py` | passed |
| 2026-06-28 | receipt external signer production readiness final | `make receipt-production-readiness-check` | passed with all component reports successful, `production_ready=false`, and blockers limited to `kms_hsm_receipt_key_custody` plus `compliance_certification` |
| 2026-06-28 | receipt external signer final reports | `python3 -m json.tool target/receipt-kms-hsm-custody/report.json`; `python3 -m json.tool target/receipt-production-readiness/report.json`; `python3 -m json.tool target/database-instance-identity/report.json` | passed: external signer runtime support is true, database identity gate passed, KMS/HSM custody remains false, and production readiness remains false |
| 2026-06-28 | receipt external signer final fmt | `cargo fmt --check` | passed |
| 2026-06-28 | receipt external signer final clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | receipt external signer final whitespace | `git diff --check` | passed |
| 2026-06-28 | receipt external signer final file-size sanity | `wc -l crates/cortex-server/src/config.rs crates/cortex-server/src/receipt.rs crates/cortex-server/src/receipt_signer.rs crates/cortex-engine/src/accountability/receipt_header.rs crates/cortex-engine/src/context/receipt_evidence.rs scripts/database_instance_identity_check.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_readiness_check.py scripts/key_management_check.py docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: config is 296 lines, receipt runtime is 227 lines, external signer runtime is 206 lines, receipt header is 254 lines, receipt evidence is 98 lines, database identity script is 133 lines, custody script is 193 lines, readiness script is 183 lines, key-management script is 224 lines, and status doc is intentionally oversized |
| 2026-06-28 | receipt KMS/HSM evidence test-first | `python3 scripts/receipt_kms_hsm_custody_check.py --root . --report target/receipt-kms-hsm-custody/pre-evidence-red.json --custody-evidence fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json --expected-key-id receipt-key.external --expected-public-key-hex 03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5 --expected-signer-ref kms://test/receipt-key` | failed as expected before implementation because `--custody-evidence` and expected runtime binding args were unsupported |
| 2026-06-28 | receipt KMS/HSM evidence scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_readiness_check.py scripts/key_management_check.py` | passed |
| 2026-06-28 | receipt KMS/HSM evidence default gate | `make receipt-kms-hsm-custody-check` | passed with default report still `kms_hsm_custody=false`, `custody_mode=external_signer_runtime_no_kms_hsm_evidence`, and blocker `operator_kms_hsm_custody_evidence_not_implemented` |
| 2026-06-28 | receipt KMS/HSM evidence positive gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-kms-hsm-custody/make-evidence-positive.json RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json RECEIPT_KMS_HSM_EXPECTED_KEY_ID=receipt-key.external RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX=03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5 RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF=kms://test/receipt-key` | passed with `kms_hsm_custody=true`, `custody_mode=kms`, `operator_evidence.valid=true`, and no KMS/HSM custody blockers |
| 2026-06-28 | receipt KMS/HSM evidence binding mismatch | `python3 scripts/receipt_kms_hsm_custody_check.py --root . --report target/receipt-kms-hsm-custody/evidence-mismatch.json --custody-evidence fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json --expected-key-id receipt-key.other --expected-public-key-hex 03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5 --expected-signer-ref kms://test/receipt-key` | failed as expected: evidence key id did not match expected runtime key id |
| 2026-06-28 | receipt KMS/HSM evidence readiness delta | `python3 scripts/receipt_production_readiness_check.py ... --receipt-kms-hsm-custody-report target/receipt-kms-hsm-custody/make-evidence-positive.json`; `python3 scripts/receipt_production_readiness_check.py ... --receipt-kms-hsm-custody-report target/receipt-kms-hsm-custody/report.json` | passed: positive evidence removes `kms_hsm_receipt_key_custody` leaving only `compliance_certification`; default report keeps both blockers |
| 2026-06-28 | receipt KMS/HSM evidence key management | `make key-management-check` | passed with receipt key custody, external command signer contract, keyed audit MAC, and CLI receipt rotation coverage |
| 2026-06-28 | receipt KMS/HSM evidence default production readiness | `make receipt-production-readiness-check` | passed with `production_ready=false`; blockers are `kms_hsm_receipt_key_custody` and `compliance_certification` |
| 2026-06-28 | receipt KMS/HSM evidence final reports | `python3 -m json.tool target/receipt-kms-hsm-custody/report.json`; `python3 -m json.tool target/receipt-kms-hsm-custody/make-evidence-positive.json`; `python3 -m json.tool target/receipt-production-readiness/report.json`; `python3 -m json.tool target/receipt-production-readiness/evidence-positive.json` | passed: default custody report keeps `kms_hsm_custody=false`, positive evidence report sets `kms_hsm_custody=true`, default readiness keeps KMS/HSM plus compliance blockers, and positive readiness leaves only `compliance_certification` |
| 2026-06-28 | receipt KMS/HSM evidence fmt | `cargo fmt --check` | passed |
| 2026-06-28 | workspace after receipt KMS/HSM evidence | `cargo test --workspace --all-features` | passed |
| 2026-06-28 | workspace after receipt KMS/HSM evidence clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-28 | API after receipt KMS/HSM evidence | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, and generated SDK type contracts validated |
| 2026-06-28 | receipt KMS/HSM evidence file-size sanity | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_readiness_check.py scripts/key_management_check.py mk/core-security-ops.mk mk/vars-core.mk mk/phony.mk docs/AUTH.md docs/SECURITY_MODEL.md docs/spec/ACCOUNTABILITY_RECEIPT_V1.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json` | passed: new evidence script is 211 lines, custody script is 226 lines, readiness script is 183 lines, key-management script is 224 lines, core security ops makefile is 279 lines, and central vars/status/auth docs are intentionally oversized manifests |
| 2026-06-28 | receipt KMS/HSM evidence whitespace | `git diff --check` | passed |
| 2026-06-29 | compliance certification evidence test-first | `python3 scripts/compliance_boundary_check.py --report target/compliance-boundary/pre-certification-red.json --certification-evidence fixtures/compliance/certification_evidence.valid.json --expected-framework soc2_type_ii` | failed as expected before implementation because `--certification-evidence` and `--expected-framework` were unsupported |
| 2026-06-29 | compliance certification evidence scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/compliance_boundary_check.py scripts/receipt_production_readiness_check.py scripts/security_release_report_check.py` | passed |
| 2026-06-29 | compliance certification evidence default gate | `make compliance-boundary-check` | passed with default report still `supported_certified_frameworks=[]`, `external_certification.valid=false`, and `compliance_immutability=false` |
| 2026-06-29 | compliance certification evidence positive gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/compliance-boundary/make-certification-positive.json COMPLIANCE_CERTIFICATION_EVIDENCE=fixtures/compliance/certification_evidence.valid.json COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK=soc2_type_ii` | passed with `supported_certified_frameworks=["soc2_type_ii"]`, `external_certification.valid=true`, and `compliance_immutability=true` |
| 2026-06-29 | compliance certification evidence framework mismatch | `python3 scripts/compliance_boundary_check.py --report target/compliance-boundary/certification-mismatch.json --certification-evidence fixtures/compliance/certification_evidence.valid.json --expected-framework iso_27001` | failed as expected: evidence framework did not match the expected framework |
| 2026-06-29 | compliance certification evidence positive readiness delta | `python3 scripts/receipt_production_readiness_check.py ... --receipt-kms-hsm-custody-report target/receipt-kms-hsm-custody/make-evidence-positive.json --compliance-boundary-report target/compliance-boundary/make-certification-positive.json --report target/receipt-production-readiness/all-evidence-positive.json` | passed: synthetic positive KMS/HSM plus synthetic positive compliance evidence remove both readiness blockers and set `production_ready=true` only in the explicit positive report |
| 2026-06-29 | compliance certification evidence default production readiness | `make receipt-production-readiness-check` | passed with `production_ready=false`; blockers remain `kms_hsm_receipt_key_custody` and `compliance_certification` |
| 2026-06-29 | compliance certification evidence reports | `python3 -m json.tool target/compliance-boundary/report.json`; `python3 -m json.tool target/compliance-boundary/make-certification-positive.json`; `python3 -m json.tool target/receipt-production-readiness/report.json`; `python3 -m json.tool target/receipt-production-readiness/all-evidence-positive.json` | passed: default reports keep production readiness blocked, while explicit synthetic positive reports prove validator wiring only |
| 2026-06-29 | compliance certification evidence file-size sanity | `wc -l scripts/compliance_certification_evidence.py scripts/compliance_boundary_check.py scripts/receipt_production_readiness_check.py mk/core-security-ops.mk mk/vars-core.mk docs/archive/COMPLIANCE_BOUNDARY_MAPPING.md docs/SECURITY_MODEL.md fixtures/compliance/certification_evidence.valid.json` | passed: new evidence script is 226 lines, compliance boundary script is 129 lines, readiness script is 195 lines, core security ops makefile is 279 lines, and central vars file remains an intentionally oversized manifest |
| 2026-06-29 | compliance certification evidence fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance certification evidence | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance certification evidence clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance certification evidence | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | Order 9 adaptive ingress scheduling test-first | `cargo test -p cortex-server cluster_ingress_adaptive_tests --all-features` | failed as expected after adding the regression: second route returned `Unavailable("cached Raft ingress monitor leader 2 is over ingress load limit")` before adaptive refresh was implemented |
| 2026-06-29 | Order 9 adaptive ingress scheduling targeted test | `cargo test -p cortex-server cluster_ingress_adaptive_tests --all-features` | passed after routing cached monitor ingress through `try_acquire_adaptive_leader_node` |
| 2026-06-29 | Order 9 adaptive ingress scheduling first make run | `make raft-ingress-adaptive-scheduling-check` | failed after Rust load-policy tests passed because `scripts/raft_ingress_load_policy_check.py` still expected the old production marker `try_acquire_cached_leader_node` in `cluster.rs` |
| 2026-06-29 | Order 9 adaptive ingress scheduling gate | `make raft-ingress-adaptive-scheduling-check` | passed with report `target/raft-ingress-adaptive-scheduling/report.json` |
| 2026-06-29 | Order 9 ingress load metrics refresh after adaptive scheduling | `make raft-ingress-load-metrics-check` | passed; chain now runs load policy, adaptive scheduling, max-in-flight parsing, Prometheus metrics, and metrics report |
| 2026-06-29 | Order 9 adaptive ingress scheduling scripts | `python3 -m py_compile scripts/raft_ingress_adaptive_scheduling_check.py scripts/raft_ingress_load_policy_check.py scripts/raft_ingress_load_metrics_check.py` | passed |
| 2026-06-29 | Order 9 adaptive ingress scheduling reports | `python3 -m json.tool target/raft-ingress-adaptive-scheduling/report.json`; `python3 -m json.tool target/raft-ingress-load-policy/report.json`; `python3 -m json.tool target/raft-ingress-load-metrics/report.json` | passed: adaptive report schema `cortexdb.raft_ingress_adaptive_scheduling_gate.v1`, load-policy report, and load-metrics report are all `status=passed` |
| 2026-06-29 | Order 9 adaptive ingress scheduling file-size sanity | `wc -l crates/cortex-server/src/cluster.rs crates/cortex-server/src/cluster/monitor.rs crates/cortex-server/src/tests/cluster_ingress_adaptive_tests.rs scripts/raft_ingress_adaptive_scheduling_check.py scripts/raft_ingress_load_policy_check.py scripts/raft_ingress_load_metrics_check.py mk/core-contracts.mk mk/vars-core.mk mk/phony.mk docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: cluster is 239 lines, monitor is 249 lines, adaptive test is 148 lines, adaptive script is 115 lines, load-policy script is 116 lines, load-metrics script is 148 lines, and status/vars/phony manifests are intentionally oversized |
| 2026-06-29 | workspace after adaptive ingress scheduling first full run | `cargo test --workspace --all-features` | failed because the adaptive test depended on a 200ms remote leader health response and flaked under the full parallel server suite |
| 2026-06-29 | Order 9 adaptive ingress scheduling stabilized targeted test | `cargo test -p cortex-server cluster_ingress_adaptive_tests --all-features` | passed after changing the simulated leadership change to local node 3, removing the remote health-timeout dependency from the second route |
| 2026-06-29 | Order 9 adaptive ingress scheduling stabilized gate | `make raft-ingress-adaptive-scheduling-check` | passed after the local-leader stabilization and regenerated `target/raft-ingress-adaptive-scheduling/report.json` |
| 2026-06-29 | adaptive ingress scheduling fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after adaptive ingress scheduling | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after adaptive ingress scheduling clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after adaptive ingress scheduling | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | Order 9 ingress load metrics final after adaptive stabilization | `make raft-ingress-load-metrics-check` | passed; chain reran load policy, adaptive scheduling, max-in-flight parsing, Prometheus metrics, and metrics report after the local-leader test stabilization |
| 2026-06-29 | Order 9 adaptive ingress scheduling final reports | `python3 -m json.tool target/raft-ingress-adaptive-scheduling/report.json`; `python3 -m json.tool target/raft-ingress-load-metrics/report.json` | passed: adaptive scheduling and load-metrics reports are both `status=passed` with updated line counts |
| 2026-06-29 | adaptive ingress scheduling whitespace | `git diff --check` | passed |
| 2026-06-29 | production ready strict gate test-first | `python3 scripts/receipt_production_readiness_check.py ... --report target/receipt-production-readiness/pre-strict-red.json --require-production-ready` | failed as expected before implementation because `--require-production-ready` was unsupported |
| 2026-06-29 | production readiness report-only after strict gate | `python3 scripts/receipt_production_readiness_check.py ... --report target/receipt-production-readiness/report-only-after-strict.json` | passed and preserved the report-only contract with `production_ready=false` |
| 2026-06-29 | production ready strict default direct | `python3 scripts/receipt_production_readiness_check.py ... --report target/receipt-production-readiness/strict-default.json --require-production-ready` | failed as expected with `production_ready=false` while KMS/HSM and compliance blockers remain |
| 2026-06-29 | production ready strict positive direct | `python3 scripts/receipt_production_readiness_check.py ... --receipt-kms-hsm-custody-report target/receipt-kms-hsm-custody/make-evidence-positive.json --compliance-boundary-report target/compliance-boundary/make-certification-positive.json --report target/receipt-production-readiness/strict-positive.json --require-production-ready` | passed with synthetic positive evidence reports and `production_ready=true` |
| 2026-06-29 | production ready strict default make target | `make receipt-production-ready-check` | failed as expected only at the final strict step after the component inventory passed; report `target/receipt-production-readiness/strict-report.json` records `production_ready=false` |
| 2026-06-29 | production ready strict positive make target | `make receipt-production-ready-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json ... COMPLIANCE_CERTIFICATION_EVIDENCE=fixtures/compliance/certification_evidence.valid.json ...` | passed with synthetic positive KMS/HSM and compliance evidence reports; report `target/receipt-production-readiness/strict-make-positive.json` records `production_ready=true` |
| 2026-06-29 | production ready strict scripts | `python3 -m py_compile scripts/receipt_production_readiness_check.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py` | passed |
| 2026-06-29 | production ready strict reports | `python3 -m json.tool target/receipt-production-readiness/strict-default.json`; `python3 -m json.tool target/receipt-production-readiness/strict-positive.json`; `python3 -m json.tool target/receipt-production-readiness/strict-make-positive.json` | passed: default strict report is `status=failed`, positive strict reports are `status=passed`, and all carry `strict_production_ready_required=true` |
| 2026-06-29 | production ready strict fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production ready strict gate | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production ready strict gate clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production ready strict gate | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production ready strict whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence origin guard test-first | `python3 scripts/receipt_production_readiness_check.py ... --receipt-kms-hsm-custody-report target/receipt-kms-hsm-custody/pre-origin-fixture.json --compliance-boundary-report target/compliance-boundary/pre-origin-fixture.json --report target/receipt-production-readiness/pre-origin-fixture-strict.json --require-production-ready` + JSON assertion | failed as expected before implementation because schema-valid synthetic fixture evidence incorrectly set `production_ready=true` |
| 2026-06-29 | production evidence origin fixture guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_readiness_check.py`; fixture KMS/HSM and compliance validators; `! python3 scripts/receipt_production_readiness_check.py ... --report target/receipt-production-readiness/origin-fixture-strict.json --require-production-ready` | passed: fixture evidence remains schema-valid validator coverage, but strict production readiness fails with `production_ready=false`, `kms_hsm_operator_evidence=false`, and `compliance_operator_evidence=false` |
| 2026-06-29 | production evidence origin default inventory | `make receipt-production-readiness-check` | passed and preserved report-only inventory with production blockers intact |
| 2026-06-29 | production evidence origin default strict gate | `make receipt-production-ready-check` | failed as expected only at the final strict production-ready step with `production_ready=false` |
| 2026-06-29 | production evidence origin fixture reports | `python3 scripts/receipt_production_readiness_check.py ... --report target/receipt-production-readiness/origin-fixture-inventory.json`; `python3 -m json.tool ...`; JSON assertions over inventory and strict reports | passed: fixture inventory report is `status=passed` with `production_ready=false`; fixture strict report is `status=failed` with operator evidence readiness false |
| 2026-06-29 | production evidence origin fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence origin guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence origin guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence origin guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence origin whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence preflight test-first | `make receipt-production-evidence-preflight-check` | failed as expected before implementation because the fail-fast operator evidence preflight target did not exist |
| 2026-06-29 | production evidence preflight missing-input guard | `python3 -m py_compile scripts/receipt_production_evidence_preflight.py ...`; `! python3 scripts/receipt_production_evidence_preflight.py --report target/receipt-production-evidence/missing-preflight.json`; JSON assertions | passed: report lists all six required KMS/HSM and compliance inputs and keeps `production_evidence_ready=false` |
| 2026-06-29 | production evidence preflight default make guard | `! make receipt-production-evidence-preflight-check`; `python3 -m json.tool target/receipt-production-evidence/preflight.json`; JSON assertions | passed: default make target fails fast with all six inputs missing |
| 2026-06-29 | production evidence preflight fixture rejection | `! make receipt-production-evidence-preflight-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json ... COMPLIANCE_CERTIFICATION_EVIDENCE=fixtures/compliance/certification_evidence.valid.json ...` | passed: schema-valid fixtures are rejected as non-operator-origin evidence |
| 2026-06-29 | production evidence preflight operator-shaped parser path | generated temporary non-fixture JSON under `target/codex-verification/operator-shaped-evidence`; `make receipt-production-evidence-preflight-check ...`; JSON assertions | passed: preflight can pass with complete non-fixture, schema-valid operator-shaped inputs; this is parser coverage only, not real external production evidence |
| 2026-06-29 | production evidence preflight fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence preflight first full run | `cargo test --workspace --all-features` | failed once in `tests::cluster_ingress_leader_hint_tests::non_primary_context_route_forwards_to_hinted_leader` with `cell_id` response `Null`; this Rust ingress test is outside the preflight Python/make changes |
| 2026-06-29 | production evidence preflight targeted rerun | `cargo test -p cortex-server tests::cluster_ingress_leader_hint_tests::non_primary_context_route_forwards_to_hinted_leader --all-features -- --exact --nocapture` | passed |
| 2026-06-29 | workspace after production evidence preflight rerun | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence preflight clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence preflight | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence preflight whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence generated-artifact origin test-first | `python3 scripts/receipt_production_evidence_preflight.py ... --custody-evidence target/codex-verification/operator-shaped-evidence/receipt-kms-hsm.json --certification-evidence target/codex-verification/operator-shaped-evidence/compliance-certification.json ...` + JSON assertion | failed as expected before implementation because generated `target/` evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence generated-artifact guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py ...`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/target-artifact-preflight.json`; JSON assertions | passed: generated `target/` evidence is classified as `generated_local_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence generated-artifact missing/fixture regression | missing-input preflight; fixture preflight through `make receipt-production-evidence-preflight-check`; JSON assertions | passed: missing inputs still fail with six required vars, and fixtures still fail as `synthetic_fixture` |
| 2026-06-29 | production evidence generated-artifact tmp parser path | copied operator-shaped JSON to `/tmp/cortexdb-operator-shaped-evidence`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/tmp-operator-shaped-preflight.json`; JSON assertions | passed: complete non-repo, non-fixture, schema-valid parser path still passes; this remains parser coverage only, not real external evidence |
| 2026-06-29 | production evidence generated-artifact fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence generated-artifact guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence generated-artifact guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence generated-artifact guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence generated-artifact whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence temporary-local origin test-first | `python3 scripts/receipt_production_evidence_preflight.py ... --custody-evidence /tmp/cortexdb-operator-shaped-evidence/receipt-kms-hsm.json --certification-evidence /tmp/cortexdb-operator-shaped-evidence/compliance-certification.json ...` + JSON assertion | failed as expected before implementation because `/tmp` evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence temporary-local guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py ...`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/tmp-artifact-preflight.json`; JSON assertions | passed: `/tmp` evidence is classified as `temporary_local_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence temporary-local regressions | generated `target/` preflight; missing-input preflight; fixture preflight through `make receipt-production-evidence-preflight-check`; JSON assertions | passed: `target/` remains `generated_local_artifact`, fixtures remain `synthetic_fixture`, and missing inputs still report six required vars |
| 2026-06-29 | production evidence temporary-local external parser path | copied operator-shaped JSON to `/mnt/hf_model_weights/arman/3bit/sites/cortexdb-operator-evidence-test`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/external-operator-shaped-preflight.json`; JSON assertions; removed the temporary external directory | passed: complete non-repo, non-fixture, non-temp parser path still passes; this remains parser coverage only, not real external evidence |
| 2026-06-29 | production evidence temporary-local fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence temporary-local guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence temporary-local guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence temporary-local guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence temporary-local whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence local-reference origin test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `file:///tmp/...` and `target/...`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-local-reference.json`; JSON assertion | failed as expected before implementation because local/generated refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence local-reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py ...`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/local-reference-preflight.json`; JSON assertions | passed: nested local/generated refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence local-reference regressions | generated `target/` preflight; temporary-local preflight; missing-input preflight; fixture preflight through `make receipt-production-evidence-preflight-check`; JSON assertions | passed: all existing negative origin guards still classify as expected |
| 2026-06-29 | production evidence local-reference external parser path | copied operator-shaped JSON to `/mnt/hf_model_weights/arman/3bit/sites/cortexdb-operator-evidence-test`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/external-operator-shaped-after-local-ref-guard.json`; JSON assertions; removed temporary external/local-ref directories | passed: complete non-repo, non-fixture, non-temp, non-local-reference parser path still passes; this remains parser coverage only, not real external evidence |
| 2026-06-29 | production evidence local-reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence local-reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence local-reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence local-reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence local-reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence path-variant reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `./target/...`, `../target/...`, and absolute `.../target/...`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-path-variant-reference.json`; JSON assertion | failed as expected before implementation because path-variant `target` refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence path-variant reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py ...`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/path-variant-reference-preflight.json`; JSON assertions | passed: path variants such as `./target`, `../target`, and absolute `.../target/...` are classified as `local_reference_artifact` |
| 2026-06-29 | production evidence path-variant reference regressions | generated `target/` preflight; fixture preflight through `make receipt-production-evidence-preflight-check`; clean temporary-local and local-reference regressions; missing-input preflight; external non-local parser path; cleanup assertions | passed: all existing negative origin guards still classify as expected, non-file URI parser path still passes, and temporary evidence dirs were removed |
| 2026-06-29 | production evidence path-variant reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence path-variant reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence path-variant reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence path-variant reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence path-variant reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence component-origin test-first | schema-valid KMS/HSM and compliance fixtures through `make receipt-kms-hsm-custody-check` and `make compliance-boundary-check`; JSON assertion expecting component production booleans to stay false | failed as expected before implementation because fixture evidence incorrectly set `kms_hsm_custody=true` |
| 2026-06-29 | production evidence component-origin guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py`; repeated fixture component checks; JSON assertions | passed: schema-valid fixture evidence remains valid validator coverage but keeps `kms_hsm_custody=false`, `production_safe=false`, `supported_certified_frameworks=[]`, and `compliance_immutability=false` |
| 2026-06-29 | production evidence component-origin inventory regression | default `make receipt-kms-hsm-custody-check` and `make compliance-boundary-check`; JSON assertions | passed: default inventory reports still pass while keeping KMS/HSM custody and compliance immutability false |
| 2026-06-29 | production evidence component-origin aggregate regression | `python3 scripts/receipt_production_readiness_check.py ...` using fixture component reports, then strict rerun with `--require-production-ready`; JSON assertions | passed: inventory report remains `status=passed` with `production_ready=false`; strict report fails with `production_ready=false` |
| 2026-06-29 | production evidence component-origin preflight regressions | missing-input preflight; fixture preflight through `make receipt-production-evidence-preflight-check`; JSON assertions | passed: missing inputs still report six required vars, and fixtures still fail as non-operator-origin |
| 2026-06-29 | production evidence component-origin external parser path | copied operator-shaped JSON to `/mnt/hf_model_weights/arman/3bit/sites/cortexdb-component-origin-operator-test`; component checks; JSON assertions; removed temporary external directory | passed: complete non-repo, non-fixture, non-temp parser evidence can still set component booleans for parser coverage; this remains parser coverage only, not real external evidence |
| 2026-06-29 | production evidence component-origin fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence component-origin guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence component-origin guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence component-origin guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence component-origin default readiness | `make receipt-production-readiness-check`; strict `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON assertions | passed: inventory report remains `status=passed` with `production_ready=false`; strict report fails with `production_ready=false` |
| 2026-06-29 | production evidence component-origin whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence loopback-reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `http://localhost`, `http://127.0.0.1`, `http://0.0.0.0`, and `http://[::1]`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-loopback-reference.json`; JSON assertion | failed as expected before implementation because loopback refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence loopback-reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/loopback-reference-preflight.json`; JSON assertions | passed: loopback refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence loopback-reference regressions | direct classifier assertions for loopback, `target`, `/tmp`, `s3://`, `https://`, and `arn:` refs; component checks with loopback refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: loopback refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence loopback-reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence loopback-reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence loopback-reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence loopback-reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence loopback-reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence resolved-path test-first | external symlink evidence files pointing into `target/codex-verification/operator-shaped-evidence`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-symlink-target-preflight.json`; JSON assertion | failed as expected before implementation because symlink-to-target evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence resolved-path guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/symlink-target-preflight.json`; JSON assertions | passed: external symlinks resolving into `target/` are classified as `generated_local_artifact` |
| 2026-06-29 | production evidence resolved-path regressions | component checks with symlink-to-target evidence; missing-input and fixture preflight; loopback-reference preflight; external non-local parser path; cleanup assertions | passed: resolved local artifacts stay blocked, existing negative guards still fail, and non-symlink external parser paths still pass as parser coverage |
| 2026-06-29 | production evidence resolved-path fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence resolved-path guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence resolved-path guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence resolved-path guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence resolved-path whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence file-scheme reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `file:/tmp/...`, `file:target/...`, `file:./target/...`, and `file:%2F%2Ftmp/...`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-file-scheme-reference.json`; JSON assertion | failed as expected before implementation because file-scheme refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence file-scheme reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/file-scheme-reference-preflight.json`; JSON assertions | passed: file-scheme refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence file-scheme reference regressions | direct classifier assertions for `file:`, percent-encoded local refs, loopback refs, `s3://`, `https://`, and `arn:`; component checks with file-scheme refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: file-scheme refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence file-scheme reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence file-scheme reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence file-scheme reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence file-scheme reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence file-scheme reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence loopback-equivalent reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at expanded IPv6 loopback, IPv4-mapped IPv6 loopback, `[::]`, and `http://0/...`; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-loopback-equivalent-preflight.json`; JSON assertion | failed as expected before implementation because loopback-equivalent refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence loopback-equivalent reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/loopback-equivalent-preflight.json`; JSON assertions | passed: loopback-equivalent refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence loopback-equivalent reference regressions | direct classifier assertions for expanded IPv6 loopback, IPv4-mapped IPv6 loopback, `[::]`, `http://0/...`, existing loopback refs, `s3://`, `https://`, `arn:`, and public documentation IP refs; component checks with loopback-equivalent refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: loopback-equivalent refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence loopback-equivalent reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence loopback-equivalent reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence loopback-equivalent reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence loopback-equivalent reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence loopback-equivalent reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence recursive-encoded reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at double-encoded `file:`, `target/`, `/tmp`, loopback URLs, and encoded backslash separators; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-recursive-encoded-preflight.json`; JSON assertion | failed as expected before implementation because recursively encoded local refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence recursive-encoded reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/recursive-encoded-preflight.json`; JSON assertions | passed: recursively encoded local refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence recursive-encoded reference regressions | direct classifier assertions for double-encoded `file:`, `target/`, `/tmp`, encoded backslash separators, loopback URLs, `s3://`, `https://`, and `arn:`; component checks with recursively encoded refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: recursively encoded local refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence recursive-encoded reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence recursive-encoded reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence recursive-encoded reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence recursive-encoded reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence recursive-encoded reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence legacy IPv4 alias test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at decimal dword, hexadecimal dword, octal dword, dotted hexadecimal, dotted octal, and short dotted loopback/unspecified aliases; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-legacy-ipv4-preflight.json`; JSON assertion | failed as expected before implementation because legacy IPv4 loopback aliases inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence legacy IPv4 alias guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/legacy-ipv4-preflight.json`; JSON assertions | passed: legacy IPv4 loopback/unspecified aliases are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence legacy IPv4 alias regressions | direct classifier assertions for decimal, hexadecimal, octal, dotted hexadecimal, dotted octal, short dotted, and unspecified aliases plus remote numeric hosts; component checks with legacy IPv4 aliases; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: legacy IPv4 local aliases are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence legacy IPv4 alias fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence legacy IPv4 alias guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence legacy IPv4 alias guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence legacy IPv4 alias guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence legacy IPv4 alias whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence Windows absolute-path test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `C:\...`, `D:/...`, percent-encoded drive paths, and double-encoded drive paths; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-windows-path-preflight.json`; JSON assertion | failed as expected before implementation because Windows absolute refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence Windows absolute-path guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/windows-path-preflight.json`; JSON assertions | passed: Windows absolute refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence Windows absolute-path regressions | direct classifier assertions for backslash, slash, percent-encoded, and double-encoded drive paths plus `https://`, `s3://`, `arn:`, `kms://`, and drive-relative non-absolute refs; component checks with Windows absolute refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: Windows absolute refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence Windows absolute-path fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence Windows absolute-path guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence Windows absolute-path guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence Windows absolute-path guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence Windows absolute-path whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence UNC/scheme-relative path test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at UNC backslash paths, scheme-relative `//host/path` refs, percent-encoded UNC paths, and double-encoded scheme-relative paths; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-unc-path-preflight.json`; JSON assertion | failed as expected before implementation because UNC/scheme-relative refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence UNC/scheme-relative path guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/unc-path-preflight.json`; JSON assertions | passed: UNC/scheme-relative refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence UNC/scheme-relative path regressions | direct classifier assertions for backslash UNC paths, scheme-relative refs, percent-encoded and double-encoded variants plus explicit `https://`, `s3://`, `arn:`, and `kms://`; component checks with UNC refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: UNC/scheme-relative refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence UNC/scheme-relative path fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence UNC/scheme-relative path guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence UNC/scheme-relative path guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence UNC/scheme-relative path guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence UNC/scheme-relative path whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence local transport URI test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `unix:`, `npipe:`, `pipe:`, percent-encoded local transport URIs, and double-encoded local transport URIs; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-local-transport-preflight.json`; JSON assertion | failed as expected before implementation because local transport URI refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence local transport URI guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/local-transport-preflight.json`; JSON assertions | passed: local transport URI refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence local transport URI regressions | direct classifier assertions for `unix:`, `npipe:`, `pipe:`, percent-encoded and double-encoded variants plus explicit `https://`, `s3://`, `arn:`, and `kms://`; component checks with local transport URI refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: local transport URI refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence local transport URI fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence local transport URI guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence local transport URI guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence local transport URI guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence local transport URI whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence shell-local expansion test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `~/`, `~user/`, `$HOME/`, `${USERPROFILE}/`, `$TMPDIR/`, `%USERPROFILE%/`, `%TEMP%/`, percent-encoded shell refs, and double-encoded shell refs; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-shell-expansion-preflight.json`; JSON assertion | failed as expected before implementation because shell/user-local expansion refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence shell-local expansion guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/shell-expansion-preflight.json`; JSON assertions | passed: shell/user-local expansion refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence shell-local expansion regressions | direct classifier assertions for `~/`, `~user/`, `$HOME/`, `${HOME}/`, `$USERPROFILE/`, `${USERPROFILE}/`, `%USERPROFILE%/`, `$TMPDIR/`, `${TEMP}/`, `%TEMP%/`, percent-encoded and double-encoded variants plus explicit `https://`, `s3://`, `arn:`, `kms://`, and `gs://`; component checks with shell-local refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: shell/user-local expansion refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence shell-local expansion fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence shell-local expansion guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence shell-local expansion guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence shell-local expansion guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence shell-local expansion whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence generic path reference test-first | modified temporary non-repo evidence JSON so nested artifact refs pointed at `operator-evidence/...`, `./operator-evidence/...`, `../operator-evidence/...`, `reports/...`, percent-encoded relative refs, and double-encoded relative refs; `python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/pre-origin-relative-ref-preflight.json`; JSON assertion | failed as expected before implementation because relative filesystem refs inside schema-valid evidence incorrectly passed as operator-origin |
| 2026-06-29 | production evidence generic path reference guard | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; `! python3 scripts/receipt_production_evidence_preflight.py ... --report target/receipt-production-evidence/relative-ref-preflight.json`; JSON assertions | passed: relative and absolute POSIX-style filesystem refs are classified as `local_reference_artifact` and rejected as non-operator-origin |
| 2026-06-29 | production evidence generic path reference regressions | direct classifier assertions for `operator-evidence/...`, `./operator-evidence/...`, `../operator-evidence/...`, `reports/...`, `/home/operator/evidence/...`, `/opt/cortexdb/evidence/...`, percent-encoded and double-encoded variants plus explicit `https://`, `s3://`, `gs://`, `arn:`, `kms://`, and prose with whitespace; component checks with relative refs; missing-input and fixture preflight; external non-local parser path; cleanup assertions | passed: path-like refs are rejected, existing negative guards still fail, and non-local parser paths still pass as parser coverage |
| 2026-06-29 | production evidence generic path reference fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence generic path reference guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence generic path reference guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence generic path reference guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence generic path reference whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence origin classifier split baseline | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; direct classifier assertions over file/target/loopback/legacy IP/Windows/UNC/local transport/shell/path refs and `https://`, `s3://`, `gs://`, `arn:`, `kms://`, prose, and version literals | passed before refactor |
| 2026-06-29 | production evidence origin classifier split refactor | `python3 -m py_compile scripts/evidence_origin.py scripts/evidence_origin_references.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py`; same direct classifier assertions after moving helper code | passed: behavior preserved after module split |
| 2026-06-29 | production evidence origin classifier split regressions | local mixed-reference preflight, external non-local parser preflight, KMS/HSM component report, compliance component report, JSON assertions, and temp-dir cleanup | passed: local/generated references still fail closed, external parser path still passes, and component production booleans stay false for non-operator evidence |
| 2026-06-29 | production evidence origin classifier split fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence origin classifier split | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence origin classifier split clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence origin classifier split | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence origin classifier split whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence origin regression gate test-first | `! make production-evidence-origin-check` | failed as expected before implementation because the repeatable classifier regression make target did not exist |
| 2026-06-29 | production evidence origin regression gate | `make production-evidence-origin-check`; `python3 -m json.tool target/production-evidence-origin/report.json` | passed: report schema `cortexdb.evidence_origin_check.v1`, 16 local reference cases, 7 remote/operator reference cases, operator/generated/temporary/symlink/nested-local/fixture origin classes, and no failures |
| 2026-06-29 | production evidence origin regression target integration | missing preflight fail-closed, default KMS/HSM custody component, default compliance component, default production readiness inventory, JSON assertions over blocker ids | passed after correcting the assertion shape for `blockers` objects: production readiness remains `false` with `kms_hsm_receipt_key_custody` and `compliance_certification` blockers |
| 2026-06-29 | production evidence origin regression gate fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence origin regression gate | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence origin regression gate clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence origin regression gate | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence origin regression gate whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence preflight origin prerequisite test-first | `! make receipt-production-evidence-preflight-check ... PRODUCTION_EVIDENCE_ORIGIN_REPORT=target/production-evidence-origin/preflight-prewire-report.json`; `test ! -e target/production-evidence-origin/preflight-prewire-report.json`; JSON assertion over missing inputs | failed as expected before wiring because missing-input preflight did not run the origin classifier regression gate |
| 2026-06-29 | production evidence preflight origin prerequisite missing-input | `! make receipt-production-evidence-preflight-check ... PRODUCTION_EVIDENCE_ORIGIN_REPORT=target/production-evidence-origin/preflight-wired-report.json`; JSON assertions over origin report and preflight report | passed: origin report is generated with `status=passed`, then preflight fails closed with six missing inputs and `production_evidence_ready=false` |
| 2026-06-29 | production evidence preflight origin prerequisite positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom origin/preflight reports; JSON assertions; temp-dir cleanup | passed: origin prerequisite passed, parser-positive operator-shaped evidence still passes preflight, and temporary test directory was removed |
| 2026-06-29 | production evidence preflight origin prerequisite fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence preflight origin prerequisite | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence preflight origin prerequisite clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence preflight origin prerequisite | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence preflight origin prerequisite whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence operator handoff test-first | `! make receipt-production-evidence-preflight-check ...`; JSON assertion over preflight report | failed as expected before implementation because the fail-closed preflight report did not include `operator_handoff` |
| 2026-06-29 | production evidence operator handoff missing-input | `! make receipt-production-evidence-preflight-check ...`; JSON assertions over `operator_handoff` schema, required inputs, evidence schemas, origin boundary, and validation command | passed: preflight remains fail-closed with six missing inputs while emitting machine-readable operator handoff metadata |
| 2026-06-29 | production evidence operator handoff positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom origin/preflight reports; JSON assertions; temp-dir cleanup | passed: parser-positive operator-shaped evidence still passes preflight and report includes `operator_handoff` |
| 2026-06-29 | production evidence operator handoff fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence operator handoff | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence operator handoff clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence operator handoff | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence operator handoff whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence standalone handoff test-first | `! make receipt-production-evidence-handoff-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_REPORT=target/receipt-production-evidence/handoff-prewire-standalone.json`; `test ! -e target/receipt-production-evidence/handoff-prewire-standalone.json` | failed as expected before implementation because no standalone handoff make target existed |
| 2026-06-29 | production evidence standalone handoff report | `make receipt-production-evidence-handoff-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_REPORT=target/receipt-production-evidence/handoff-standalone.json`; JSON assertions | passed: target emits `cortexdb.receipt_production_evidence_handoff.v1` without readiness status or production-evidence-ready fields |
| 2026-06-29 | production evidence standalone handoff fail-closed preflight regression | `! make receipt-production-evidence-preflight-check ...`; JSON assertions over missing inputs and embedded handoff | passed: standalone handoff did not soften preflight fail-closed behavior |
| 2026-06-29 | production evidence standalone handoff positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom reports; JSON assertions; temp-dir cleanup | passed: parser-positive operator-shaped evidence still passes preflight with embedded handoff |
| 2026-06-29 | production evidence standalone handoff fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence standalone handoff | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence standalone handoff clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence standalone handoff | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence standalone handoff whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence field-level handoff test-first | `make receipt-production-evidence-handoff-check ...`; JSON assertion over handoff report | failed as expected before implementation because the standalone handoff did not include `evidence_field_checklist` |
| 2026-06-29 | production evidence field-level handoff report | `make receipt-production-evidence-handoff-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_REPORT=target/receipt-production-evidence/field-checklist-standalone.json`; JSON assertions over KMS/HSM and compliance checklist fields | passed: report includes required fields, required values, controls, artifact requirements, forbidden secret fields, and still emits no readiness status |
| 2026-06-29 | production evidence field-level handoff missing-input preflight | `! make receipt-production-evidence-preflight-check ...`; JSON assertions over embedded `operator_handoff.evidence_field_checklist` | passed: preflight remains fail-closed with six missing inputs and embeds the field-level checklist |
| 2026-06-29 | production evidence field-level handoff positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom reports; JSON assertions; temp-dir cleanup | passed: parser-positive operator-shaped evidence still passes preflight and embeds the field-level checklist |
| 2026-06-29 | production evidence field-level handoff fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence field-level handoff | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence field-level handoff clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence field-level handoff | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence field-level handoff whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence handoff consistency test-first | `! make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/handoff-consistency-prewire.json`; `test ! -e target/receipt-production-evidence/handoff-consistency-prewire.json` | failed as expected before implementation because no handoff consistency target existed |
| 2026-06-29 | production evidence handoff consistency gate | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/handoff-consistency.json`; JSON assertions | passed: report schema `cortexdb.receipt_production_evidence_handoff_check.v1`, status `passed`, expected inputs/components, and no failures |
| 2026-06-29 | production evidence handoff consistency standalone report-only regression | `make receipt-production-evidence-handoff-check ...`; JSON assertions | passed: standalone handoff still includes field checklist and still emits no readiness status |
| 2026-06-29 | production evidence handoff consistency missing-input preflight regression | `! make receipt-production-evidence-preflight-check ...`; JSON assertions | passed: preflight remains fail-closed with six missing inputs and embedded checklist |
| 2026-06-29 | production evidence handoff consistency positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom reports; JSON assertions; temp-dir cleanup | passed: parser-positive operator-shaped evidence still passes preflight and embeds the checklist |
| 2026-06-29 | production evidence handoff consistency fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence handoff consistency | `cargo test --workspace --all-features`; targeted rerun for `tests::cluster_ingress_discovery_tests::context_route_discovers_raft_leader_then_forwards_to_ingress_address`; full `cargo test --workspace --all-features` rerun | passed on full rerun; first full run hit a transient cluster ingress discovery assertion, the exact test passed immediately, and the full workspace rerun passed |
| 2026-06-29 | workspace after production evidence handoff consistency clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence handoff consistency | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence handoff consistency whitespace | `git diff --check` | passed |
| 2026-06-29 | production evidence preflight consistency prerequisite test-first | `! make receipt-production-evidence-preflight-check ... RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/preflight-consistency-prewire-report.json`; `test ! -e target/receipt-production-evidence/preflight-consistency-prewire-report.json` | failed as expected before wiring because preflight did not run the handoff-consistency gate |
| 2026-06-29 | production evidence preflight consistency prerequisite missing-input | `! make receipt-production-evidence-preflight-check ...`; JSON assertions over consistency, origin, and preflight reports | passed: consistency and origin reports are generated with `status=passed`, then preflight fails closed with six missing inputs |
| 2026-06-29 | production evidence preflight consistency prerequisite positive parser path | `make receipt-production-evidence-preflight-check ...` with temporary non-repo operator-shaped evidence JSON and custom consistency/origin/preflight reports; JSON assertions; temp-dir cleanup | passed: consistency prerequisite passed, parser-positive operator-shaped evidence still passes preflight, and temporary test directory was removed |
| 2026-06-29 | production evidence preflight consistency prerequisite fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence preflight consistency prerequisite | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence preflight consistency prerequisite clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence preflight consistency prerequisite | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence preflight consistency prerequisite whitespace | `git diff --check` | passed |
| 2026-06-29 | production readiness handoff consistency prewire | `make receipt-production-readiness-check ... RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/readiness-prewire-handoff-consistency.json`; JSON assertions | passed as test-first evidence: readiness inventory passed but did not create or report handoff-consistency evidence before wiring |
| 2026-06-29 | production readiness handoff consistency inventory | `make receipt-production-readiness-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-readiness/handoff-consistency-wired.json RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/readiness-wired-handoff-consistency.json`; JSON assertions | passed: handoff-consistency report is generated, readiness records `production_evidence_handoff_consistency=passed`, and blockers remain `kms_hsm_receipt_key_custody` plus `compliance_certification` |
| 2026-06-29 | production ready strict handoff consistency fail-closed | `make receipt-production-ready-check ... RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/strict-handoff-consistency.json`; JSON assertions | failed as expected only at the final strict step with `production_ready=false`, while handoff consistency stayed passed and the two external evidence blockers remained |
| 2026-06-29 | production readiness handoff consistency scripts | `python3 -m py_compile scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_preflight.py` | passed |
| 2026-06-29 | production readiness handoff consistency fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production readiness handoff consistency | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production readiness handoff consistency clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production readiness handoff consistency | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production readiness handoff consistency whitespace | `git diff --check` | passed |
| 2026-06-29 | production ready preflight fail-fast test-first | `make receipt-production-ready-check ... RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT=target/receipt-production-evidence/strict-preflight-prewire.json`; JSON assertions | passed as test-first evidence: strict ready did not create a preflight report before wiring and only failed at the final strict `production_ready=false` step |
| 2026-06-29 | production ready preflight fail-fast default | `make receipt-production-ready-check RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT=target/receipt-production-evidence/strict-preflight-wired.json ...`; JSON assertions | failed as expected at `receipt-production-evidence-preflight-check` with six missing operator inputs; origin and handoff-consistency reports were refreshed, and readiness/strict reports were not generated |
| 2026-06-29 | production ready preflight fail-fast scripts | `python3 -m py_compile scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production ready preflight fail-fast fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production ready preflight fail-fast | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production ready preflight fail-fast clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production ready preflight fail-fast | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production ready preflight fail-fast whitespace | `git diff --check` | passed |
| 2026-06-29 | production ready origin-proof red-first parser-only bypass | `make receipt-production-ready-check ...` with temporary non-repo parser-only KMS/HSM and compliance JSON | passed as test-first evidence before the guard: strict production ready incorrectly passed on local parser-only evidence |
| 2026-06-29 | production ready origin-proof strict parser-only guard | `! make receipt-production-ready-check ...` with the same temporary non-repo parser-only evidence; JSON assertions over `target/receipt-production-evidence/strict-parser-probe-after-guard-preflight.json` | passed: strict production ready now fails at `receipt-production-evidence-production-preflight-check` because both evidence files lack `production_origin_proof` |
| 2026-06-29 | production ready origin-proof parser-coverage regression | `make receipt-production-evidence-preflight-check ...` with the same temporary non-repo parser-only evidence | passed: non-strict preflight remains available for schema/parser coverage and records `production_origin_proof_required=false` |
| 2026-06-29 | production ready origin-proof direct strict bypass regression | `! python3 scripts/receipt_production_readiness_check.py ... --require-production-ready` without `--production-evidence-preflight-report`; JSON assertions | passed: direct strict readiness now records blocker `production_evidence_preflight` and keeps `production_ready=false` |
| 2026-06-29 | production ready origin-proof handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/origin-proof-handoff-consistency.json` | passed: handoff consistency now checks the production-origin-proof schema and checklist fields |
| 2026-06-29 | production ready origin-proof default strict fail-fast | `! make receipt-production-ready-check RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT=target/receipt-production-evidence/origin-proof-default-strict-preflight.json ...`; JSON assertions | passed: default strict target fails before readiness with six missing inputs and `production_origin_proof_required=true` |
| 2026-06-29 | production ready origin-proof readiness inventory | `make receipt-production-readiness-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-readiness/origin-proof-inventory.json ...`; JSON assertions | passed: report-only inventory remains `status=passed`, keeps `production_ready=false`, and records blockers `production_evidence_preflight`, `kms_hsm_receipt_key_custody`, and `compliance_certification` |
| 2026-06-29 | production ready origin-proof scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production ready origin-proof fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production ready origin-proof guard | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production ready origin-proof guard clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production ready origin-proof guard | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof content-binding red-first | `make receipt-production-evidence-production-preflight-check ...` with temporary non-repo proof-shaped KMS/HSM and compliance JSON lacking evidence digest binding | passed as test-first evidence before the guard: production preflight incorrectly accepted proof-shaped parser-only evidence |
| 2026-06-29 | production origin-proof unbound proof guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-shaped-after-guard-preflight.json` | passed: unbound proof-shaped evidence now fails because required `issuer_ref`, `signed_statement_sha256_hex`, `evidence_sha256_hex`, and `expires_at` fields are absent |
| 2026-06-29 | production origin-proof wrong digest guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-binding-wrong-digest-preflight.json` | passed: production preflight rejects proof objects whose `evidence_sha256_hex` does not match the evidence body without `production_origin_proof` |
| 2026-06-29 | production origin-proof bound parser path | `make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-binding-bound-preflight.json` | passed: content-bound proof-shaped evidence passes preflight parser coverage with `production_origin_proof_valid=true`; this is not real external evidence |
| 2026-06-29 | production origin-proof content-binding handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/content-binding-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof content-binding scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof content-binding fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof content-binding | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof content-binding clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof content-binding | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signed-statement red-first | `make receipt-production-evidence-production-preflight-check ...` with temporary non-repo content-bound KMS/HSM and compliance JSON carrying arbitrary `signed_statement_sha256_hex` but no embedded statement | passed as test-first evidence before the guard: production preflight incorrectly accepted content-bound proof without a locally checkable signed-statement body |
| 2026-06-29 | production origin-proof missing statement guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON report `target/receipt-production-evidence/proof-statement-after-guard-preflight.json` | passed: the same evidence now fails because issuer key/signature fields are missing, `proof_sha256_hex` is unbound, and `signed_statement` is absent |
| 2026-06-29 | production origin-proof wrong statement digest guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-statement-wrong-digest-preflight.json` | passed: production preflight rejects proof objects whose `signed_statement_sha256_hex` does not match the embedded statement |
| 2026-06-29 | production origin-proof signed-statement bound parser path | `make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-statement-bound-preflight.json` | passed: proof, signed-statement, signature metadata, and evidence digest bindings pass parser coverage with `production_origin_proof_valid=true`; this is not real external evidence |
| 2026-06-29 | production origin-proof signed-statement handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/statement-binding-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof signed-statement scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof signed-statement fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof signed-statement binding | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof signed-statement binding clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof signed-statement binding | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signed-statement whitespace | `git diff --check` | passed |
| 2026-06-29 | production origin-proof signature-ref-only red-first | `make receipt-production-evidence-production-preflight-check ...` with temporary non-repo proof/statement-bound KMS/HSM and compliance JSON carrying only `signature_ref`/`signature_sha256_hex` metadata | passed as test-first evidence before the guard: production preflight incorrectly accepted proof without `issuer_public_key_hex`, `signature_hex`, or cryptographic statement verification |
| 2026-06-29 | production origin-proof signature-ref-only guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-signature-ref-after-guard-preflight.json` | passed: the same evidence now fails because issuer public-key/signature bytes are missing and `signature_sha256_hex` is no longer allowed inside signed statement bytes |
| 2026-06-29 | production origin-proof bad signature guard | `! make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-signature-bad-preflight.json` | passed: production preflight rejects proof objects whose `signature_hex` does not verify `signed_statement` with `issuer_public_key_hex` |
| 2026-06-29 | production origin-proof verified signature parser path | `make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-signature-verified-preflight.json` | passed: proof, statement, evidence digest, raw signature digest, and Ed25519 signature verification pass parser coverage with `production_origin_proof_valid=true`; this is still not real external evidence |
| 2026-06-29 | production origin-proof signature helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof signature handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/signature-verification-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof signature scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof signature fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof signature verification | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof signature verification clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof signature verification | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signature whitespace | `git diff --check` | passed |
| 2026-06-29 | production origin-proof issuer-key-attestation self-key guard | `if make receipt-production-evidence-production-preflight-check ...; then exit 1; else echo EXPECTED_FAIL; fi` with temporary non-repo KMS/HSM and compliance JSON carrying a verified statement signature but no issuer key attestation | passed: strict production preflight now rejects self-asserted issuer keys and reports missing `issuer_key_attestation`/`key_attestor_*` fields |
| 2026-06-29 | production origin-proof issuer-key-attestation bad signature guard | `if make receipt-production-evidence-production-preflight-check ...; then exit 1; else echo EXPECTED_FAIL; fi` with temporary non-repo proof JSON carrying an invalid key-attestation signature | passed: strict production preflight rejects proof objects whose `key_attestation_signature_hex` does not verify `issuer_key_attestation` with `key_attestor_public_key_hex` |
| 2026-06-29 | production origin-proof issuer-key-attestation bound parser path | `make receipt-production-evidence-production-preflight-check ...`; JSON assertions over `target/receipt-production-evidence/proof-key-attestation-bound-current-preflight.json` | passed: proof, issuer key attestation, statement, evidence digest, raw signature digests, and both Ed25519 signature checks pass parser coverage with `production_origin_proof_valid=true`; this is still not real external evidence |
| 2026-06-29 | production origin-proof issuer-key-attestation handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/key-attestation-current-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof issuer-key-attestation scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof issuer-key-attestation signature helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof issuer-key-attestation fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof issuer-key-attestation | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof issuer-key-attestation clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof issuer-key-attestation | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof key-attestor trust-anchor missing-input guard | `if make receipt-production-evidence-production-preflight-check ...; then exit 1; else echo EXPECTED_FAIL; fi` with signed temporary non-repo KMS/HSM and compliance proof JSON but no `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*` inputs | passed: strict production preflight now fails fast with all four missing expected key-attestor trust-anchor inputs |
| 2026-06-29 | production origin-proof key-attestor trust-anchor mismatch guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID=wrong-root-key ...; then exit 1; else echo EXPECTED_FAIL; fi` | passed: both KMS/HSM and compliance proof validation reject `key_attestor_key_id` that does not match the expected trust anchor |
| 2026-06-29 | production origin-proof key-attestor trust-anchor bound parser path | `make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=<matching values>`; JSON assertions over `target/receipt-production-evidence/proof-attestor-anchor-bound-current-preflight.json` | passed: strict preflight accepts matching expected attestor key id, public key, attestor ref, and public-key ref while keeping this as parser coverage, not real external evidence |
| 2026-06-29 | production origin-proof key-attestor trust-anchor non-strict parser path | `make receipt-production-evidence-preflight-check ...` without `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*` inputs | passed: non-strict preflight remains usable for parser/inventory coverage without strict production trust-anchor inputs |
| 2026-06-29 | production origin-proof key-attestor trust-anchor handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/attestor-anchor-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof key-attestor trust-anchor scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof key-attestor trust-anchor fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof key-attestor trust-anchor | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof key-attestor trust-anchor clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof key-attestor trust-anchor | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof trust-anchor publication missing-input guard | `if make receipt-production-evidence-production-preflight-check ...; then exit 1; else echo EXPECTED_FAIL; fi` with signed temporary non-repo KMS/HSM and compliance proof JSON but no `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE` | passed: strict production preflight now fails fast on the missing trust-anchor publication evidence input |
| 2026-06-29 | production origin-proof trust-anchor publication local-origin guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=target/receipt-production-evidence/local-probes/trust-anchor.json ...; then exit 1; else echo EXPECTED_FAIL; fi` | passed: generated-local trust-anchor evidence is rejected as non-operator-origin even when its fields match expected values |
| 2026-06-29 | production origin-proof trust-anchor publication mismatch guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<mismatched-json> ...; then exit 1; else echo EXPECTED_FAIL; fi` | passed: trust-anchor evidence whose `key_attestor_key_id` does not match the expected trust anchor is rejected |
| 2026-06-29 | production origin-proof trust-anchor publication bound parser path | `make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<matching-json> ...`; JSON assertions over `target/receipt-production-evidence/proof-trust-anchor-bound-current-preflight.json` | passed: strict preflight accepts a matching operator-origin-shaped trust-anchor publication artifact, KMS/HSM proof, and compliance proof; this is still parser coverage, not real external evidence |
| 2026-06-29 | production origin-proof trust-anchor publication non-strict parser path | `make receipt-production-evidence-preflight-check ...` without trust-anchor evidence | passed: non-strict preflight remains usable for parser/inventory coverage without strict publication evidence |
| 2026-06-29 | production origin-proof trust-anchor publication handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-publication-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof trust-anchor publication scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof trust-anchor publication fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof trust-anchor publication | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof trust-anchor publication clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof trust-anchor publication | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signed trust-anchor unsigned-publication guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<unsigned-json> ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-production-evidence/proof-trust-anchor-unsigned-current-preflight.json` | passed: strict preflight rejects trust-anchor publication evidence without `signature_hex` and `signature_sha256_hex` |
| 2026-06-29 | production origin-proof signed trust-anchor bad-signature guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<bad-signature-json> ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-production-evidence/proof-trust-anchor-bad-signature-current-preflight.json` | passed: strict preflight rejects a trust-anchor publication whose signature does not verify with `publisher_public_key_hex` |
| 2026-06-29 | production origin-proof signed trust-anchor publisher mismatch guard | `if make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID=wrong-publisher ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-production-evidence/proof-trust-anchor-publisher-mismatch-current-preflight.json` | passed: strict preflight rejects a trust-anchor publication whose publisher binding does not match the expected publisher inputs |
| 2026-06-29 | production origin-proof signed trust-anchor bound parser path | `make receipt-production-evidence-production-preflight-check ... RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE=<signed-matching-json> ...`; JSON assertions over `target/receipt-production-evidence/proof-trust-anchor-signed-current-preflight.json` | passed: strict preflight accepts a signed trust-anchor publication whose publisher and key-attestor fields match the separately supplied expected inputs; this is parser coverage, not real external trust registry evidence |
| 2026-06-29 | production origin-proof signed trust-anchor non-strict parser path | `make receipt-production-evidence-preflight-check ...` without trust-anchor evidence | passed: non-strict preflight remains usable for parser/inventory coverage without strict signed trust-anchor publication evidence |
| 2026-06-29 | production origin-proof signed trust-anchor handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-signature-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof signed trust-anchor crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof signed trust-anchor scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof signed trust-anchor fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof signed trust-anchor | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof signed trust-anchor clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof signed trust-anchor | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signed trust-anchor diff hygiene | `git diff --check` | passed |
| 2026-06-29 | receipt KMS/HSM runtime signing probe missing guard | `if make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<missing-probe-json> ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-kms-hsm-custody/kms-runtime-probe-missing.json` | passed: KMS/HSM custody evidence without `runtime_signing_probe` is rejected |
| 2026-06-29 | receipt KMS/HSM runtime signing probe bad-signature guard | `if make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<bad-signature-json> ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-kms-hsm-custody/kms-runtime-probe-bad-signature.json` | passed: KMS/HSM custody evidence whose probe signature does not verify with `public_key_hex` is rejected |
| 2026-06-29 | receipt KMS/HSM runtime signing probe binding mismatch guard | `if make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<binding-mismatch-json> ...; then exit 1; else echo EXPECTED_FAIL; fi`; JSON assertions over `target/receipt-kms-hsm-custody/kms-runtime-probe-binding-mismatch.json` | passed: KMS/HSM custody evidence whose probe `key_id` does not match the top-level runtime key is rejected |
| 2026-06-29 | receipt KMS/HSM runtime signing probe bound parser path | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<signed-matching-json> ...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-runtime-probe-signed.json` | passed: a signed operator-shaped probe validates and sets the component report to `kms_hsm_custody=true`; this is parser coverage only, not real external operator evidence |
| 2026-06-29 | receipt KMS/HSM runtime signing probe default inventory | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-kms-hsm-custody/kms-runtime-probe-default.json` | passed: default report remains inventory-only with KMS/HSM custody unclaimed |
| 2026-06-29 | receipt KMS/HSM runtime signing probe fixture parser coverage | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json ...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-runtime-probe-fixture.json` | passed: fixture is schema-valid with a signed probe but remains synthetic parser coverage and does not set `kms_hsm_custody=true` |
| 2026-06-29 | receipt KMS/HSM runtime signing probe handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-runtime-probe-handoff-consistency.json` | passed |
| 2026-06-29 | receipt KMS/HSM runtime signing probe crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | receipt KMS/HSM runtime signing probe scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | receipt KMS/HSM runtime signing probe fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM runtime signing probe | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM runtime signing probe clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after receipt KMS/HSM runtime signing probe | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | receipt KMS/HSM runtime signing probe diff hygiene | `git diff --check` | passed |
| 2026-06-29 | receipt KMS/HSM component production-origin red-first | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<operator-shaped-no-production-proof-json> ...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-component-proof-red-pre.json` before the guard | failed as expected before implementation: signed runtime probe evidence without `production_origin_proof` incorrectly set `kms_hsm_custody=true` |
| 2026-06-29 | receipt KMS/HSM component production-origin no-proof guard | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<operator-shaped-no-production-proof-json> ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-component-proof-no-proof-after.json` | passed: no-proof operator-shaped evidence fails and keeps `kms_hsm_custody=false` / `production_safe=false` |
| 2026-06-29 | receipt KMS/HSM component production-origin missing-expected guard | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<operator-shaped-with-production-proof-json> ...` without expected key-attestor inputs; JSON assertions over `target/receipt-kms-hsm-custody/kms-component-proof-missing-expected.json` | passed: proof-bearing evidence without separately supplied expected key-attestor inputs fails and keeps `kms_hsm_custody=false` |
| 2026-06-29 | receipt KMS/HSM component production-origin bound parser path | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=<operator-shaped-with-production-proof-json> ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-component-proof-bound.json` | passed: proof-bound operator-shaped parser artifact can set the component report to `kms_hsm_custody=true`; this remains parser coverage only, not real external operator evidence |
| 2026-06-29 | receipt KMS/HSM component production-origin default inventory | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-kms-hsm-custody/kms-component-proof-default.json`; JSON assertions | passed: default inventory remains green with `kms_hsm_custody=false` and the evidence blocker |
| 2026-06-29 | receipt KMS/HSM component production-origin fixture guard | `make receipt-kms-hsm-custody-check ... RECEIPT_KMS_HSM_CUSTODY_EVIDENCE=fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/receipt-kms-hsm-custody/kms-component-proof-fixture-after.json` | passed: fixture remains synthetic/non-production and keeps `kms_hsm_custody=false` |
| 2026-06-29 | receipt KMS/HSM component production-origin handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-component-proof-handoff-consistency.json` | passed |
| 2026-06-29 | receipt KMS/HSM component production-origin crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | receipt KMS/HSM component production-origin scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | receipt KMS/HSM component production-origin fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM component production-origin | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM component production-origin clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after receipt KMS/HSM component production-origin | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | receipt KMS/HSM component production-origin diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance component production-origin red-first | `make compliance-boundary-check ... COMPLIANCE_CERTIFICATION_EVIDENCE=<operator-shaped-no-production-proof-json> ...`; JSON assertions over `target/compliance-boundary/compliance-component-proof-red-pre.json` before the guard | failed as expected before implementation: operator-shaped certification evidence without `production_origin_proof` incorrectly set `compliance_immutability=true` |
| 2026-06-29 | compliance component production-origin no-proof guard | `make compliance-boundary-check ... COMPLIANCE_CERTIFICATION_EVIDENCE=<operator-shaped-no-production-proof-json> ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/compliance-boundary/compliance-component-proof-no-proof-after.json` | passed: no-proof operator-shaped evidence fails and keeps `supported_certified_frameworks=[]` / `compliance_immutability=false` |
| 2026-06-29 | compliance component production-origin missing-expected guard | `make compliance-boundary-check ... COMPLIANCE_CERTIFICATION_EVIDENCE=<operator-shaped-with-production-proof-json> ...` without expected key-attestor inputs; JSON assertions over `target/compliance-boundary/compliance-component-proof-proof-missing-expected.json` | passed: proof-bearing evidence without separately supplied expected key-attestor inputs fails and keeps `compliance_immutability=false` |
| 2026-06-29 | compliance component production-origin bound parser path | `make compliance-boundary-check ... COMPLIANCE_CERTIFICATION_EVIDENCE=<operator-shaped-with-production-proof-json> ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/compliance-boundary/compliance-component-proof-bound.json` | passed: proof-bound operator-shaped parser artifact can set the component report to `supported_certified_frameworks=["soc2_type_ii"]` and `compliance_immutability=true`; this remains parser coverage only, not real external compliance evidence |
| 2026-06-29 | compliance component production-origin default inventory | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/compliance-boundary/compliance-component-proof-default.json`; JSON assertions | passed: default inventory remains green with `supported_certified_frameworks=[]`, `compliance_immutability=false`, and the evidence blocker |
| 2026-06-29 | compliance component production-origin fixture guard | `make compliance-boundary-check ... COMPLIANCE_CERTIFICATION_EVIDENCE=fixtures/compliance/certification_evidence.valid.json ... RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*=...`; JSON assertions over `target/compliance-boundary/compliance-component-proof-fixture-after.json` | passed: fixture remains synthetic/non-production and keeps `compliance_immutability=false` |
| 2026-06-29 | compliance component production-origin handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-component-proof-handoff-consistency.json` | passed |
| 2026-06-29 | compliance component production-origin crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | compliance component production-origin scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/compliance_certification_evidence.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | compliance component production-origin fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance component production-origin | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance component production-origin clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance component production-origin | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance component production-origin diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof independent publisher red-first | direct `validate_trust_anchor_evidence(...)` over a temporary signed self-published trust-anchor JSON; JSON assertions over `target/receipt-production-evidence/trust-anchor-independent-publisher-red-pre.json` before the guard | failed as expected before implementation: the validator accepted a trust-anchor publication whose publisher key id, public key, ref, and public-key ref were identical to the key-attestor identity |
| 2026-06-29 | production origin-proof independent publisher self-published guard | direct `validate_trust_anchor_evidence(...)` over the same signed self-published trust-anchor JSON; JSON assertions over `target/receipt-production-evidence/trust-anchor-independent-publisher-self-after.json` | passed: self-published trust-anchor evidence is rejected with distinct-publisher failures |
| 2026-06-29 | production origin-proof independent publisher bound parser path | direct `validate_trust_anchor_evidence(...)` over a temporary signed trust-anchor JSON with separate attestor and publisher keys; JSON assertions over `target/receipt-production-evidence/trust-anchor-independent-publisher-bound.json` | passed: an independently signed operator-shaped parser artifact remains valid; this is parser coverage only, not real external trust-registry evidence |
| 2026-06-29 | production origin-proof independent publisher handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-independent-publisher-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof independent publisher split handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-independent-publisher-handoff-consistency-after-split.json` | passed |
| 2026-06-29 | production origin-proof independent publisher scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof independent publisher crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof independent publisher module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py` | passed: touched Python modules are 62 / 164 / 203 / 281 / 286 lines |
| 2026-06-29 | production origin-proof independent publisher fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof independent publisher | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof independent publisher clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof independent publisher | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof independent publisher diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence distinct-artifacts red-first | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with duplicated `evidence_artifacts`; JSON assertions over `target/receipt-production-evidence/duplicate-artifacts-red-pre.json` before the guard | failed as expected before implementation: all three validators accepted duplicate artifact entries as satisfying the two-artifact requirement |
| 2026-06-29 | production evidence distinct-artifacts guard | direct KMS/HSM, compliance, and trust-anchor validator calls over the same duplicate-artifact JSON; JSON assertions over `target/receipt-production-evidence/duplicate-artifacts-after.json` | passed: duplicate artifact entries are rejected and require distinct artifact URIs plus distinct artifact digests |
| 2026-06-29 | production evidence distinct-artifacts positive regression | direct validator calls over `fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json`, `fixtures/compliance/certification_evidence.valid.json`, and a temporary signed trust-anchor JSON with distinct artifact URIs/digests; JSON assertions over `target/receipt-production-evidence/distinct-artifacts-positive.json` | passed: schema-valid fixtures and independently signed trust-anchor parser artifact with distinct artifacts remain parser-valid while still non-production/parser coverage |
| 2026-06-29 | production evidence distinct-artifacts handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/distinct-artifacts-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence distinct-artifacts scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence distinct-artifacts module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py` | passed: touched Python modules are 115 / 164 / 190 / 249 / 246 / 284 / 288 lines |
| 2026-06-29 | production evidence distinct-artifacts fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence distinct-artifacts | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence distinct-artifacts clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence distinct-artifacts | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence distinct-artifacts diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence top-level expiry red-first | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with `valid_until=2026-01-01T00:00:00Z`; JSON assertions over `target/receipt-production-evidence/expired-evidence-red-pre.json` before the guard | failed as expected before implementation: all three validators accepted expired top-level evidence windows |
| 2026-06-29 | production evidence top-level expiry guard | direct KMS/HSM, compliance, and trust-anchor validator calls over the same expired evidence; JSON assertions over `target/receipt-production-evidence/expired-evidence-after.json` | passed: expired `operator_attestation.valid_until`, `external_review.valid_until`, and trust-anchor `valid_until` are rejected |
| 2026-06-29 | production evidence top-level expiry positive regression | direct validator calls over fresh KMS/HSM and compliance fixtures plus a temporary signed trust-anchor JSON with `valid_until=2099-01-01T00:00:00Z`; JSON assertions over `target/receipt-production-evidence/fresh-trust-anchor-after-expiry-guard.json` and `target/receipt-production-evidence/fresh-evidence-positive.json` | passed: fresh top-level windows remain parser-valid while still synthetic/parser coverage where applicable |
| 2026-06-29 | production evidence top-level expiry handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/top-level-expiry-handoff-consistency.json`; JSON assertions over the handoff report | passed: handoff consistency remains green with the fresh `valid_until` checklist requirements |
| 2026-06-29 | production evidence top-level expiry scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence top-level expiry module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py` | passed: touched Python modules are 135 / 191 / 251 / 240 / 292 / 289 lines |
| 2026-06-29 | production evidence top-level expiry fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence top-level expiry | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence top-level expiry clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence top-level expiry | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence top-level expiry diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof expiry red-first | direct KMS/HSM and compliance validator calls over temporary proof-bound JSON with `production_origin_proof.expires_at=2026-01-01T00:00:00Z`; JSON assertions over `target/receipt-production-evidence/expired-origin-proof-red-pre.json` before the guard | failed as expected before implementation: expired `production_origin_proof.expires_at` was accepted when it was still after `issued_at` |
| 2026-06-29 | production origin-proof expiry guard | direct KMS/HSM and compliance validator calls over the same expired proof-bound evidence; JSON assertions over `target/receipt-production-evidence/expired-origin-proof-after.json` | passed: expired `production_origin_proof.expires_at` is rejected and marks `production_origin_proof_valid=false` |
| 2026-06-29 | production origin-proof expiry positive regression | direct KMS/HSM and compliance validator calls over fresh proof-bound parser artifacts with `production_origin_proof.expires_at=2099-01-01T00:00:00Z`; JSON assertions over `target/receipt-production-evidence/fresh-origin-proof-after.json` | passed: fresh proof-bound parser artifacts remain valid while still not being real external operator evidence |
| 2026-06-29 | production origin-proof expiry handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/origin-proof-expiry-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof expiry scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof expiry fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof expiry | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof expiry clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof expiry | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof expiry diff hygiene | `git diff --check` | passed |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness red-first | direct KMS/HSM validator call over temporary operator-origin evidence with `runtime_signing_probe.signed_at=2026-06-27T00:00:00Z`; JSON assertions over `target/receipt-production-evidence/stale-runtime-probe-red-pre.json` before the guard | failed as expected before implementation: stale runtime signer probe evidence was accepted as schema-valid |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness guard | direct KMS/HSM validator calls over stale and fresh temporary operator-origin evidence; JSON assertions over `target/receipt-production-evidence/stale-runtime-probe-after.json` | passed: operator-origin runtime signer probes older than 24 hours are rejected, fresh probes remain valid, and synthetic fixtures remain parser-valid |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/runtime-probe-freshness-handoff-consistency.json` | passed |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness scripts | `python3 -m py_compile scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_evidence.py scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness module sizes | `wc -l scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py` | passed: touched runtime/handoff Python modules are 292 / 252 / 296 / 290 lines |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM runtime probe freshness | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM runtime probe freshness clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after receipt KMS/HSM runtime probe freshness | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | receipt KMS/HSM runtime probe freshness diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence future timestamp red-first | direct KMS/HSM, compliance, trust-anchor, and production-origin proof validator calls over temporary JSON with future `issued_at`/`published_at`; JSON assertions over `target/receipt-production-evidence/future-issued-at-red-pre.json` before the guard | failed as expected before implementation: all four future-dated evidence timestamps were accepted |
| 2026-06-29 | production evidence future timestamp guard | direct KMS/HSM, compliance, trust-anchor, and production-origin proof validator calls over the same future-dated evidence; JSON assertions over `target/receipt-production-evidence/future-issued-at-after.json` | passed: `operator_attestation.issued_at`, `external_review.issued_at`, trust-anchor `published_at`, and `production_origin_proof.issued_at` are rejected when more than 300 seconds in the future |
| 2026-06-29 | production evidence future timestamp positive regression | direct validator calls over current parser artifacts plus synthetic parser fixtures; JSON assertions over `target/receipt-production-evidence/future-issued-at-positive.json` | passed: current issue/publication timestamps and parser fixtures remain valid |
| 2026-06-29 | production evidence future timestamp handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/future-issued-at-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence future timestamp scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence future timestamp bounded module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py` | passed: bounded touched modules are 152 / 254 / 242 / 193 / 292 / 300 / 298 lines; shared `scripts/evidence_origin.py` remains an existing oversized validator module |
| 2026-06-29 | production evidence future timestamp fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence future timestamp | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence future timestamp clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence future timestamp | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence future timestamp diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof signed-statement detached signature red-first | generated temporary proof-bound KMS/HSM and compliance JSON whose embedded `signed_statement` also carried `signature_hex`; direct `production_origin_proof_failures` assertions over `target/receipt-production-evidence/signed-statement-signature-field-red-pre.json` before the guard | failed as expected before implementation: proof-bound parser artifacts with `signed_statement.signature_hex` were accepted |
| 2026-06-29 | production origin-proof signed-statement detached signature guard | direct validator calls over the same temporary proof-bound JSON; JSON assertions over `target/receipt-production-evidence/signed-statement-signature-field-after.json` | passed: embedded `production_origin_proof.signed_statement.signature_hex` is rejected while the detached proof signature remains outside the statement |
| 2026-06-29 | production origin-proof signed-statement detached signature positive regression | generated temporary proof-bound KMS/HSM and compliance JSON with clean detached statement signatures; JSON assertions over `target/receipt-production-evidence/signed-statement-signature-field-positive.json` | passed: clean proof-bound parser artifacts remain valid |
| 2026-06-29 | production origin-proof signed-statement detached signature scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof signed-statement detached signature crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof signed-statement detached signature handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/signed-statement-signature-field-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof signed-statement detached signature module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: handoff modules remain 300 / 298 lines; shared `scripts/evidence_origin.py` and status/security docs remain intentionally oversized |
| 2026-06-29 | production origin-proof signed-statement detached signature fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof signed-statement detached signature | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof signed-statement detached signature clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof signed-statement detached signature | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof signed-statement detached signature diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof nested digest detachment red-first | generated temporary proof-bound KMS/HSM and compliance JSON whose embedded `issuer_key_attestation` and `signed_statement` also carried their own detached digest fields; direct `production_origin_proof_failures` assertions over `target/receipt-production-evidence/nested-digest-fields-red-pre.json` before the guard | failed as expected before implementation: nested `issuer_key_attestation_sha256_hex` and `signed_statement_sha256_hex` fields were accepted |
| 2026-06-29 | production origin-proof nested digest detachment guard | direct validator calls over the same temporary proof-bound JSON; JSON assertions over `target/receipt-production-evidence/nested-digest-fields-after.json` | passed: `production_origin_proof.issuer_key_attestation.issuer_key_attestation_sha256_hex` and `production_origin_proof.signed_statement.signed_statement_sha256_hex` are rejected |
| 2026-06-29 | production origin-proof nested digest detachment positive regression | generated temporary proof-bound KMS/HSM and compliance JSON with clean detached digest fields; JSON assertions over `target/receipt-production-evidence/nested-digest-fields-positive.json` | passed: clean proof-bound parser artifacts remain valid |
| 2026-06-29 | production origin-proof nested digest detachment scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof nested digest detachment crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof nested digest detachment handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/nested-digest-fields-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof nested digest detachment module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: handoff modules are 300 / 300 lines; shared `scripts/evidence_origin.py` and status/security docs remain intentionally oversized |
| 2026-06-29 | production origin-proof nested digest detachment fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof nested digest detachment | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof nested digest detachment clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof nested digest detachment | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof nested digest detachment diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof nested closed-shape red-first | generated temporary proof-bound KMS/HSM and compliance JSON whose embedded `issuer_key_attestation` and `signed_statement` carried signed `unreviewed_extension` fields; direct `production_origin_proof_failures` assertions over `target/receipt-production-evidence/nested-extra-fields-red-pre.json` before the guard | failed as expected before implementation: nested signed objects with fields outside the v1 schemas were accepted |
| 2026-06-29 | production origin-proof nested closed-shape guard | direct validator calls over the same temporary proof-bound JSON; JSON assertions over `target/receipt-production-evidence/nested-extra-fields-after.json` | passed: extra `issuer_key_attestation.unreviewed_extension` and `signed_statement.unreviewed_extension` fields are rejected |
| 2026-06-29 | production origin-proof nested closed-shape positive regression | generated temporary proof-bound KMS/HSM and compliance JSON with only the required nested v1 fields; JSON assertions over `target/receipt-production-evidence/nested-extra-fields-positive.json` | passed: clean proof-bound parser artifacts remain valid |
| 2026-06-29 | production origin-proof nested closed-shape scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof nested closed-shape crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof nested closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/nested-extra-fields-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof nested closed-shape module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: handoff modules are 300 / 299 lines; shared `scripts/evidence_origin.py` and status/security docs remain intentionally oversized |
| 2026-06-29 | production origin-proof nested closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof nested closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof nested closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof nested closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof nested closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production origin-proof top-level closed-shape red-first | generated temporary proof-bound KMS/HSM and compliance JSON whose top-level `production_origin_proof` carried a signed `unreviewed_extension` field; direct `production_origin_proof_failures` assertions over `target/receipt-production-evidence/proof-top-level-extra-field-red-pre.json` before the guard | failed as expected before implementation: top-level proof fields outside the v1 schema were accepted |
| 2026-06-29 | production origin-proof top-level closed-shape guard | direct validator calls over the same temporary proof-bound JSON; JSON assertions over `target/receipt-production-evidence/proof-top-level-extra-field-after.json` | passed: extra `production_origin_proof.unreviewed_extension` is rejected |
| 2026-06-29 | production origin-proof top-level closed-shape positive regression | generated temporary proof-bound KMS/HSM and compliance JSON with only required top-level proof fields; JSON assertions over `target/receipt-production-evidence/proof-top-level-extra-field-positive.json` | passed: clean proof-bound parser artifacts remain valid |
| 2026-06-29 | production origin-proof top-level closed-shape scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production origin-proof top-level closed-shape crypto helper | `cargo test -p cortex-crypto --bin production_origin_signature` | passed |
| 2026-06-29 | production origin-proof top-level closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-top-level-extra-field-handoff-consistency.json` | passed |
| 2026-06-29 | production origin-proof top-level closed-shape module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: handoff modules are 300 / 300 lines; shared `scripts/evidence_origin.py` and status/security docs remain intentionally oversized |
| 2026-06-29 | production origin-proof top-level closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production origin-proof top-level closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production origin-proof top-level closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production origin-proof top-level closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production origin-proof top-level closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape red-first | direct KMS/HSM validator call over temporary schema-valid evidence with `unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/kms-top-level-extra-field-red-pre.json` before the guard | failed as expected before implementation: top-level custody fields outside the v1 schema were accepted |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape guard | direct KMS/HSM validator call over the same temporary evidence; JSON assertions over `target/receipt-production-evidence/kms-top-level-extra-field-after.json` | passed: extra `kms_hsm_custody_evidence.unreviewed_extension` is rejected |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape positive regression | direct KMS/HSM validator call over `fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/kms-top-level-extra-field-positive.json` | passed: clean KMS/HSM fixture remains valid |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape component gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-production-evidence/kms-top-level-extra-field-custody-check.json` | passed |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-top-level-extra-field-handoff-consistency.json` | passed |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape module sizes | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: KMS/HSM validator is 273 lines and handoff payload remains 300 lines; shared handoff check and docs remain intentionally oversized |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM custody top-level closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM custody top-level closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after receipt KMS/HSM custody top-level closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | receipt KMS/HSM custody top-level closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance certification top-level closed-shape red-first | direct compliance validator call over temporary schema-valid evidence with `unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/compliance-top-level-extra-field-red-pre.json` before the guard | failed as expected before implementation: top-level certification fields outside the v1 schema were accepted |
| 2026-06-29 | compliance certification top-level closed-shape guard | direct compliance validator call over the same temporary evidence; JSON assertions over `target/receipt-production-evidence/compliance-top-level-extra-field-after.json` | passed: extra `compliance_certification_evidence.unreviewed_extension` is rejected |
| 2026-06-29 | compliance certification top-level closed-shape positive regression | direct compliance validator call over `fixtures/compliance/certification_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/compliance-top-level-extra-field-positive.json` | passed: clean compliance fixture remains valid |
| 2026-06-29 | compliance certification top-level closed-shape scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | compliance certification top-level closed-shape component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/compliance-top-level-extra-field-boundary-check.json` | passed |
| 2026-06-29 | compliance certification top-level closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-top-level-extra-field-handoff-consistency.json` | passed |
| 2026-06-29 | compliance certification top-level closed-shape module sizes | `wc -l scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: compliance validator is 258 lines and handoff payload is 299 lines; shared handoff check and docs remain intentionally oversized |
| 2026-06-29 | compliance certification top-level closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance certification top-level closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance certification top-level closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance certification top-level closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance certification top-level closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance certification nested closed-shape red-first | direct compliance validator call over temporary schema-valid evidence whose `external_review`, `scope`, and `immutability_evidence` objects carried `unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/compliance-nested-extra-fields-red-pre.json` before the guard | failed as expected before implementation: nested certification fields outside the v1 schemas were accepted |
| 2026-06-29 | compliance certification nested closed-shape guard | direct compliance validator call over the same temporary evidence; JSON assertions over `target/receipt-production-evidence/compliance-nested-extra-fields-after.json` | passed: extra `external_review.unreviewed_extension`, `scope.unreviewed_extension`, and `immutability_evidence.unreviewed_extension` are rejected |
| 2026-06-29 | compliance certification nested closed-shape positive regression | direct compliance validator call over `fixtures/compliance/certification_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/compliance-nested-extra-fields-positive.json` | passed: clean compliance fixture remains valid |
| 2026-06-29 | compliance certification nested closed-shape scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | compliance certification nested closed-shape component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/compliance-nested-extra-fields-boundary-check.json` | passed |
| 2026-06-29 | compliance certification nested closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-nested-extra-fields-handoff-consistency.json` | passed |
| 2026-06-29 | compliance certification nested closed-shape module sizes | `wc -l scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: compliance validator is 285 lines and handoff payload is 294 lines; shared handoff check and docs remain intentionally oversized |
| 2026-06-29 | compliance certification nested closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance certification nested closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance certification nested closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance certification nested closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance certification nested closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape red-first | direct KMS/HSM validator call over temporary schema-valid evidence whose `runtime_binding`, `runtime_signing_probe`, and `operator_attestation` objects carried `unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/kms-nested-extra-fields-red-pre.json` before the guard | failed as expected before implementation: nested custody fields outside the v1 schemas were accepted |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape guard | direct KMS/HSM validator call over the same temporary evidence; JSON assertions over `target/receipt-production-evidence/kms-nested-extra-fields-after.json` | passed: extra `runtime_binding.unreviewed_extension`, `runtime_signing_probe.unreviewed_extension`, and `operator_attestation.unreviewed_extension` are rejected |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape positive regression | direct KMS/HSM validator call over `fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/kms-nested-extra-fields-positive.json` | passed: clean KMS/HSM fixture remains valid |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_custody_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape component gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-production-evidence/kms-nested-extra-fields-custody-check.json` | passed |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-nested-extra-fields-handoff-consistency.json` | passed |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape module sizes | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: KMS/HSM validator is 291 lines, runtime probe validator is 299 lines, and handoff payload is 298 lines; shared handoff check and docs remain intentionally oversized |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM custody nested closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after receipt KMS/HSM custody nested closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after receipt KMS/HSM custody nested closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | receipt KMS/HSM custody nested closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence artifact item closed-shape red-first | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with `evidence_artifacts[0].unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/artifact-extra-fields-red-pre.json` before the guard | failed as expected before implementation: all three validators accepted extra artifact item fields |
| 2026-06-29 | production evidence artifact item closed-shape guard | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with `evidence_artifacts[0].unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/artifact-extra-fields-after.json` | passed: all three validators reject `evidence_artifacts[0].unreviewed_extension is not allowed` |
| 2026-06-29 | production evidence artifact item closed-shape positive regression | direct validator calls over clean KMS/HSM and compliance fixtures plus a clean signed trust-anchor parser artifact; JSON assertions over `target/receipt-production-evidence/artifact-extra-fields-positive.json` | passed: clean KMS/HSM fixture, compliance fixture, and signed trust-anchor parser artifact remain valid |
| 2026-06-29 | production evidence artifact item closed-shape scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence artifact item closed-shape KMS/HSM component gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-production-evidence/artifact-extra-fields-kms-custody-check.json` | passed |
| 2026-06-29 | production evidence artifact item closed-shape compliance component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/artifact-extra-fields-compliance-boundary-check.json` | passed |
| 2026-06-29 | production evidence artifact item closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/artifact-extra-fields-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence artifact item closed-shape module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared artifact validator is 158 lines, handoff requirements is 300 lines, handoff payload is 299 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production evidence artifact item closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence artifact item closed-shape first run | `cargo test --workspace --all-features` | failed once in existing `tests::cluster_ingress_discovery_tests::context_route_discovers_raft_leader_then_forwards_to_ingress_address`; the touched Python/docs paths are not on this Rust path |
| 2026-06-29 | workspace transient follow-up | `cargo test -p cortex-server tests::cluster_ingress_discovery_tests::context_route_discovers_raft_leader_then_forwards_to_ingress_address -- --exact` | passed |
| 2026-06-29 | workspace after production evidence artifact item closed-shape rerun | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence artifact item closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence artifact item closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence artifact item closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence artifact item closed-shape untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape red-first | direct trust-anchor validator call over a signed temporary trust-anchor publication with top-level `unreviewed_extension`; JSON assertions over `target/receipt-production-evidence/trust-anchor-top-level-extra-field-red-pre.json` before the guard | failed as expected before implementation: signed top-level trust-anchor fields outside the v1 schema were accepted |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape guard | direct trust-anchor validator call over the same signed temporary trust-anchor publication; JSON assertions over `target/receipt-production-evidence/trust-anchor-top-level-extra-field-after.json` | passed: extra `production_origin_trust_anchor_evidence.unreviewed_extension` is rejected |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape positive regression | direct trust-anchor validator call over a clean signed trust-anchor parser artifact; JSON assertions over `target/receipt-production-evidence/trust-anchor-top-level-extra-field-positive-report.json` | passed: clean signed trust-anchor parser artifact remains valid |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape scripts | `python3 -m py_compile scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_preflight.py` | passed |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-top-level-extra-field-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape module sizes | `wc -l scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: trust-anchor validator is 218 lines, handoff requirements is 300 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin trust-anchor top-level closed-shape | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin trust-anchor top-level closed-shape clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin trust-anchor top-level closed-shape | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin trust-anchor top-level closed-shape untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production evidence controls closed-set red-first | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with `unreviewed_control_claim` plus a duplicate required control; JSON assertions over `target/receipt-production-evidence/controls-closed-set-red-pre.json` before the guard | failed as expected before implementation: all three validators accepted extra and duplicate controls |
| 2026-06-29 | production evidence controls closed-set guard | direct KMS/HSM, compliance, and trust-anchor validator calls over the same temporary JSON; JSON assertions over `target/receipt-production-evidence/controls-closed-set-after.json` | passed: all three validators reject unsupported controls and duplicate controls |
| 2026-06-29 | production evidence controls closed-set positive regression | direct validator calls over clean KMS/HSM and compliance fixtures plus a clean signed trust-anchor parser artifact; JSON assertions over `target/receipt-production-evidence/controls-closed-set-positive.json` | passed: clean KMS/HSM fixture, compliance fixture, and signed trust-anchor parser artifact remain valid |
| 2026-06-29 | production evidence controls closed-set scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence controls closed-set KMS/HSM component gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-production-evidence/controls-closed-set-kms-custody-check.json` | passed |
| 2026-06-29 | production evidence controls closed-set compliance component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/controls-closed-set-compliance-boundary-check.json` | passed |
| 2026-06-29 | production evidence controls closed-set handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/controls-closed-set-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence controls closed-set module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 181 lines, KMS/HSM validator is 291 lines, compliance validator is 286 lines, trust-anchor validator is 219 lines, handoff payload is 299 lines, handoff requirements is 300 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production evidence controls closed-set fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence controls closed-set | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence controls closed-set clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence controls closed-set | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence controls closed-set diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence controls closed-set untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | compliance operator responsibilities closed-set red-first | direct compliance validator call over temporary JSON with `unreviewed operator responsibility claim` plus a duplicate required responsibility; JSON assertions over `target/receipt-production-evidence/compliance-responsibilities-closed-set-red-pre.json` before the guard | failed as expected before implementation: compliance evidence accepted extra and duplicate operator responsibilities |
| 2026-06-29 | compliance operator responsibilities closed-set guard | direct compliance validator call over the same temporary JSON; JSON assertions over `target/receipt-production-evidence/compliance-responsibilities-closed-set-after.json` | passed: unsupported and duplicate operator responsibilities are rejected |
| 2026-06-29 | compliance operator responsibilities closed-set positive regression | direct compliance validator call over `fixtures/compliance/certification_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/compliance-responsibilities-closed-set-positive.json` | passed: the clean compliance fixture remains valid |
| 2026-06-29 | compliance operator responsibilities closed-set scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | compliance operator responsibilities closed-set component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/compliance-responsibilities-closed-set-compliance-boundary-check.json` | passed |
| 2026-06-29 | compliance operator responsibilities closed-set handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-responsibilities-closed-set-handoff-consistency.json` | passed |
| 2026-06-29 | compliance operator responsibilities closed-set module sizes | `wc -l scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: compliance validator is 298 lines, handoff payload is 300 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | compliance operator responsibilities closed-set fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance operator responsibilities closed-set | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance operator responsibilities closed-set clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance operator responsibilities closed-set | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance operator responsibilities closed-set diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance operator responsibilities closed-set untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production evidence non-local references red-first | direct KMS/HSM and compliance validator calls over temporary JSON with `target/...` signer/report/immutability refs; JSON assertions over `target/receipt-production-evidence/top-level-local-reference-red-pre.json` before the guard | failed as expected before implementation: KMS/HSM and compliance evidence accepted local/generated reference fields as parser-valid |
| 2026-06-29 | production evidence non-local references guard | direct KMS/HSM and compliance validator calls over the same temporary JSON; JSON assertions over `target/receipt-production-evidence/top-level-local-reference-after.json` | passed: local/generated signer, report, retention-policy, and tamper-evidence refs are rejected |
| 2026-06-29 | production evidence non-local references positive regression | direct validator calls over clean KMS/HSM and compliance fixtures; JSON assertions over `target/receipt-production-evidence/top-level-local-reference-positive.json` | passed: clean KMS/HSM and compliance fixtures remain valid parser coverage |
| 2026-06-29 | production evidence non-local references scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence non-local references KMS/HSM component gate | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-production-evidence/top-level-local-reference-kms-custody-check.json` | passed |
| 2026-06-29 | production evidence non-local references compliance component gate | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/receipt-production-evidence/top-level-local-reference-compliance-boundary-check.json` | passed |
| 2026-06-29 | production evidence non-local references handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/top-level-local-reference-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence non-local references module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 192 lines, KMS/HSM validator is 295 lines, compliance validator is 291 lines, handoff payload is 297 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production evidence non-local references fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence non-local references | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence non-local references clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence non-local references | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence non-local references diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence non-local references untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production trust-anchor non-local references red-first | direct trust-anchor validator call over signed temporary JSON with `target/...` top-level trust-anchor refs; JSON assertions over `target/receipt-production-evidence/trust-anchor-local-reference-red-pre.json` before the guard | failed as expected before implementation: the trust-anchor evidence accepted local/generated top-level refs as parser-valid |
| 2026-06-29 | production trust-anchor non-local references guard | direct trust-anchor validator call over the same signed temporary JSON; JSON assertions over `target/receipt-production-evidence/trust-anchor-local-reference-after.json` | passed: local/generated key-attestor, publisher, publication, and signature refs are rejected |
| 2026-06-29 | production trust-anchor non-local references positive regression | direct trust-anchor validator call over a clean signed temporary JSON with external HTTPS top-level refs; JSON assertions over `target/receipt-production-evidence/trust-anchor-local-reference-positive.json` | passed: clean signed trust-anchor parser evidence remains valid |
| 2026-06-29 | production trust-anchor non-local references scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | production trust-anchor non-local references handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-local-reference-handoff-consistency.json` | passed |
| 2026-06-29 | production trust-anchor non-local references module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 192 lines, trust-anchor validator is 234 lines, handoff requirements is 285 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production trust-anchor non-local references fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production trust-anchor non-local references | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production trust-anchor non-local references clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production trust-anchor non-local references | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production trust-anchor non-local references diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production trust-anchor non-local references untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production artifact-kind closed-set red-first | direct KMS/HSM, compliance, and trust-anchor validator calls over temporary JSON with `unreviewed_*` artifact kind values; JSON assertions over `target/receipt-production-evidence/artifact-kind-closed-set-red-pre.json` before the guard | failed as expected before implementation: all three validators accepted arbitrary artifact kind values as parser-valid |
| 2026-06-29 | production artifact-kind closed-set guard | direct KMS/HSM, compliance, and trust-anchor validator calls over the same temporary JSON; JSON assertions over `target/receipt-production-evidence/artifact-kind-closed-set-after.json` | passed: unsupported artifact kinds are rejected across KMS/HSM, compliance, and trust-anchor evidence |
| 2026-06-29 | production artifact-kind closed-set positive regression | direct validator calls over the clean KMS/HSM fixture, compliance fixture, and clean signed trust-anchor parser artifact; JSON assertions over `target/receipt-production-evidence/artifact-kind-closed-set-positive.json` | passed: supported artifact kind values remain valid |
| 2026-06-29 | production artifact-kind closed-set scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production artifact-kind closed-set handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/artifact-kind-closed-set-handoff-consistency.json` | passed |
| 2026-06-29 | production artifact-kind closed-set module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 197 lines, KMS/HSM validator is 300 lines, compliance validator is 296 lines, trust-anchor validator is 239 lines, handoff payload is 300 lines, handoff requirements is 287 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | production artifact-kind closed-set fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production artifact-kind closed-set | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production artifact-kind closed-set clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production artifact-kind closed-set | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production artifact-kind closed-set diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production artifact-kind closed-set untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin reviewer independence red-first | direct compliance validator call over a signed production-origin proof with `reviewed_by == issuer_ref`; JSON assertions over `target/receipt-production-evidence/proof-reviewer-independence-red-pre.json` before the guard | failed as expected before implementation: the validator accepted a self-reviewed production-origin proof |
| 2026-06-29 | production-origin reviewer independence guard | direct compliance validator call over the same signed production-origin proof; JSON assertions over `target/receipt-production-evidence/proof-reviewer-independence-after.json` | passed: `production_origin_proof.reviewed_by must be distinct from issuer_ref` is rejected |
| 2026-06-29 | production-origin reviewer independence positive regression | direct compliance validator call over a signed production-origin proof with an independent reviewer; JSON assertions over `target/receipt-production-evidence/proof-reviewer-independence-positive.json` | passed: independent `reviewed_by` remains valid with matching proof digest, key-attestation signature, and statement signature |
| 2026-06-29 | production-origin reviewer independence scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production-origin reviewer independence handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-reviewer-independence-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin reviewer independence module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: proof validator is 648 lines, handoff requirements is 292 lines, handoff check is 559 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin reviewer independence fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin reviewer independence | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin reviewer independence clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin reviewer independence | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin reviewer independence diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin reviewer independence untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin issuer-attestor independence red-first | direct compliance validator call over a signed production-origin proof where issuer and key-attestor identities were the same ref/key/public-key; JSON assertions over `target/receipt-production-evidence/proof-issuer-attestor-independence-red-pre.json` before the guard | failed as expected before implementation: the validator accepted a self-attested production-origin proof |
| 2026-06-29 | production-origin issuer-attestor independence guard | direct compliance validator call over the same signed production-origin proof; JSON assertions over `target/receipt-production-evidence/proof-issuer-attestor-independence-after.json` | passed: matching issuer/key-attestor ref, key id, public-key ref, and public-key hex are rejected |
| 2026-06-29 | production-origin issuer-attestor independence positive regression | direct compliance validator call over a signed production-origin proof with separate issuer and key-attestor identities; JSON assertions over `target/receipt-production-evidence/proof-issuer-attestor-independence-positive.json` | passed: independent issuer/key-attestor identities remain valid with matching proof digest, key-attestation signature, and statement signature |
| 2026-06-29 | production-origin issuer-attestor independence scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production-origin issuer-attestor independence handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-issuer-attestor-independence-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin issuer-attestor independence module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: proof validator is 670 lines, handoff requirements is 298 lines, handoff check is 564 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin issuer-attestor independence fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin issuer-attestor independence | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin issuer-attestor independence clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin issuer-attestor independence | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin issuer-attestor independence diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin issuer-attestor independence untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production evidence artifact digest lowercase red-first | direct KMS/HSM and compliance validator calls over parser artifacts with uppercase `evidence_artifacts[0].sha256_hex`; JSON assertions over `target/receipt-production-evidence/artifact-digest-lowercase-red-pre.json` before the guard | failed as expected before implementation: both validators accepted uppercase artifact digests |
| 2026-06-29 | production evidence artifact digest lowercase guard | direct KMS/HSM and compliance validator calls over the same parser artifacts; JSON assertions over `target/receipt-production-evidence/artifact-digest-lowercase-after.json` | passed: uppercase artifact digests are rejected as non-lowercase SHA-256 hex |
| 2026-06-29 | production evidence artifact digest lowercase positive regression | direct KMS/HSM and compliance validator calls over clean fixtures; JSON assertions over `target/receipt-production-evidence/artifact-digest-lowercase-positive.json` | passed: lowercase artifact digest fixtures remain valid |
| 2026-06-29 | production evidence artifact digest lowercase scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence artifact digest lowercase handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/artifact-digest-lowercase-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence artifact digest lowercase module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 197 lines, KMS/HSM validator is 300 lines, compliance validator is 296 lines, trust-anchor validator is 239 lines, handoff requirements is 298 lines, handoff check is 564 lines, and docs remain intentionally oversized |
| 2026-06-29 | production evidence artifact digest lowercase fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence artifact digest lowercase | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence artifact digest lowercase clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence artifact digest lowercase | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence artifact digest lowercase diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence artifact digest lowercase untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | KMS/HSM runtime lowercase hex red-first | direct KMS/HSM validator call over a parser artifact with uppercase top-level `public_key_hex` and uppercase runtime probe `public_key_hex`, `canonical_header_hex`, request/response/signature digests, and `signature_hex`; JSON assertions over `target/receipt-production-evidence/kms-runtime-hex-lowercase-red-pre.json` before the guard | failed as expected before implementation: the validator accepted uppercase KMS/HSM runtime hex after lowercasing input strings |
| 2026-06-29 | KMS/HSM runtime lowercase hex guard | direct KMS/HSM validator call over the same parser artifact; JSON assertions over `target/receipt-production-evidence/kms-runtime-hex-lowercase-after.json` | passed: uppercase top-level/runtime public key, canonical header bytes, request/response/signature digests, and signature hex are rejected as non-lowercase hex |
| 2026-06-29 | KMS/HSM runtime lowercase hex positive regression | direct KMS/HSM validator call over `fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/kms-runtime-hex-lowercase-positive.json` | passed: lowercase KMS/HSM fixture remains valid parser coverage |
| 2026-06-29 | KMS/HSM runtime lowercase hex scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | KMS/HSM runtime lowercase hex handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-runtime-hex-lowercase-handoff-consistency.json` | passed |
| 2026-06-29 | KMS/HSM runtime lowercase hex module sizes | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: KMS/HSM validator is 300 lines, runtime probe is 297 lines, handoff payload is 287 lines, handoff check/docs remain intentionally oversized |
| 2026-06-29 | KMS/HSM runtime lowercase hex fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after KMS/HSM runtime lowercase hex | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after KMS/HSM runtime lowercase hex clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after KMS/HSM runtime lowercase hex | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | KMS/HSM runtime lowercase hex diff hygiene | `git diff --check` | passed |
| 2026-06-29 | KMS/HSM runtime lowercase hex untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | trust-anchor lowercase hex red-first | direct trust-anchor validator call over a signed parser artifact whose `key_attestor_public_key_hex`, `publisher_public_key_hex`, `signature_hex`, and `signature_sha256_hex` were uppercase in the signed JSON; JSON assertions over `target/receipt-production-evidence/trust-anchor-hex-lowercase-red-pre.json` before the guard | failed as expected before implementation: the validator accepted uppercase trust-anchor hex after lowercasing input strings |
| 2026-06-29 | trust-anchor lowercase hex guard | direct trust-anchor validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/trust-anchor-hex-lowercase-after.json` | passed: uppercase trust-anchor public-key, signature, and signature digest hex are rejected as non-lowercase hex |
| 2026-06-29 | trust-anchor lowercase hex positive regression | direct trust-anchor validator call over a freshly signed lowercase parser artifact; JSON assertions over `target/receipt-production-evidence/trust-anchor-hex-lowercase-positive.json` | passed: lowercase signed trust-anchor parser evidence remains valid |
| 2026-06-29 | trust-anchor lowercase hex scripts | `python3 -m py_compile scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | trust-anchor lowercase hex handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-hex-lowercase-handoff-consistency.json` | passed |
| 2026-06-29 | trust-anchor lowercase hex module sizes | `wc -l scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: trust-anchor validator is 239 lines, trust-anchor checks is 164 lines, handoff requirements is 298 lines, handoff payload is 287 lines, and handoff check/docs remain intentionally oversized |
| 2026-06-29 | trust-anchor lowercase hex fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after trust-anchor lowercase hex | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after trust-anchor lowercase hex clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after trust-anchor lowercase hex | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | trust-anchor lowercase hex diff hygiene | `git diff --check` | passed |
| 2026-06-29 | trust-anchor lowercase hex untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | expected public-key lowercase red-first | direct KMS/HSM validator, trust-anchor expected-binding helper, and production-origin proof expected-binding helper calls with uppercase expected public-key inputs; JSON assertions over `target/receipt-production-evidence/expected-public-key-lowercase-red-pre.json` before the guard | failed as expected before implementation: uppercase expected public-key inputs were accepted after lowercasing |
| 2026-06-29 | expected public-key lowercase guard | direct KMS/HSM validator, trust-anchor expected-binding helper, and production-origin proof expected-binding helper calls over the same uppercase expected inputs; JSON assertions over `target/receipt-production-evidence/expected-public-key-lowercase-after.json` | passed: uppercase expected runtime, key-attestor, and trust-anchor publisher public keys are rejected as non-lowercase hex |
| 2026-06-29 | expected public-key lowercase positive regression | direct KMS/HSM validator, trust-anchor expected-binding helper, and production-origin proof expected-binding helper calls with lowercase expected public-key inputs; JSON assertions over `target/receipt-production-evidence/expected-public-key-lowercase-positive.json` | passed: lowercase expected public-key inputs remain accepted |
| 2026-06-29 | expected public-key lowercase scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | expected public-key lowercase handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/expected-public-key-lowercase-handoff-consistency.json` | passed |
| 2026-06-29 | expected public-key lowercase module sizes | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: KMS/HSM validator is 295 lines, trust-anchor checks is 164 lines, proof validator is 670 lines, handoff payload is 290 lines, handoff requirements is 298 lines, and handoff check/docs remain intentionally oversized |
| 2026-06-29 | expected public-key lowercase fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after expected public-key lowercase | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after expected public-key lowercase clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after expected public-key lowercase | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | expected public-key lowercase diff hygiene | `git diff --check` | passed |
| 2026-06-29 | expected public-key lowercase untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_kms_hsm_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | evidence hex whitespace red-first | direct KMS/HSM and trust-anchor validator calls over parser artifacts with leading/trailing whitespace around KMS/HSM public key/runtime hex, trust-anchor public-key/signature hex, and artifact digests; JSON assertions over `target/receipt-production-evidence/evidence-hex-whitespace-red-pre.json` before the guard | failed as expected before implementation: both validators accepted whitespace-padded hex after stripping input strings |
| 2026-06-29 | evidence hex whitespace guard | direct KMS/HSM and trust-anchor validator calls over the same parser artifacts; JSON assertions over `target/receipt-production-evidence/evidence-hex-whitespace-after.json` | passed: whitespace-padded evidence hex is rejected instead of normalized |
| 2026-06-29 | evidence hex whitespace positive regression | direct KMS/HSM and trust-anchor validator calls over clean lowercase parser artifacts; JSON assertions over `target/receipt-production-evidence/evidence-hex-whitespace-positive.json` | passed: clean lowercase evidence remains valid parser coverage |
| 2026-06-29 | evidence hex whitespace scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | evidence hex whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/evidence-hex-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | evidence hex whitespace module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 201 lines, KMS/HSM validator is 297 lines, runtime probe is 299 lines, trust-anchor validator is 239 lines, trust-anchor checks is 164 lines, handoff payload is 290 lines, handoff requirements is 298 lines, and handoff check/docs remain intentionally oversized |
| 2026-06-29 | evidence hex whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after evidence hex whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after evidence hex whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after evidence hex whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | evidence hex whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | evidence hex whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | compliance string whitespace red-first | direct compliance validator call over a parser artifact with leading/trailing whitespace around schema, framework, report, reviewer, scope, timing, and immutability reference strings; JSON assertions over `target/receipt-production-evidence/compliance-string-whitespace-red-pre.json` before the guard | failed as expected before implementation: compliance evidence strings were accepted after stripping input strings |
| 2026-06-29 | compliance string whitespace guard | direct compliance validator call over the same parser artifact; JSON assertions over `target/receipt-production-evidence/compliance-string-whitespace-after.json` | passed: whitespace-padded compliance evidence strings are rejected instead of normalized |
| 2026-06-29 | compliance string whitespace positive regression | direct compliance validator call over `fixtures/compliance/certification_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/compliance-string-whitespace-positive.json` | passed: clean compliance fixture remains valid parser coverage |
| 2026-06-29 | compliance string whitespace scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | compliance string whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-string-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | compliance string whitespace module sizes | `wc -l scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: compliance validator is 298 lines, handoff payload is 291 lines, and handoff check/docs remain intentionally oversized |
| 2026-06-29 | compliance string whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance string whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance string whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance string whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance string whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance string whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin proof string whitespace red-first | generated proof-bound KMS/HSM parser artifact with leading/trailing whitespace around `production_origin_proof` identity, reference, and reviewer strings, including matching signed `issuer_key_attestation` and `signed_statement`; JSON assertions over `target/receipt-production-evidence/proof-string-whitespace-red-pre.json` before the guard | failed as expected before implementation: proof and KMS/HSM validators accepted signed non-canonical proof strings |
| 2026-06-29 | production-origin proof string whitespace guard | direct proof and KMS/HSM validator calls over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/proof-string-whitespace-after.json` | passed: whitespace-padded production-origin proof identity, reference, and reviewer strings are rejected instead of normalized |
| 2026-06-29 | production-origin proof string whitespace positive regression | generated clean proof-bound KMS/HSM parser artifact without surrounding whitespace; JSON assertions over `target/receipt-production-evidence/proof-string-whitespace-positive.json` | passed: clean signed production-origin proof parser coverage remains valid |
| 2026-06-29 | production-origin proof string whitespace scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | production-origin proof string whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-string-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin proof string whitespace module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: proof validator is 672 lines, handoff requirements is 302 lines, handoff check is 613 lines, handoff payload is 291 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin proof string whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin proof string whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin proof string whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin proof string whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin proof string whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin proof string whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin proof key-id whitespace red-first | generated proof-bound KMS/HSM parser artifact with internal whitespace in signed `production_origin_proof.issuer_key_id` and `production_origin_proof.key_attestor_key_id`; JSON assertions over `target/receipt-production-evidence/proof-key-id-whitespace-red-pre.json` before the guard | failed as expected before implementation: proof and KMS/HSM validators accepted signed proof key ids with internal whitespace |
| 2026-06-29 | production-origin proof key-id whitespace guard | direct KMS/HSM validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/proof-key-id-whitespace-after.json` | passed: signed proof key ids with internal whitespace are rejected instead of accepted as canonical identifiers |
| 2026-06-29 | production-origin proof key-id whitespace positive regression | generated clean proof-bound KMS/HSM parser artifact with whitespace-free proof key ids; JSON assertions over `target/receipt-production-evidence/proof-key-id-whitespace-positive.json` | passed: clean signed production-origin proof parser coverage remains valid |
| 2026-06-29 | production-origin proof key-id whitespace scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | production-origin proof key-id whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-key-id-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin proof key-id whitespace module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: proof validator is 676 lines, handoff requirements is 303 lines, handoff check is 618 lines, handoff payload is 291 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin proof key-id whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin proof key-id whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin proof key-id whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin proof key-id whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin proof key-id whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin proof key-id whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin proof reference whitespace red-first | generated proof-bound KMS/HSM parser artifact with internal whitespace in signed `production_origin_proof.proof_ref`; JSON assertions over `target/receipt-production-evidence/proof-reference-whitespace-red-pre.json` before the guard | failed as expected before implementation: proof and KMS/HSM validators accepted a signed proof reference with internal whitespace |
| 2026-06-29 | production-origin proof reference whitespace guard | direct KMS/HSM validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/proof-reference-whitespace-after.json` | passed: signed proof references with raw whitespace are rejected instead of accepted as canonical external references |
| 2026-06-29 | production-origin proof reference whitespace positive regression | generated clean proof-bound KMS/HSM parser artifact with whitespace-free proof references; JSON assertions over `target/receipt-production-evidence/proof-reference-whitespace-positive.json` | passed: clean signed production-origin proof parser coverage remains valid |
| 2026-06-29 | production-origin proof reference whitespace scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | production-origin proof reference whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-reference-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin proof reference whitespace module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: proof validator is 681 lines, handoff requirements is 304 lines, handoff check is 624 lines, handoff payload is 291 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin proof reference whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin proof reference whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin proof reference whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin proof reference whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin proof reference whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin proof reference whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | KMS/HSM signer-ref whitespace red-first | generated KMS/HSM parser artifact with internal whitespace in top-level `signer_ref`, matching `runtime_signing_probe.signer_ref`, and recomputed runtime request digest; JSON assertions over `target/receipt-production-evidence/kms-signer-ref-whitespace-red-pre.json` before the guard | failed as expected before implementation: KMS/HSM validator accepted the signed runtime signer reference with internal whitespace |
| 2026-06-29 | KMS/HSM signer-ref whitespace guard | direct KMS/HSM validator call over the same parser artifact; JSON assertions over `target/receipt-production-evidence/kms-signer-ref-whitespace-after.json` | passed: top-level `signer_ref` and nested `runtime_signing_probe.signer_ref` with raw whitespace are rejected |
| 2026-06-29 | KMS/HSM signer-ref whitespace positive regression | direct KMS/HSM validator call over `fixtures/accountability_receipt/kms_hsm_custody_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/kms-signer-ref-whitespace-positive.json` | passed: clean KMS/HSM custody fixture remains valid parser coverage |
| 2026-06-29 | KMS/HSM signer-ref whitespace scripts | `python3 -m py_compile scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_requirements.py` | passed |
| 2026-06-29 | KMS/HSM signer-ref whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/kms-signer-ref-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | KMS/HSM signer-ref whitespace module sizes | `wc -l scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: KMS/HSM validator is 303 lines, runtime probe validator is 301 lines, handoff payload is 293 lines, and shared handoff check/docs remain intentionally oversized |
| 2026-06-29 | KMS/HSM signer-ref whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after KMS/HSM signer-ref whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after KMS/HSM signer-ref whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after KMS/HSM signer-ref whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | KMS/HSM signer-ref whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | KMS/HSM signer-ref whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_kms_hsm_evidence.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | compliance reference whitespace red-first | direct compliance validator call over a parser artifact with internal whitespace in `report_ref`; JSON assertions over `target/receipt-production-evidence/compliance-reference-whitespace-red-pre.json` before the guard | failed as expected before implementation: compliance validator accepted the non-canonical report reference with internal whitespace |
| 2026-06-29 | compliance reference whitespace guard | direct compliance validator call over the same parser artifact; JSON assertions over `target/receipt-production-evidence/compliance-reference-whitespace-after.json` | passed: `report_ref` with raw whitespace is rejected |
| 2026-06-29 | compliance reference whitespace positive regression | direct compliance validator call over `fixtures/compliance/certification_evidence.valid.json`; JSON assertions over `target/receipt-production-evidence/compliance-reference-whitespace-positive.json` | passed: clean compliance fixture remains valid parser coverage |
| 2026-06-29 | compliance reference whitespace scripts | `python3 -m py_compile scripts/compliance_certification_evidence.py scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_requirements.py` | passed |
| 2026-06-29 | compliance reference whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/compliance-reference-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | compliance reference whitespace module sizes | `wc -l scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: compliance validator is 304 lines, handoff payload is 296 lines, handoff check is 640 lines, and docs remain intentionally oversized |
| 2026-06-29 | compliance reference whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after compliance reference whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after compliance reference whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after compliance reference whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | compliance reference whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | compliance reference whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | artifact URI whitespace red-first | direct KMS/HSM and compliance validator calls over parser artifacts with internal whitespace in `evidence_artifacts[0].uri`; JSON assertions over `target/receipt-production-evidence/artifact-uri-whitespace-red-pre.json` before the guard | failed as expected before implementation: both validators accepted non-canonical artifact URIs with raw whitespace |
| 2026-06-29 | artifact URI whitespace guard | direct KMS/HSM and compliance validator calls over the same parser artifacts; JSON assertions over `target/receipt-production-evidence/artifact-uri-whitespace-after.json` | passed: `evidence_artifacts[0].uri` with raw whitespace is rejected by the shared artifact validator |
| 2026-06-29 | artifact URI whitespace positive regression | direct KMS/HSM and compliance validator calls over clean fixtures; JSON assertions over `target/receipt-production-evidence/artifact-uri-whitespace-positive.json` | passed: clean KMS/HSM and compliance fixture artifact URIs remain valid parser coverage |
| 2026-06-29 | artifact URI whitespace scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | artifact URI whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/artifact-uri-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | artifact URI whitespace module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 203 lines, KMS/HSM validator is 303 lines, compliance validator is 304 lines, trust-anchor validator is 239 lines, handoff payload is 298 lines, handoff requirements is 305 lines, handoff check is 646 lines, and docs remain intentionally oversized |
| 2026-06-29 | artifact URI whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after artifact URI whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after artifact URI whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after artifact URI whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | artifact URI whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | artifact URI whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | trust-anchor reference whitespace red-first | generated signature-valid trust-anchor parser artifact with internal whitespace in signed `publication_ref`; JSON assertions over `target/receipt-production-evidence/trust-anchor-reference-whitespace-red-pre.json` before the guard | failed as expected before implementation: trust-anchor validator accepted the signed non-canonical publication reference |
| 2026-06-29 | trust-anchor reference whitespace guard | direct trust-anchor validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/trust-anchor-reference-whitespace-after.json` | passed: signed `publication_ref` with raw whitespace is rejected |
| 2026-06-29 | trust-anchor reference whitespace positive regression | generated clean re-signed trust-anchor parser artifact without reference whitespace; JSON assertions over `target/receipt-production-evidence/trust-anchor-reference-whitespace-positive.json` | passed: clean signed trust-anchor reference parser coverage remains valid |
| 2026-06-29 | trust-anchor reference whitespace scripts | `python3 -m py_compile scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/operator_evidence_validation.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | trust-anchor reference whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-reference-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | trust-anchor reference whitespace module sizes | `wc -l scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: trust-anchor validator is 247 lines, handoff requirements is 309 lines, handoff check is 652 lines, and docs remain intentionally oversized |
| 2026-06-29 | trust-anchor reference whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after trust-anchor reference whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after trust-anchor reference whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after trust-anchor reference whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | trust-anchor reference whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | trust-anchor reference whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | trust-anchor key-id whitespace red-first | generated signature-valid trust-anchor parser artifact with internal whitespace in signed `key_attestor_key_id`; JSON assertions over `target/receipt-production-evidence/trust-anchor-key-id-whitespace-red-pre.json` before the guard | failed as expected before implementation: trust-anchor validator accepted the signed non-canonical key id |
| 2026-06-29 | trust-anchor key-id whitespace guard | direct trust-anchor validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/trust-anchor-key-id-whitespace-after.json` | passed: signed `key_attestor_key_id` with raw whitespace is rejected |
| 2026-06-29 | trust-anchor key-id whitespace positive regression | generated clean re-signed trust-anchor parser artifact without key-id whitespace; JSON assertions over `target/receipt-production-evidence/trust-anchor-key-id-whitespace-positive.json` | passed: clean signed trust-anchor key ids remain valid parser coverage |
| 2026-06-29 | trust-anchor key-id whitespace scripts | `python3 -m py_compile scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_origin_trust_anchor_checks.py scripts/operator_evidence_validation.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | trust-anchor key-id whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/trust-anchor-key-id-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | trust-anchor key-id whitespace module sizes | `wc -l scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: trust-anchor validator is 253 lines, handoff requirements is 310 lines, handoff check is 657 lines, and docs remain intentionally oversized |
| 2026-06-29 | trust-anchor key-id whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after trust-anchor key-id whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after trust-anchor key-id whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after trust-anchor key-id whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | trust-anchor key-id whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | trust-anchor key-id whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production-origin proof reviewer whitespace red-first | generated signed compliance parser artifact with internal whitespace in signed `production_origin_proof.reviewed_by`; JSON assertions over `target/receipt-production-evidence/proof-reviewer-whitespace-red-pre.json` before the guard | failed as expected before implementation: compliance validator accepted a signed non-canonical reviewer identity |
| 2026-06-29 | production-origin proof reviewer whitespace guard | direct compliance validator call over the same signed parser artifact; JSON assertions over `target/receipt-production-evidence/proof-reviewer-whitespace-after.json` | passed: signed `production_origin_proof.reviewed_by` with raw whitespace is rejected |
| 2026-06-29 | production-origin proof reviewer whitespace positive regression | generated clean signed compliance parser artifact with `reviewed_by=external-reviewer`; JSON assertions over `target/receipt-production-evidence/proof-reviewer-whitespace-positive.json` | passed: clean signed proof reviewer identity remains valid parser coverage |
| 2026-06-29 | production-origin proof reviewer whitespace scripts | `python3 -m py_compile scripts/evidence_origin.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_evidence_handoff_payload.py` | passed |
| 2026-06-29 | production-origin proof reviewer whitespace handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/proof-reviewer-whitespace-handoff-consistency.json` | passed |
| 2026-06-29 | production-origin proof reviewer whitespace module sizes | `wc -l scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: evidence origin validator is 684 lines, handoff requirements is 311 lines, handoff check is 662 lines, and docs remain intentionally oversized |
| 2026-06-29 | production-origin proof reviewer whitespace fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production-origin proof reviewer whitespace | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production-origin proof reviewer whitespace clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production-origin proof reviewer whitespace | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production-origin proof reviewer whitespace diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production-origin proof reviewer whitespace untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/evidence_origin.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production evidence timezone-aware timestamp red-first | direct KMS/HSM, compliance, trust-anchor, and production-origin proof validator calls over timezone-less timestamp parser artifacts; JSON assertions over `target/receipt-production-evidence/timestamp-timezone-red-pre.json` before the guard | failed as expected before implementation: KMS/HSM, compliance, and trust-anchor validators accepted timezone-less evidence timestamps, and production-origin proof validation crashed on naive/aware datetime comparison |
| 2026-06-29 | production evidence timezone-aware timestamp guard | direct validator calls over the same parser artifacts plus a runtime-probe timestamp case; JSON assertions over `target/receipt-production-evidence/timestamp-timezone-after.json` | passed: timezone-less evidence timestamps are rejected with timezone-aware ISO-8601 failures and production-origin proof validation no longer crashes |
| 2026-06-29 | production evidence timezone-aware timestamp positive regression | direct validator calls over timezone-aware KMS/HSM and compliance fixtures plus generated signed trust-anchor and proof parser artifacts; JSON assertions over `target/receipt-production-evidence/timestamp-timezone-positive.json` | passed: timezone-aware production evidence timestamps remain valid parser coverage |
| 2026-06-29 | production evidence timezone-aware timestamp scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence timezone-aware timestamp handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/timestamp-timezone-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence timezone-aware timestamp module sizes | `wc -l scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 207 lines, evidence origin validator is 687 lines, runtime probe validator is 304 lines, handoff payload is 301 lines, handoff requirements is 320 lines, handoff check is 690 lines, and docs remain intentionally oversized |
| 2026-06-29 | production evidence timezone-aware timestamp fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence timezone-aware timestamp | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence timezone-aware timestamp clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence timezone-aware timestamp | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production evidence timezone-aware timestamp diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production evidence timezone-aware timestamp untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/operator_evidence_validation.py scripts/evidence_origin.py scripts/receipt_kms_hsm_runtime_probe.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | current strict production evidence preflight boundary | `make receipt-production-evidence-production-preflight-check RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT=target/receipt-production-evidence/current-production-preflight.json` | failed as expected: strict preflight requires real KMS/HSM custody, compliance certification, trust-anchor, key-attestor, publisher, and runtime binding inputs before production-grade public receipt claims |
| 2026-06-29 | current production readiness inventory boundary | `make receipt-production-readiness-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-evidence/current-production-readiness.json`; JSON inspection | passed as an inventory gate with `production_ready=false`; blockers remain production evidence preflight, operator KMS/HSM custody evidence, and operator compliance evidence |
| 2026-06-29 | current strict production ready gate boundary | `make receipt-production-ready-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-evidence/current-production-ready-check.json` | failed as expected at `receipt-production-evidence-production-preflight-check` because the required real operator evidence inputs are absent |
| 2026-06-29 | production evidence normalized secret-field red-first | direct KMS/HSM and compliance validator calls over parser artifacts with optional non-strict `production_origin_proof.privateKey` and `production_origin_proof.apiToken`; JSON assertions over `target/receipt-production-evidence/secret-field-mixed-case-red-pre.json` before the guard | failed as expected before implementation: both parser artifacts were accepted as valid because mixed-case secret field names were not recognized and optional proof failures were not promoted in non-strict parser mode |
| 2026-06-29 | production evidence normalized secret-field guard | direct KMS/HSM and compliance validator calls over the same parser artifacts; JSON assertions over `target/receipt-production-evidence/secret-field-mixed-case-after.json` | passed: recursive forbidden secret detection now rejects normalized mixed-case `privateKey` and `apiToken` field names before evidence can validate |
| 2026-06-29 | production evidence normalized secret-field positive regression | direct validator calls over clean KMS/HSM and compliance fixtures; JSON assertions over `target/receipt-production-evidence/secret-field-mixed-case-positive.json` | passed: clean parser fixtures remain valid after normalized secret-field matching |
| 2026-06-29 | production evidence normalized secret-field scripts | `python3 -m py_compile scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py` | passed |
| 2026-06-29 | production evidence normalized secret-field handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/secret-field-mixed-case-handoff-consistency.json` | passed |
| 2026-06-29 | production evidence normalized secret-field module sizes | `wc -l scripts/operator_evidence_validation.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/receipt_production_origin_trust_anchor_evidence.py scripts/receipt_production_evidence_handoff_payload.py scripts/receipt_production_evidence_handoff_requirements.py scripts/receipt_production_evidence_handoff_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: shared validator is 235 lines, KMS/HSM validator is 284 lines, compliance validator is 291 lines, trust-anchor validator is 243 lines, handoff payload is 311 lines, handoff requirements is 325 lines, handoff check is 710 lines, and docs remain intentionally oversized |
| 2026-06-29 | production evidence normalized secret-field fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production evidence normalized secret-field | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production evidence normalized secret-field clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production evidence normalized secret-field | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production readiness aggregate proof-binding red-first | forged/weak component reports under `target/receipt-production-evidence/aggregate-proof-binding-red` with `production_origin_proof_required=false` and `production_origin_proof_valid=false`, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON inspection | failed as expected before implementation: strict readiness accepted the weak component reports and wrote `production_ready=true` with no blockers |
| 2026-06-29 | production readiness aggregate proof-binding guard | same forged/weak component reports, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON assertions over `target/receipt-production-evidence/aggregate-proof-binding-red/strict-after.json` | passed: strict readiness rejects component reports that do not carry required valid production-origin proof flags |
| 2026-06-29 | production readiness aggregate proof-binding positive regression | proof-bound positive aggregate fixture under `target/receipt-production-evidence/aggregate-proof-binding-positive`, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON assertions over `target/receipt-production-evidence/aggregate-proof-binding-positive/strict.json` | passed: strict readiness still accepts operator-origin component reports that carry required valid production-origin proof flags |
| 2026-06-29 | production readiness aggregate proof-binding scripts | `python3 -m py_compile scripts/receipt_production_readiness_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py scripts/receipt_kms_hsm_evidence.py scripts/compliance_certification_evidence.py scripts/evidence_origin.py scripts/operator_evidence_validation.py` | passed |
| 2026-06-29 | production readiness aggregate proof-binding current inventory | `make receipt-production-readiness-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-evidence/aggregate-proof-binding-current-readiness.json`; JSON inspection | passed as an inventory gate with `production_ready=false`; blockers remain production evidence preflight, operator KMS/HSM custody evidence, and operator compliance evidence |
| 2026-06-29 | production readiness aggregate proof-binding module sizes | `wc -l scripts/receipt_production_readiness_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: readiness aggregate is 361 lines, security model is 524 lines, and implementation status remains intentionally oversized |
| 2026-06-29 | production readiness aggregate proof-binding diff hygiene | `git diff --check` | passed |
| 2026-06-29 | production readiness aggregate proof-binding untracked whitespace hygiene | `rg -n "[[:blank:]]$" scripts/receipt_production_readiness_check.py docs/SECURITY_MODEL.md docs/ACCOUNTABILITY_IMPLEMENTATION_STATUS.md` | passed: no trailing whitespace in touched tracked or untracked files |
| 2026-06-29 | production readiness aggregate proof-binding fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production readiness aggregate proof-binding | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production readiness aggregate proof-binding clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production readiness aggregate proof-binding | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |
| 2026-06-29 | production component trust-anchor publication red/green | `PYTHONPATH=scripts python3 - <<'PY' ...` monkeypatched KMS/HSM and compliance component validators to return proof-bound operator evidence with and without trust-anchor inputs | passed: missing trust-anchor publication inputs keep `kms_hsm_custody=false` and `compliance_immutability=false`; trust-anchor-bound positive component paths still set the expected production booleans |
| 2026-06-29 | production component trust-anchor publication scripts | `python3 -m py_compile scripts/receipt_production_component_origin.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_readiness_check.py scripts/receipt_production_origin_trust_anchor_evidence.py` | passed |
| 2026-06-29 | production component trust-anchor publication KMS/HSM default | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-kms-hsm-custody/component-trust-anchor-default.json`; JSON inspection | passed: default inventory stays green with `kms_hsm_custody=false`, `production_safe=false`, and the KMS/HSM evidence blocker |
| 2026-06-29 | production component trust-anchor publication compliance default | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/compliance-boundary/component-trust-anchor-default.json`; JSON inspection | passed: default inventory stays green with `supported_certified_frameworks=[]`, `compliance_immutability=false`, and the external certification blocker |
| 2026-06-29 | production component trust-anchor publication handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/component-trust-anchor-handoff-consistency.json` | passed |
| 2026-06-29 | production readiness aggregate trust-anchor binding red-first | forged KMS/HSM and compliance component reports under `target/receipt-production-evidence/aggregate-trust-anchor-binding-red` with operator-origin proof flags but no component-level `production_origin_trust_anchor`, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready` before the guard | failed as expected before implementation: strict readiness accepted the forged component reports and wrote `production_ready=true` |
| 2026-06-29 | production readiness aggregate trust-anchor binding guard | same forged component reports, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON assertions over `target/receipt-production-evidence/aggregate-trust-anchor-binding-red/strict-after.json` | passed: strict readiness rejects component reports that omit component-level operator-origin trust-anchor validation |
| 2026-06-29 | production readiness aggregate trust-anchor binding positive regression | proof-bound positive aggregate fixture with component-level operator-origin trust-anchor reports under `target/receipt-production-evidence/aggregate-trust-anchor-binding-positive`, then `python3 scripts/receipt_production_readiness_check.py ... --require-production-ready`; JSON assertions | passed: strict readiness still accepts component reports that carry operator-origin proof flags and component-level trust-anchor validation |
| 2026-06-29 | production readiness aggregate trust-anchor binding scripts | `python3 -m py_compile scripts/receipt_production_readiness_check.py scripts/receipt_production_component_origin.py scripts/receipt_kms_hsm_custody_check.py scripts/compliance_boundary_check.py scripts/receipt_production_evidence_preflight.py scripts/receipt_production_evidence_handoff.py scripts/receipt_production_evidence_handoff_check.py scripts/receipt_production_origin_trust_anchor_evidence.py` | passed |
| 2026-06-29 | production readiness aggregate trust-anchor binding KMS/HSM default | `make receipt-kms-hsm-custody-check RECEIPT_KMS_HSM_CUSTODY_REPORT=target/receipt-kms-hsm-custody/aggregate-trust-anchor-binding-default.json` | passed: default inventory stays green with `kms_hsm_custody=false` |
| 2026-06-29 | production readiness aggregate trust-anchor binding compliance default | `make compliance-boundary-check COMPLIANCE_BOUNDARY_REPORT=target/compliance-boundary/aggregate-trust-anchor-binding-default.json` | passed: default inventory stays green with `compliance_immutability=false` |
| 2026-06-29 | production readiness aggregate trust-anchor binding handoff/current preflight | `make receipt-production-evidence-handoff-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_REPORT=target/receipt-production-evidence/real-operator-handoff-current.json`; `! make receipt-production-evidence-production-preflight-check RECEIPT_PRODUCTION_EVIDENCE_PREFLIGHT_REPORT=target/receipt-production-evidence/real-operator-preflight-current-after-guard.json` | passed: operator handoff report emits the real `receipt-production-ready-check` path, and strict preflight remains fail-closed with 15 missing external evidence inputs |
| 2026-06-29 | production readiness aggregate trust-anchor binding handoff consistency | `make receipt-production-evidence-handoff-consistency-check RECEIPT_PRODUCTION_EVIDENCE_HANDOFF_CONSISTENCY_REPORT=target/receipt-production-evidence/aggregate-trust-anchor-binding-handoff-consistency.json` | passed |
| 2026-06-29 | production readiness aggregate trust-anchor binding current inventory | `make receipt-production-readiness-check RECEIPT_PRODUCTION_READINESS_REPORT=target/receipt-production-evidence/aggregate-trust-anchor-binding-current-readiness.json`; JSON inspection | passed as an inventory gate with `production_ready=false`; KMS/HSM and compliance component trust-anchor readiness remain false without real operator evidence |
| 2026-06-29 | production readiness aggregate trust-anchor binding fmt | `cargo fmt --check` | passed |
| 2026-06-29 | workspace after production readiness aggregate trust-anchor binding | `cargo test --workspace --all-features` | passed |
| 2026-06-29 | workspace after production readiness aggregate trust-anchor binding clippy | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| 2026-06-29 | API after production readiness aggregate trust-anchor binding | `make openapi-contract-check` | passed; emitted existing OpenAPI coverage warnings but live responses, error taxonomy, generated SDK type artifacts, and SDK codegen control validated |

## Remaining In Order

1. Provide real operator KMS/HSM custody evidence and configure
   `receipt-kms-hsm-custody-check` with the expected runtime key id, public key,
   signer reference, and expected key-attestor trust-anchor inputs before
   claiming production-grade public receipt guarantees. The validator and make
   wiring now exist, but default runs still record
   `operator_kms_hsm_custody_evidence_not_implemented`; the aggregate
   `receipt-production-readiness-check` records this as
   `kms_hsm_receipt_key_custody`. Schema-valid fixture evidence is synthetic
   validator coverage only and is not operator-origin evidence. Top-level
   KMS/HSM custody evidence is a closed v1 shape: fields outside the documented
   schema plus `production_origin_proof` are rejected. Nested
   `runtime_binding`, `runtime_signing_probe`, and `operator_attestation`
   objects are also closed v1 shapes. Each `evidence_artifacts[]` item is also
   closed to `kind`, `uri`, and `sha256_hex`; extra artifact-level fields are
   rejected across KMS/HSM, compliance, and trust-anchor evidence. Artifact
   `sha256_hex` values must be exactly 64 lowercase hex characters; uppercase,
   surrounding whitespace, or otherwise non-canonical digests are rejected.
   Artifact `kind` values are also component-specific closed sets:
   `provider_key_policy`/`signer_deployment_config` for KMS/HSM,
   `redacted_external_report`/`immutability_attestation` for compliance, and
   `publication`/`publisher-key` for trust-anchor evidence. Required controls
   lists are also closed and duplicate-free across those evidence schemas, so
   extra control names and duplicate controls are rejected. Artifact
   `uri` values, KMS/HSM `provider_key_ref` and `signer_ref`, and trust-anchor
   `key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
   `publisher_public_key_ref`, `publication_ref`, and `signature_ref` must be
   non-local references; artifact `uri` values, KMS/HSM `provider_key_ref` and
   `signer_ref`, plus the trust-anchor reference fields must also contain no
   raw whitespace; trust-anchor `key_attestor_key_id` and `publisher_key_id`
   must contain no whitespace; generated, temporary, loopback, local transport,
   shell-local, and filesystem refs are rejected. Use
   `receipt-production-evidence-preflight-check` to fail fast on missing or
   synthetic custody evidence before running parser-coverage inventory checks.
   The strict `receipt-production-ready-check` uses
   `receipt-production-evidence-production-preflight-check`, so production
   readiness also requires separately supplied key-attestor trust-anchor
   publication evidence and expected inputs:
   `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF`.
   Expected public-key hex inputs, including
   `RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
   must themselves be 64 lowercase hex characters; uppercase or otherwise
   non-canonical expected inputs are rejected instead of normalized.
   The trust-anchor evidence must use schema
   `cortexdb.operator_evidence_origin_trust_anchor.v1`, type
   `key_attestor_publication`, `external_control_plane=true`, non-local
   `key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
   `publisher_public_key_ref`, `publication_ref`, and `signature_ref`,
   required controls, at least two distinct hashed artifacts with distinct URIs
   and digests, and key-attestor fields that match the expected inputs. Its
   `published_at` must not be more than 300 seconds in
   the future at validation time. Trust-anchor `published_at` and
   `valid_until` must be timezone-aware ISO-8601 timestamps. Strict preflight also
   requires a `production_origin_proof` object with schema
   `cortexdb.operator_evidence_origin_proof.v1`, external `proof_ref`,
   `issuer_ref`, `issuer_public_key_ref`, `issuer_key_attestation_ref`,
   `key_attestor_ref`, `key_attestor_public_key_ref`, `signed_statement_ref`,
   `signature_ref`, and `key_attestation_signature_ref` references,
   those proof reference fields must be non-local external references and must
   contain no raw whitespace,
   `issuer_key_id`, `issuer_public_key_hex`, `key_attestor_key_id`,
   `key_attestor_public_key_hex`, `signature_hex`,
   `key_attestation_signature_hex`, SHA-256 digests for the proof, embedded
   issuer key attestation, embedded signed statement, detached signature bytes,
   detached key-attestation signature bytes, and evidence body, `reviewed_by`,
   `issued_at`, `expires_at`, and `external_control_plane=true`. The proof's
   `expires_at` must be after `issued_at` and still in the future at validation
   time, and the proof's `issued_at` must not be more than 300 seconds in the
   future at validation time. Both proof timestamps must be timezone-aware
   ISO-8601. The proof's identity, reference, and reviewer
   string values must not include surrounding whitespace. The proof's
   `issuer_key_id`, `key_attestor_key_id`, and `reviewed_by` values must
   contain no whitespace.
   The proof's
   key-attestor fields must match the separately supplied expected trust-anchor
   inputs. The proof's issuer identity fields must be distinct from the
   corresponding key-attestor identity fields, so the issuer key cannot attest
   itself. The proof's `reviewed_by` value must be distinct from `issuer_ref`,
   `issuer_key_id`, `issuer_public_key_ref`, `key_attestor_ref`,
   `key_attestor_key_id`, and `key_attestor_public_key_ref`. The
   `evidence_sha256_hex` value must match the evidence JSON after removing
   `production_origin_proof` and serializing with sorted keys and compact
   separators. The proof must embed a
   `cortexdb.operator_evidence_origin_key_attestation.v1` key attestation whose
   canonical digest matches `issuer_key_attestation_sha256_hex` and whose
   issuer key, attestor key, timing, statement-signing-domain, and reference
   fields match the proof. `key_attestation_signature_hex` must verify that
   attestation with Ed25519 under `key_attestor_public_key_hex`, and
   `key_attestation_signature_sha256_hex` must match the raw signature bytes.
   The proof must also embed a
   `cortexdb.operator_evidence_origin_statement.v1` statement whose canonical
   digest matches `signed_statement_sha256_hex` and whose schema, evidence,
   issuer, public-key, timing, and reference fields match the proof.
   `signature_hex` must verify that statement with Ed25519 under
   `issuer_public_key_hex`, and `signature_sha256_hex` must match the raw
   signature bytes. The `proof_sha256_hex` value must match the proof object
   after removing `proof_sha256_hex`.
   The trust-anchor publication itself must also bind the separately supplied
   expected publisher inputs through `publisher_key_id`,
   `publisher_public_key_hex`, `publisher_ref`, and
   `publisher_public_key_ref`; must carry `signature_algorithm=ed25519`,
   `signature_ref`, `signature_hex`, and `signature_sha256_hex`; and
   `signature_hex` must verify against `publisher_public_key_hex` over
   `cortexdb.operator_evidence_origin_trust_anchor.sign.v1 || 0x00 ||
   canonical_json(trust_anchor_evidence_without_signature_hex_and_signature_sha256_hex)`.
   The trust-anchor `key_attestor_public_key_hex`, `publisher_public_key_hex`,
   `signature_hex`, and `signature_sha256_hex` values must be lowercase hex in
   the original evidence JSON; uppercase, surrounding whitespace, or otherwise
   non-canonical hex is rejected instead of normalized.
   The top-level trust-anchor evidence object is also a closed v1 shape: fields
   outside the documented schema are rejected instead of being treated as
   trust-registry publication claims.
   The publisher key id, public key, publisher ref, and publisher public-key
   ref must be distinct from the key-attestor identity fields; a self-published
   key-attestor anchor is not sufficient trust-registry evidence.
   KMS/HSM custody evidence must include `runtime_signing_probe` with matching
   `key_id`, `public_key_hex`, and whitespace-free `signer_ref`, canonical external signer
   request/response digests, `signature_hex`, `signature_sha256_hex`, and a
   signature that verifies over
   `cortexdb.accountability_receipt.sign.v1 || 0x00 ||
   canonical_header_hex bytes`. The top-level and runtime-probe
   `public_key_hex`, the probe `canonical_header_hex`, `request_sha256_hex`,
   `response_sha256_hex`, `signature_hex`, and `signature_sha256_hex` values
   must be lowercase hex in the original evidence JSON; uppercase, surrounding
   whitespace, or otherwise non-canonical hex is rejected instead of
   normalized. The runtime probe `signed_at` timestamp must be
   within 24 hours of validation time and no more than 300 seconds in the
   future, and it must be timezone-aware ISO-8601.
   `operator_attestation.issued_at` must not be more than 300 seconds
   in the future at validation time. `operator_attestation.issued_at` and
   `operator_attestation.valid_until` must be timezone-aware ISO-8601
   timestamps.
   The standalone component gate also requires a valid
   `production_origin_proof`, separately supplied
   `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`, expected key-attestor
   inputs, and expected trust-anchor publisher inputs before setting
   `kms_hsm_custody=true`; a signed runtime probe and proof-bound component
   JSON alone are not enough.
   Generated local artifacts under `target/` are also not operator-origin
   evidence, and neither are scratch files under `/tmp`, `/var/tmp`, or
   `/dev/shm`. Nested artifact references inside the evidence JSON must also
   avoid local/generated refs such as `file:`, `file://`, `fixtures/`, `target/`,
   `./target`, `../target`, absolute `.../target/...`, `/tmp`, `/var/tmp`, or
   `/dev/shm`, percent-encoded local refs including repeated encoding and
   encoded path separators, plus loopback refs such as `localhost`,
   `127.0.0.1`, `0.0.0.0`, `0`, `[::1]`, expanded IPv6 loopback,
   IPv4-mapped IPv6 loopback, `[::]`, and legacy IPv4 loopback/unspecified
   aliases in decimal, hexadecimal, octal, or short dotted notation. Windows
   drive absolute refs such as `C:\...`, `D:/...`, and percent-encoded variants
   are also local evidence references. UNC and scheme-relative path refs such as
   `\\server\share`, `//server/share`, and encoded variants are also
   non-operator local/network path references. Local transport URI refs such as
   `unix:`, `npipe:`, `pipe:`, and encoded variants are also non-operator
   local runtime references. Shell/user-local expansion refs such as `~/...`,
   `~user/...`, `$HOME/...`, `${USERPROFILE}/...`, `$TMPDIR/...`,
   `%USERPROFILE%/...`, and `%TEMP%/...` are also non-operator local runtime
   references. Generic filesystem path refs such as
   `operator-evidence/report.pdf`, `./operator-evidence/report.pdf`,
   `../operator-evidence/report.pdf`, `/home/operator/evidence/report.pdf`,
   and encoded variants are also non-operator local references.
   Symlinks are evaluated by their resolved path, so links back into
   generated, fixture, or temporary local paths are still non-operator
   evidence.
   Component `kms_hsm_custody` and `production_safe` also remain false for
   schema-valid non-operator evidence.
2. Provide real external compliance certification/immutability evidence before
   claiming production-grade public receipt guarantees. The validator and make
   wiring now exist, but default runs still record
   `external_certification.valid=false`, `compliance_immutability=false`, and
   the aggregate `receipt-production-readiness-check` blocker
   `compliance_certification`. Schema-valid fixture evidence is synthetic
   validator coverage only and is not operator-origin evidence. Top-level
   compliance certification evidence is a closed v1 shape: fields outside the
   documented schema plus `production_origin_proof` are rejected. Nested
   `external_review`, `scope`, and `immutability_evidence` objects are also
   closed v1 shapes. Each `evidence_artifacts[]` item is also closed to `kind`,
   `uri`, and `sha256_hex`; extra artifact-level fields are rejected across
   KMS/HSM, compliance, and trust-anchor evidence. Artifact `sha256_hex`
   values must be exactly 64 lowercase hex characters; uppercase, surrounding
   whitespace, or otherwise non-canonical digests are rejected. Artifact `kind` values are also
   component-specific closed sets:
   `provider_key_policy`/`signer_deployment_config` for KMS/HSM,
   `redacted_external_report`/`immutability_attestation` for compliance, and
   `publication`/`publisher-key` for trust-anchor evidence. Required controls
   lists are also closed and duplicate-free across those evidence schemas, so
   extra control names and duplicate controls are rejected. The compliance
   `operator_responsibilities` list is also closed and duplicate-free to the
   three required responsibilities, so extra reviewer responsibility claims and
   duplicates are rejected. Compliance evidence string values that bind schema,
   framework, report, reviewer, scope, timing, and immutability references must
   not include surrounding whitespace. Artifact `uri`, `report_ref`,
   `immutability_evidence.retention_policy_ref`, and
   `immutability_evidence.tamper_evidence_ref` values plus trust-anchor
   `key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
   `publisher_public_key_ref`, `publication_ref`, and `signature_ref` must be
   non-local references; artifact `uri` values, compliance `report_ref`,
   `immutability_evidence.retention_policy_ref`,
   `immutability_evidence.tamper_evidence_ref`, plus the trust-anchor
   reference fields must also contain no raw whitespace; trust-anchor
   `key_attestor_key_id` and `publisher_key_id` must contain no whitespace;
   generated, temporary, loopback, local transport, shell-local, and filesystem
   refs are rejected. Use
   `receipt-production-evidence-preflight-check` to fail fast on missing or
   synthetic certification evidence before running parser-coverage inventory
   checks. Certification evidence `external_review.issued_at` must not be more
   than 300 seconds in the future at validation time.
   `external_review.issued_at` and `external_review.valid_until` must be
   timezone-aware ISO-8601 timestamps. The strict
   `receipt-production-ready-check` uses
   `receipt-production-evidence-production-preflight-check`, so production
   readiness also requires separately supplied key-attestor trust-anchor
   publication evidence and expected inputs:
   `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF`.
   Expected public-key hex inputs, including
   `RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX`,
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`, and
   `RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
   must themselves be 64 lowercase hex characters; uppercase or otherwise
   non-canonical expected inputs are rejected instead of normalized.
   The trust-anchor evidence must use schema
   `cortexdb.operator_evidence_origin_trust_anchor.v1`, type
   `key_attestor_publication`, `external_control_plane=true`, non-local
   `key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
   `publisher_public_key_ref`, `publication_ref`, and `signature_ref`,
   required controls, at least two distinct hashed artifacts with distinct URIs
   and digests, and key-attestor fields that match the expected inputs. Its
   `published_at` must not be more than 300 seconds in
   the future at validation time. Trust-anchor `published_at` and
   `valid_until` must be timezone-aware ISO-8601 timestamps. Strict preflight also
   requires a `production_origin_proof` object with schema
   `cortexdb.operator_evidence_origin_proof.v1`, external `proof_ref`,
   `issuer_ref`, `issuer_public_key_ref`, `issuer_key_attestation_ref`,
   `key_attestor_ref`, `key_attestor_public_key_ref`, `signed_statement_ref`,
   `signature_ref`, and `key_attestation_signature_ref` references,
   those proof reference fields must be non-local external references and must
   contain no raw whitespace,
   `issuer_key_id`, `issuer_public_key_hex`, `key_attestor_key_id`,
   `key_attestor_public_key_hex`, `signature_hex`,
   `key_attestation_signature_hex`, SHA-256 digests for the proof, embedded
   issuer key attestation, embedded signed statement, detached signature bytes,
   detached key-attestation signature bytes, and evidence body, `reviewed_by`,
   `issued_at`, `expires_at`, and `external_control_plane=true`. The proof's
   `expires_at` must be after `issued_at` and still in the future at validation
   time, and the proof's `issued_at` must not be more than 300 seconds in the
   future at validation time. Both proof timestamps must be timezone-aware
   ISO-8601. The proof's identity, reference, and reviewer
   string values must not include surrounding whitespace. The proof's
   `issuer_key_id`, `key_attestor_key_id`, and `reviewed_by` values must
   contain no whitespace.
   The proof's
   key-attestor fields must match the separately supplied expected trust-anchor
   inputs. The proof's issuer identity fields must be distinct from the
   corresponding key-attestor identity fields, so the issuer key cannot attest
   itself. The proof's `reviewed_by` value must be distinct from `issuer_ref`,
   `issuer_key_id`, `issuer_public_key_ref`, `key_attestor_ref`,
   `key_attestor_key_id`, and `key_attestor_public_key_ref`. The
   `evidence_sha256_hex` value must match the evidence JSON after removing
   `production_origin_proof` and serializing with sorted keys and compact
   separators. The proof must embed a
   `cortexdb.operator_evidence_origin_key_attestation.v1` key attestation whose
   canonical digest matches `issuer_key_attestation_sha256_hex` and whose
   issuer key, attestor key, timing, statement-signing-domain, and reference
   fields match the proof. `key_attestation_signature_hex` must verify that
   attestation with Ed25519 under `key_attestor_public_key_hex`, and
   `key_attestation_signature_sha256_hex` must match the raw signature bytes.
   The proof must also embed a
   `cortexdb.operator_evidence_origin_statement.v1` statement whose canonical
   digest matches `signed_statement_sha256_hex` and whose schema, evidence,
   issuer, public-key, timing, and reference fields match the proof.
   `signature_hex` must verify that statement with Ed25519 under
   `issuer_public_key_hex`, and `signature_sha256_hex` must match the raw
   signature bytes. The `proof_sha256_hex` value must match the proof object
   after removing `proof_sha256_hex`.
   The trust-anchor publication itself must also bind the separately supplied
   expected publisher inputs through `publisher_key_id`,
   `publisher_public_key_hex`, `publisher_ref`, and
   `publisher_public_key_ref`; must carry `signature_algorithm=ed25519`,
   `signature_ref`, `signature_hex`, and `signature_sha256_hex`; and
   `signature_hex` must verify against `publisher_public_key_hex` over
   `cortexdb.operator_evidence_origin_trust_anchor.sign.v1 || 0x00 ||
   canonical_json(trust_anchor_evidence_without_signature_hex_and_signature_sha256_hex)`.
   The trust-anchor `key_attestor_public_key_hex`, `publisher_public_key_hex`,
   `signature_hex`, and `signature_sha256_hex` values must be lowercase hex in
   the original evidence JSON; uppercase, surrounding whitespace, or otherwise
   non-canonical hex is rejected instead of normalized.
   The top-level trust-anchor evidence object is also a closed v1 shape: fields
   outside the documented schema are rejected instead of being treated as
   trust-registry publication claims.
   The publisher key id, public key, publisher ref, and publisher public-key
   ref must be distinct from the key-attestor identity fields; a self-published
   key-attestor anchor is not sufficient trust-registry evidence.
   KMS/HSM custody evidence must include `runtime_signing_probe` with matching
   `key_id`, `public_key_hex`, and whitespace-free `signer_ref`, canonical external signer
   request/response digests, `signature_hex`, `signature_sha256_hex`, and a
   signature that verifies over
   `cortexdb.accountability_receipt.sign.v1 || 0x00 ||
   canonical_header_hex bytes`. The top-level and runtime-probe
   `public_key_hex`, the probe `canonical_header_hex`, `request_sha256_hex`,
   `response_sha256_hex`, `signature_hex`, and `signature_sha256_hex` values
   must be lowercase hex in the original evidence JSON; uppercase, surrounding
   whitespace, or otherwise non-canonical hex is rejected instead of
   normalized. The runtime probe `signed_at` timestamp must be
   within 24 hours of validation time and no more than 300 seconds in the
   future, and it must be timezone-aware ISO-8601.
   `operator_attestation.issued_at` must not be more than 300 seconds
   in the future at validation time. `operator_attestation.issued_at` and
   `operator_attestation.valid_until` must be timezone-aware ISO-8601
   timestamps.
   The standalone compliance component gate also requires a valid
   `production_origin_proof`, separately supplied
   `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`, expected key-attestor
   inputs, and expected trust-anchor publisher inputs before setting
   `supported_certified_frameworks` or `compliance_immutability=true`.
   Generated local artifacts under `target/` are also not operator-origin
   evidence, and neither are scratch files under `/tmp`, `/var/tmp`, or
   `/dev/shm`. Nested artifact references inside the evidence JSON must also
   avoid local/generated refs such as `file:`, `file://`, `fixtures/`, `target/`,
   `./target`, `../target`, absolute `.../target/...`, `/tmp`, `/var/tmp`, or
   `/dev/shm`, percent-encoded local refs including repeated encoding and
   encoded path separators, plus loopback refs such as `localhost`,
   `127.0.0.1`, `0.0.0.0`, `0`, `[::1]`, expanded IPv6 loopback,
   IPv4-mapped IPv6 loopback, `[::]`, and legacy IPv4 loopback/unspecified
   aliases in decimal, hexadecimal, octal, or short dotted notation. Windows
   drive absolute refs such as `C:\...`, `D:/...`, and percent-encoded variants
   are also local evidence references. UNC and scheme-relative path refs such as
   `\\server\share`, `//server/share`, and encoded variants are also
   non-operator local/network path references. Local transport URI refs such as
   `unix:`, `npipe:`, `pipe:`, and encoded variants are also non-operator
   local runtime references. Shell/user-local expansion refs such as `~/...`,
   `~user/...`, `$HOME/...`, `${USERPROFILE}/...`, `$TMPDIR/...`,
   `%USERPROFILE%/...`, and `%TEMP%/...` are also non-operator local runtime
   references. Generic filesystem path refs such as
   `operator-evidence/report.pdf`, `./operator-evidence/report.pdf`,
   `../operator-evidence/report.pdf`, `/home/operator/evidence/report.pdf`,
   and encoded variants are also non-operator local references.
   Symlinks are evaluated by their resolved path, so links back into
   generated, fixture, or temporary local paths are still non-operator
   evidence.
   Component `supported_certified_frameworks` and
   `compliance_immutability` also remain unset for schema-valid non-operator
   evidence.
3. Do not claim automated multi-leader load balancing. Adaptive
   refresh-on-overload is implemented for saturated cached Raft leaders, but
   Raft remains single-writer and there is no multi-writer balancer.

## Do Not Repeat

- Do not reopen AR-2/AR-4/AR-5/AR-7/AR-8 unless a receipt contract gate fails.
- Do not describe configured local JSON receipt emission as compliance-grade
  transparency, KMS/HSM custody, or externally witnessed accountability.
- Do not sign response-only JSON; emitted receipts must stay tied to captured
  retrieval/verification evidence.
- Do not treat `accountability_receipt_hash` audit binding as receipt/audit
  re-anchor records, external transparency, or compliance immutability.
- Do not treat local `cortexdb.receipt_audit_reanchor.v1` records as external
  transparency anchoring, KMS/HSM custody, or compliance immutability.
- Do not treat external command receipt signing or
  `receipt-kms-hsm-custody-check` as real KMS/HSM custody until operator
  custody evidence exists and the report sets `kms_hsm_custody=true`.
- Do not treat `fixtures/compliance/certification_evidence.valid.json` as real
  compliance certification; it is synthetic validator coverage only.
- Do not treat pack-time access re-derivation as captured enforcement.
- Do not move the four-competitor AAB matrix into the critical path.
- Do not try to implement 31-bit session/feedback slots inside
  `agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence`; it needs a new
  persisted schema version plus migration or refuse-to-read guard.
