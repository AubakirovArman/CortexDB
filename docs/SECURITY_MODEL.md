# CortexDB Security Model

This document summarizes the **current security model** for Core Alpha and what is
*not* guaranteed yet.

## Scope

CortexDB ships today as a single-node durable store + HTTP API. The security model is
anchored around:

- durable local storage safety (WAL + segments + manifest),
- request boundary controls (auth + tenant isolation),
- AgentView authorization in AQL + execution paths,
- contract-stable, non-sensitive API errors.

## Invariants that hold today

1. **Integrity on write path**
   - WAL and persisted indexes are CRC-protected.
   - Corruption is rejected or recovered according to strict/best-effort mode.

2. **Auth and tenant boundary**
   - Optional bearer-token auth is enforced when configured.
   - Token roles are currently `admin` and `data`.
   - Token policies are role-scoped (`admin`/`data`) and can be rotated from a file.
   - Tenant ID is validated and used for filesystem realm isolation.
   - Tenant allowlists are enforced before realm open/create, so a forbidden
     tenant data route cannot create or mutate that tenant directory.
   - Both the Axum `DatabaseActor` path and the legacy synchronous test harness
     route non-default tenants to `root/realms/<tenant>/`.
   - `make tenant-recovery-check` verifies tenant payload boundaries before
     and after backup/restore using a real HTTP server.

3. **Authorization in query execution**
   - AQL policies and runtime `AgentAllowed` masks prevent scope privilege expansion.
   - ContextPack JSON includes a per-cell `access_decision` trail linking each
     selected `cell_id` to the AgentView readable-scope decision that allowed it
     into the pack. HTTP responses attach the authenticated `principal_id` and
     `auth_role` when bearer-token policy store auth is configured.

4. **Permission-safe read invariant**
   - The source of truth for read authorization is
     `cortex_engine::plan::PolicyRewrite`.
   - Logical read plans are rewritten before execution so every `Scan` node
     carries the `agent_allowed` permission predicate; structural tests cover
     AQL retrieve/explain, search/explain, ContextPack/trace, cell get, verify,
     graph, memory, feedback, and export surfaces.
   - Direct descriptor-backed server surfaces such as `/v1/cell`, feedback,
     and memory routes delegate read decisions to `PolicyRewrite` before payload
     materialization. Stored-cell authorization uses durable `CellDescriptor`
     scope, not spoofable payload headers.
   - `make check` runs `policy-rewrite-gate-check`, which rejects direct
     production `AgentView::can_read_scope` calls outside `PolicyRewrite` and
     verifies the read-surface registry and structural tests remain present.
   - `cargo test -p cortex-server agent_view_property --all-features` exercises
     the E09 property suite across HTTP read surfaces before and after flush:
     no unreadable-scope payload marker may appear in success or error bodies.

5. **Error hardening**
   - Public API errors use stable machine-readable codes.
   - Policy errors avoid internal names like brain/scope identifiers.

6. **Operational safety**
   - Database lock prevents concurrent local writers.
   - Validation and repair tools run before recovery-critical operations.

7. **Local crypto surfaces that hold today**
   - `backup-encrypted` writes `cortexdb.encrypted_backup.v2` archives using
     XChaCha20-Poly1305 with Argon2id-derived keys, random salt/nonce, and
     AEAD authentication over the archive header as AAD.
   - Legacy encrypted backup v1 archives are refused on restore instead of
     being silently decoded.
   - File-backed audit JSONL records written by the server use
     `cortexdb.audit.v2` with SHA-256 event hashes and an HMAC-SHA-256
     `event_mac`. `CORTEXDB_AUDIT_MAC_KEY_HEX` is required when
     `CORTEXDB_AUDIT_LOG_FILE` is set.
   - The audit CLI verifies keyed v2 chains with `--mac-key-file`; legacy v1
     hash-chain fixtures remain readable as local compatibility records.
   - When configured local JSON receipt emission returns an
     `accountability_receipt.v1`, file-backed audit v2 records commit the
     receipt hash in `accountability_receipt_hash`.
   - Receipt signing key custody is available for configured local
     `accountability_receipt.v1` JSON emission:
     `cortexdb receipt-key generate`, `export-public`, and `rotate` write
     `cortexdb.receipt_signing_key.v1`, `cortexdb.receipt_public_key.v1`, and
     `cortexdb.receipt_trust.v1` files; server startup can parse a receipt
     signing key from `CORTEXDB_RECEIPT_SIGNING_KEY_FILE` or
     `CORTEXDB_RECEIPT_SIGNING_KEY_HEX` without logging the seed.
   - Receipt key rotation can write and verify a dual-signed
     `cortexdb.receipt_audit_reanchor.v1` record binding the old/new receipt
     public keys, trust manifest hash, audit chain head, and audit sequence.
   - When `CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE` is set together with
     configured receipt signing, the server writes a local transparency log for
     receipt `pack_root` anchors. The log is append-only JSONL with chained
     `cortexdb.transparency.log.record.v1` entries and rejects
     same-`determinism_hash` equivocation. This is not a third-party witness,
     Byzantine transparency service, KMS/HSM custody, or compliance ledger.
   - `cortexdb.transparency.witness.record.v1` provides an external mirror
     witness: a separate Ed25519 witness key signs the verified local log head,
     sequence range, and first/last receipt identities. This is off-database
     mirror evidence, not public transparency service availability, CT-style
     gossip, KMS/HSM custody, or compliance ledgering.
   - `cortexdb.transparency.witness.quorum.v1` provides independent witness
     quorum evidence: multiple separately signed witness records must agree on
     the same verified local log head and sequence range, while duplicate
     witness ids, key ids, or public keys are rejected. This is independent
     witness quorum evidence, not public transparency service availability,
     CT-style gossip, KMS/HSM custody, or compliance ledgering.
   - `cortexdb.transparency.inclusion.proof.v1` provides a Merkle inclusion
     proof from a sequence-bound transparency record hash to the published
     transparency root for that log snapshot, rejecting record-hash and
     sibling-path tampering. This is an inclusion proof primitive for public
     audit clients, not public transparency service availability, CT-style
     gossip/consistency exchange, KMS/HSM custody, or compliance ledgering.
   - `cortexdb.transparency.consistency.v1` provides append-only consistency
     evidence across published transparency snapshots, rejecting divergent
     snapshot prefixes and truncated newer snapshots. This is a public-monitor
     consistency primitive, not public transparency service availability,
     network gossip fanout, independent monitor uptime, KMS/HSM custody, or
     compliance ledgering.
   - `cortexdb.transparency.availability.evidence.v1` provides public monitor
     availability evidence for a published transparency head: fresh HTTPS
     observations must come from independent monitor ids and URLs, return
     available 2xx status, satisfy freshness and monitor-uptime policy, and
     agree on the published log count, log head, and Merkle root. This proves
     CI-safe public service availability and independent monitor uptime
     evidence, not network gossip fanout, KMS/HSM custody, or compliance
     ledgering.
   - `cortexdb.transparency.gossip.evidence.v1` provides network gossip fanout
     evidence for a published transparency head: fresh delivered HTTPS
     monitor-to-monitor exchanges must satisfy a declared fanout and agree on
     the published log count, log head, and Merkle root. This proves CI-safe
     gossip fanout evidence, not continuous production SLO compliance,
     Byzantine monitor key custody, KMS/HSM custody, or compliance ledgering.
   - `cortexdb.transparency.slo.evidence.v1` provides continuous public
     transparency operations/SLO evidence: ordered operations windows must
     cover the declared period without gaps, meet the declared availability
     percentage, carry monitor quorum and gossip fanout summaries, report
     append-only consistency status, and keep log counts monotonic. This proves
     CI-safe operations/SLO evidence, not live production deployment,
     Byzantine monitor key custody, KMS/HSM custody, or compliance ledgering.
   - Signed `accountability_receipt.v1` headers include `audit_chain_head`,
     the audit-chain tail observed before receipt emission. The
     `receipt-replica-invariance-check` gate proves the signed header is
     byte-identical for the same committed inputs and changes when the audit
     head changes; it does not prove live multi-node failover behavior.
   - Configured local receipt emission uses a durable database-instance
     identity from `cortexdb.database_instance_identity.v1`, stored as
     `cortexdb.database_instance_identity.json` in the database root, so receipt
     header `db_instance_id` is stable across tenants for the same local
     database instance.

## Not Yet Production Security

This section is intentionally explicit for beta/release checks: these items are
not production security guarantees yet.

The following are not production guarantees in Core Alpha:

- production IAM federation, distributed policy service, and external identity
  lifecycle,
- TLS/MTLS lifecycle in-process (use reverse proxy for HTTPS/TLS offload),
- live encryption of the running database directory or WAL,
- KMS-backed envelope encryption and secret management integrations,
- externally witnessed transparency anchoring, KMS/HSM-backed receipt key
  custody, and compliance-grade receipt witness infrastructure,
- compliance-certified immutable audit export or vendor-managed SIEM delivery,
- production distributed consensus correctness guarantees;
  `consensus-release-lane-check` is release-lane CI evidence, not a live
  production HA claim,
- multi-tenant zero-trust isolation across untrusted processes.

`receipt-production-readiness-check` aggregates the current receipt,
transparency, key-management, receipt KMS/HSM custody, security-release, and
compliance-boundary evidence into a single report. It currently reports
`production_ready=false` because KMS/HSM-backed receipt key custody and
external compliance certification evidence are not supplied by default.
`receipt-production-ready-check` is the strict production claim gate: it uses
the same component reports plus
`receipt-production-evidence-production-preflight-check`, and fails while
`production_ready=false`, so it should be used before any production-grade
public receipt guarantee is made.
The aggregate readiness check treats schema-valid fixture files as
synthetic validator coverage only. Production readiness requires
operator-origin evidence for both KMS/HSM custody and external compliance
certification; synthetic fixtures cannot clear those blockers. The component
reports use the same production boolean boundary: schema-valid synthetic
evidence can be reported for validator coverage, but it must not set
`kms_hsm_custody=true`, `production_safe=true`,
`supported_certified_frameworks`, or `compliance_immutability=true`.
The aggregate gate also independently requires those operator-origin component
reports to carry `production_origin_proof_required=true` and
`production_origin_proof_valid=true`, and each KMS/HSM and compliance component
report must also carry an operator-origin `production_origin_trust_anchor`
validation. Component summary booleans alone are not enough to clear the strict
production-ready gate.
Generated local artifacts under `target/` are also not operator-origin
evidence; production evidence must come from an operator-managed evidence
location outside generated build/test output and temporary local directories
such as `/tmp`, `/var/tmp`, or `/dev/shm`. Symlinks are evaluated by their
resolved path, so a link from an external-looking directory back into `target/`,
`fixtures/`, or a temporary local directory is still non-operator evidence.
Evidence files must also not point their artifact references at local/generated
evidence locations such as
`file:`, `file://`, `fixtures/`, `target`, path variants like `./target` or
`../target`, percent-encoded local references including repeated encoding and
encoded path separators, `/tmp`, `/var/tmp`, `/dev/shm`, or
loopback/unspecified references such as `localhost`, `127.0.0.1`, `0.0.0.0`,
`0`, `[::1]`, expanded IPv6 loopback, IPv4-mapped IPv6 loopback, `[::]`, and
legacy IPv4 loopback/unspecified aliases in decimal, hexadecimal, octal, or
short dotted notation. Absolute Windows drive paths such as `C:\...`, `D:/...`,
and percent-encoded variants are local references, not operator-origin
evidence locations. UNC or scheme-relative filesystem-like refs such as
`\\server\share`, `//server/share`, and encoded variants are also
non-operator local/network path references. Local transport URI refs such as
`unix:`, `npipe:`, `pipe:`, and encoded variants are local runtime references,
not operator-origin evidence locations. Shell/user-local expansion refs such as
`~/...`, `~user/...`, `$HOME/...`, `${USERPROFILE}/...`, `$TMPDIR/...`,
`%USERPROFILE%/...`, and `%TEMP%/...` are local runtime references, not
operator-origin evidence locations. Generic filesystem path refs such as
`operator-evidence/report.pdf`, `./operator-evidence/report.pdf`,
`../operator-evidence/report.pdf`, `/home/operator/evidence/report.pdf`, and
encoded variants are also local references, not operator-origin evidence
locations.
`receipt-production-evidence-preflight-check` is a fail-fast operator helper
for the same boundary: it validates that the configured KMS/HSM and compliance
evidence inputs are present, schema-valid, and operator-origin before running
the heavier production readiness chain. Passing the preflight does not replace
`receipt-production-ready-check`. The strict production claim path uses
`receipt-production-evidence-production-preflight-check`, which additionally
requires `RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF` as separate
strict preflight trust-anchor inputs. It also requires
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_KEY_ID`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_REF`, and
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_REF` so
the trust-anchor publication cannot name an arbitrary in-band publisher. The
expected public-key hex inputs, including
`RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`, and
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_TRUST_ANCHOR_PUBLISHER_PUBLIC_KEY_HEX`,
must themselves be 64 lowercase hex characters; uppercase or otherwise
non-canonical expected inputs are rejected instead of normalized. The
trust-anchor evidence must use schema
`cortexdb.operator_evidence_origin_trust_anchor.v1`, type
`key_attestor_publication`, `external_control_plane=true`, non-local
`key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
`publisher_public_key_ref`, `publication_ref`, and `signature_ref`, at least
two distinct hashed evidence artifacts with distinct URIs and digests, and
controls for attestor identity review, public-key publication, publication
digest recording, and production-origin scope review. Its `valid_until` must be
after `published_at` and still in the future at validation time, and its
`published_at` must not be more than 300 seconds in the future at validation
time. Both timestamps must be timezone-aware ISO-8601; timezone-less strings
are rejected instead of being interpreted as UTC. Its `key_attestor_key_id` and
`publisher_key_id` must contain no
whitespace. Its `key_attestor_key_id`, `key_attestor_public_key_hex`,
`key_attestor_ref`, and `key_attestor_public_key_ref` must match the separately
configured strict preflight inputs. Its `publisher_key_id`,
`publisher_public_key_hex`, `publisher_ref`, and `publisher_public_key_ref`
must match the separately configured publisher inputs. The trust-anchor
publication must use
`signature_algorithm=ed25519`, carry `signature_ref`, `signature_hex`, and
`signature_sha256_hex`, and verify `signature_hex` against
`publisher_public_key_hex` over
`cortexdb.operator_evidence_origin_trust_anchor.sign.v1 || 0x00 ||
canonical_json(trust_anchor_evidence_without_signature_hex_and_signature_sha256_hex)`.
The trust-anchor `key_attestor_public_key_hex`, `publisher_public_key_hex`,
`signature_hex`, and `signature_sha256_hex` values are validated as lowercase
hex in their original JSON strings; uppercase, surrounding whitespace, or
otherwise non-canonical hex is rejected rather than normalized.
The publisher key id, public key, publisher ref, and publisher public-key ref
must be distinct from the key-attestor identity fields, so a self-published
key-attestor anchor cannot satisfy the strict trust-registry boundary.
The top-level trust-anchor evidence object is a closed v1 shape: fields outside
the documented schema are rejected instead of being treated as signed but
unvalidated trust-registry publication claims.
Both KMS/HSM and compliance evidence JSON files must carry a
`production_origin_proof` object with schema
`cortexdb.operator_evidence_origin_proof.v1`, external
`proof_ref`, `issuer_ref`, `issuer_public_key_ref`,
`issuer_key_attestation_ref`, `key_attestor_ref`,
`key_attestor_public_key_ref`, `key_attestation_signature_ref`,
`signed_statement_ref`, and `signature_ref` references, `issuer_key_id`,
`issuer_public_key_hex`, `key_attestor_key_id`, `key_attestor_public_key_hex`,
SHA-256 digests for the proof, embedded issuer key attestation, embedded signed
statement, detached signature bytes, and evidence body, `signature_hex`,
`key_attestation_signature_hex`, `issued_at`/`expires_at`, and
`external_control_plane=true`. Those proof reference fields must be non-local
external references and must contain no raw whitespace. The proof's `expires_at` must be after
`issued_at` and still in the future at validation time, and the proof's
`issued_at` must not be more than 300 seconds in the future. Both proof
timestamps must be timezone-aware ISO-8601; timezone-less strings are rejected
instead of being interpreted as UTC. The proof's
identity, reference, and reviewer string values must not include surrounding
whitespace; signed non-canonical strings are rejected instead of silently
normalizing them before proof validation. The proof's `issuer_key_id` and
`key_attestor_key_id` values must contain no whitespace, and `reviewed_by`
must contain no whitespace as a canonical reviewer identity. The proof's
`key_attestor_key_id`,
`key_attestor_public_key_hex`, `key_attestor_ref`, and
`key_attestor_public_key_ref` values must match the separately configured
strict preflight trust-anchor inputs. The proof's issuer identity fields
(`issuer_ref`, `issuer_key_id`, `issuer_public_key_ref`, and
`issuer_public_key_hex`) must be distinct from the corresponding key-attestor
identity fields, so the issuer key cannot attest itself. The proof's
`reviewed_by` value must be
distinct from `issuer_ref`, `issuer_key_id`, `issuer_public_key_ref`,
`key_attestor_ref`, `key_attestor_key_id`, and
`key_attestor_public_key_ref`, so the production-origin review cannot be a
clone of the issuer or key-attestor identity. The proof must bind the exact evidence
body with
`evidence_sha256_hex`, computed over the evidence JSON after removing the
`production_origin_proof` object and serializing with sorted keys and compact
JSON separators. It must embed a
`cortexdb.operator_evidence_origin_key_attestation.v1`
`issuer_key_attestation` whose canonical SHA-256 matches
`issuer_key_attestation_sha256_hex` and whose issuer key, attestor key, timing,
statement-signing-domain, and reference fields match the proof. The key
attestation is signed over
`cortexdb.operator_evidence_origin_key_attestation.sign.v1 || 0x00 ||
canonical_json` with Ed25519; `key_attestation_signature_hex` must verify
against `key_attestor_public_key_hex`, and
`key_attestation_signature_sha256_hex` must match the raw signature bytes.
`issuer_key_attestation_sha256_hex`, `key_attestation_signature_hex`, and
`key_attestation_signature_sha256_hex` are deliberately outside
`issuer_key_attestation` so the attestation signed bytes do not contain their
own detached digest or signature fields. The nested
`issuer_key_attestation` object is a closed v1 shape: additional fields are
rejected instead of being treated as signed but unvalidated operator claims. It
must also embed a
`cortexdb.operator_evidence_origin_statement.v1` `signed_statement` whose
canonical SHA-256 matches `signed_statement_sha256_hex` and whose issuer, public
key, timing, proof-reference, evidence-schema, and evidence-digest fields match
the enclosing proof and evidence body. The statement is signed over
`cortexdb.operator_evidence_origin_statement.sign.v1 || 0x00 || canonical_json`
with Ed25519; `signature_hex` must verify against `issuer_public_key_hex`, and
`signature_sha256_hex` must match the raw signature bytes. `signature_hex` and
`signature_sha256_hex` are deliberately outside `signed_statement`, and
`signed_statement_sha256_hex` is also outside the statement body, so the signed
bytes are not self-referential. The nested `signed_statement` object is also a
closed v1 shape: additional fields are rejected instead of being treated as
signed but unvalidated operator claims. `proof_sha256_hex` must match the
canonical proof object after removing `proof_sha256_hex`. The top-level
`production_origin_proof` object is also a closed v1 shape: additional fields
are rejected instead of being treated as signed but unvalidated proof claims. A local JSON file that is merely shaped like
operator evidence can remain useful for parser coverage, but it cannot clear the
strict production-ready gate without that content-bound, signature-verified
production-origin proof, statement, separately supplied key-attestor trust
anchor binding, and operator-origin trust-anchor publication evidence.
Across KMS/HSM custody, compliance certification, and trust-anchor evidence,
each `evidence_artifacts[]` item is also a closed v1 shape: only `kind`, `uri`,
and `sha256_hex` are accepted, so additional artifact-level claims cannot sit
beside the hashed evidence reference without validator coverage. Artifact
`sha256_hex` values must be exactly 64 lowercase hex characters; uppercase,
surrounding whitespace, or otherwise non-canonical digests are rejected.
Artifact `kind` values are
also component-specific closed sets: KMS/HSM custody accepts
`provider_key_policy` and `signer_deployment_config`; compliance certification
accepts `redacted_external_report` and `immutability_attestation`; trust-anchor
publication accepts `publication` and `publisher-key`.
Artifact `uri` values must also be non-local references with no raw whitespace;
generated paths, temporary paths, loopback endpoints, local transports,
shell-local expansions, and filesystem paths are rejected instead of being
treated as external evidence.
Trust-anchor top-level refs use the same non-local boundary for
`key_attestor_ref`, `key_attestor_public_key_ref`, `publisher_ref`,
`publisher_public_key_ref`, `publication_ref`, and `signature_ref`, and those
refs must contain no raw whitespace, so a signed trust-anchor publication
cannot validate local build paths or non-canonical URI-like strings as external
trust-registry evidence.
Trust-anchor `key_attestor_key_id` and `publisher_key_id` values must also
contain no whitespace.
Required controls lists are closed and duplicate-free across those same
evidence schemas: extra control names are rejected instead of being treated as
unvalidated operator, reviewer, or trust-registry claims.
Forbidden secret field names are checked recursively with case-insensitive
normalized matching, so snake_case, camelCase, kebab-case, and compact aliases
such as `privateKey`, `apiToken`, and `seedHex` are rejected even when they
appear inside optional production-origin metadata used only for parser
coverage.

`receipt-kms-hsm-custody-check` records the current receipt custody boundary.
The report is expected to show `external_signer_runtime_supported=true`,
`kms_hsm_custody=false`, and
`custody_mode=external_signer_runtime_no_kms_hsm_evidence` until an operator
provides `RECEIPT_KMS_HSM_CUSTODY_EVIDENCE` plus the expected key id, public
key, and signer reference. The external signer runtime path keeps
`signing_seed_hex` out of server memory, signs the canonical receipt header with
the `cortexdb.accountability_receipt.sign.v1` domain through
`CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND`, verifies the returned signature
against `key_id`/`public_key_hex`, and fails closed without local-seed fallback.
The runtime command path is not itself a KMS/HSM claim: operator KMS/HSM custody
evidence must use schema `cortexdb.receipt_kms_hsm_custody_evidence.v1` and
bind the provider key reference to the runtime signer identity. The top-level
custody evidence object is a closed v1 shape: fields outside the documented
schema are rejected instead of being treated as unvalidated custody claims. It
also rejects local/generated `provider_key_ref` and `signer_ref` values instead
of accepting a local path as KMS/HSM custody evidence, and those runtime
reference fields must contain no raw whitespace. It
must also include `runtime_signing_probe` with the external signer
request/response schemas, matching `key_id`, `public_key_hex`, and
whitespace-free `signer_ref`, a non-empty `canonical_header_hex` challenge, canonical
request/response SHA-256 digests, `signature_hex`, `signature_sha256_hex`, and
`signed_at`. The top-level and runtime-probe `public_key_hex`, the probe
`canonical_header_hex`, `request_sha256_hex`, `response_sha256_hex`,
`signature_hex`, and `signature_sha256_hex` values are validated as lowercase
hex in their original JSON strings; uppercase, surrounding whitespace, or
otherwise non-canonical hex is rejected rather than normalized. The nested `runtime_binding`, `runtime_signing_probe`, and
`operator_attestation` objects are also closed v1 shapes, so extra nested
fields cannot be treated as runtime, probe, or operator-control claims. The
probe signature must verify with `public_key_hex` over
`cortexdb.accountability_receipt.sign.v1 || 0x00 || canonical_header_hex bytes`.
The runtime probe `signed_at` timestamp must be within 24 hours of validation
time and no more than 300 seconds in the future, and it must be
timezone-aware ISO-8601, so a stale or timezone-ambiguous signer probe cannot
stand in for current KMS/HSM custody evidence.
The operator attestation `valid_until` must be after `issued_at` and still in
the future at validation time, and `operator_attestation.issued_at` must not be
more than 300 seconds in the future. Both operator attestation timestamps must
be timezone-aware ISO-8601.
The standalone component gate now requires the same `production_origin_proof`
content binding as the strict preflight before it can set
`kms_hsm_custody=true`: operators must supply
`RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX`,
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF`, and
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF` alongside the
trust-anchor publisher inputs and runtime key expectations. The supplied
trust-anchor publication must itself validate as operator-origin evidence.
Operator-shaped JSON with a valid runtime signing probe but no
production-origin proof or no operator-origin trust-anchor publication remains
parser coverage and keeps `kms_hsm_custody=false` / `production_safe=false`.
If the supplied
evidence is schema-valid but synthetic, generated, local, or fixture-backed,
the report keeps `kms_hsm_custody=false` and records an operator-origin blocker.

`compliance-boundary-check` records the compliance-certification boundary. The
default report keeps `supported_certified_frameworks=[]`,
`external_certification.valid=false`, and `compliance_immutability=false`.
Operators can supply `COMPLIANCE_CERTIFICATION_EVIDENCE` using schema
`cortexdb.compliance_certification_evidence.v1`, optionally constrained by
`COMPLIANCE_CERTIFICATION_EXPECTED_FRAMEWORK`. Valid evidence must bind an
external reviewer, framework, report reference, accountability receipt scope,
reviewed controls, operator responsibilities, an external immutable store,
append-only export, retention policy, tamper evidence, and hashed artifacts.
The top-level `operator_responsibilities` list is closed and duplicate-free:
extra entries are rejected, and the required responsibilities are operating the
external immutable evidence store, retaining the redacted report under the
evidence request process, and binding production receipt key custody evidence
separately.
Compliance `report_ref`, `immutability_evidence.retention_policy_ref`, and
`immutability_evidence.tamper_evidence_ref` must also be non-local references
with no raw whitespace; local/generated refs are rejected before they can be
treated as external review or immutable-store evidence.
The top-level compliance certification evidence object is a closed v1 shape:
fields outside the documented schema are rejected instead of being treated as
unvalidated certification or immutability claims.
The nested `external_review`, `scope`, and `immutability_evidence` objects are
also closed v1 shapes, so extra nested fields cannot be treated as reviewed
certification, scope, or immutability claims.
Compliance evidence string values that bind schema, framework, report,
reviewer, scope, timing, and immutability references must not include
surrounding whitespace; non-canonical strings are rejected instead of silently
normalized.
The external review `valid_until` must be after `issued_at` and still in the
future at validation time, and `external_review.issued_at` must not be more
than 300 seconds in the future. Both external review timestamps must be
timezone-aware ISO-8601.
The standalone component gate requires a valid `production_origin_proof`,
`RECEIPT_PRODUCTION_ORIGIN_TRUST_ANCHOR_EVIDENCE`, the same separately supplied
`RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_*` inputs, and the
trust-anchor publisher inputs before publishing a supported framework or
setting `compliance_immutability=true`. Operator-shaped certification JSON
without that proof or without an operator-origin trust-anchor publication
remains parser coverage and keeps the production booleans false.
The repository does not claim SOC 2, ISO 27001, HIPAA, GDPR, or legal-grade
certification unless real operator evidence is supplied and the report sets the
external certification and immutability fields to true. Schema-valid synthetic
certification evidence does not publish a supported framework and does not set
`compliance_immutability=true`.

## Operational posture

For non-local deployments, treat Core Alpha as **alpha quality** and:

- run behind trusted network controls,
- keep `admin` tokens strong and rotate them via token files,
- enable audit logging if route-level traceability is needed,
- gate exposed endpoints with reverse-proxy hardening and rate limiting,
- run `cortexdb validate`, `cortexdb backup`, and `cortexdb restore` in change control.
- run `make tenant-recovery-check` before releases that modify tenant routing,
  backup/restore, or server actor lifecycle.
- run `make security-check` before beta/release packaging; it writes
  `target/security/report.json` with focused auth, tenant, CORS, rate-limit,
  audit-redaction, AgentView, body-limit, and OpenAPI contract evidence.

## Tenant Recovery Evidence

`make tenant-recovery-check` starts a real `cortex-server`, writes the same
`cell_id` into the `default`, `tenant-alpha`, and `tenant-beta` realms, flushes
and validates each tenant, verifies invalid tenant IDs fail closed, backs up the
server root, restores it to a new root, restarts the server, and verifies the
tenant payloads remain isolated after restore.

The report is written to:

```text
target/tenant-recovery/report.json
```

This is still a Core Alpha local tenant boundary, not a zero-trust
multi-process isolation guarantee.

## Relation to threat model

See `docs/SECURITY_THREAT_MODEL.md` for detailed threat analysis and current
controls list.

The beta-facing baseline is maintained in
[`docs/archive/SECURITY_BETA_BASELINE.md`](archive/SECURITY_BETA_BASELINE.md).
That document is the historical beta baseline. Current security claims are
defined by this file plus the current gate reports; compliance-grade audit
ledgering, KMS custody, and externally witnessed accountability receipt
guarantees remain future work outside the configured local
`accountability_receipt.v1` JSON emission path until their dedicated gates pass.
