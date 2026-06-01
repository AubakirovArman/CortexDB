# Future Non-goal Epics

Reviewed: 2026-06-01

This document turns the current public non-goals into explicit future epics.
These epics are not part of the closed local single-node evidence boundary.
They must not be marketed as implemented until their gates pass with current
evidence.

## Status Summary

Total future epics: 7.

| # | Epic | Status | Design gate | Promotion boundary |
|---|---|---|---|---|
| 1 | Production Distributed Consensus | future-design-ready | `make distributed-consensus-design-check` | Multi-node replicated log, leader failover, split-brain prevention, and sustained rejoin evidence |
| 2 | Managed Cloud | future-design-ready | `make managed-cloud-design-check` | Hosted control plane, tenant isolation, billing/quotas, cloud operations, and support lifecycle |
| 3 | Enterprise RBAC And Compliance | future-phase-1-started | `make enterprise-rbac-design-check` | Durable policy store, auditable permissions, compliance controls, and admin lifecycle |
| 4 | Full Production HNSW Without Fallback | future-design-ready | `make hnsw-no-fallback-design-check` | ANN can serve critical workloads without exact fallback while meeting recall and latency SLOs |
| 5 | Built-in LLM Inference | future-design-ready | `make llm-inference-design-check` | Model runtime, resource isolation, prompt safety, provider compatibility, and operational cost controls |
| 6 | External Identity Providers | future-design-ready | `make external-identity-design-check` | OIDC/SAML or equivalent identity integration with role/scope mapping and rotation |
| 7 | Legal-grade Verification | future-design-ready | `make legal-verification-design-check` | Legal-domain evidence model, citations, review workflow, liability boundaries, and evaluation by domain experts |

## Promotion Rules

1. A future epic becomes active only after the current release owner explicitly
   moves it into the active execution queue.
2. Every future epic needs a design document before implementation.
3. Every future epic needs a machine-readable report under `target/` before any
   public readiness claim.
4. Public docs must continue to say these are not implemented until the relevant
   gate passes.
5. No future epic can weaken the current local single-node guarantees.

## Phase 0 Design Gates

The current repository now tracks design-level gates for all seven future
epics. These gates prove that each epic has a scoped design, promotion boundary,
and public-claim guard. They do not prove implementation readiness.

Run all design gates:

```bash
make future-epic-design-check
```

Run individual design gates:

```bash
make distributed-consensus-design-check
make managed-cloud-design-check
make enterprise-rbac-design-check
make hnsw-no-fallback-design-check
make llm-inference-design-check
make external-identity-design-check
make legal-verification-design-check
```

## Epic 1 - Production Distributed Consensus

Goal: turn experimental/local replication primitives into a production-grade
multi-node consensus system.

Why this is future:

- Current production boundary is local single-node.
- Current replication and consensus evidence is useful hardening evidence, not
  a production distributed database claim.

Task pool:

1. Write `docs/DISTRIBUTED_CONSENSUS_DESIGN.md` with the failure model,
   consistency model, quorum model, and non-goals.
2. Split local WAL semantics from the replicated consensus log.
3. Define durable term, vote, commit index, applied index, and snapshot metadata.
4. Implement leader election with persisted votes and term safety.
5. Implement log replication with conflict resolution and commit advancement.
6. Implement snapshot install and recovery after restart.
7. Implement membership changes with joint-consensus or another documented safe
   transition protocol.
8. Add split-brain prevention tests under partition and clock skew simulation.
9. Add rejoin, follower lag repair, and snapshot catch-up soak tests.
10. Add failover SLOs and operational runbooks.

Required gates:

1. `make distributed-consensus-check`
2. `make consensus-partition-soak-check`
3. `make consensus-failover-slo-check`
4. `make consensus-rejoin-check`
5. `make public-claims-check`

Acceptance:

- A three-node cluster survives leader loss, network partition, restart, and
  rejoin without committed-log divergence.
- A node cannot serve stale committed data as current state.
- All failover and repair SLOs are documented and measured.
- Public docs clearly separate supported distributed behavior from remaining
  limitations.

## Epic 2 - Managed Cloud

Goal: define and implement a hosted CortexDB product surface.

Why this is future:

- Current product is a local single-node database.
- Managed cloud needs operational, support, security, billing, and isolation
  systems that do not exist yet.

Task pool:

1. Write `docs/MANAGED_CLOUD_DESIGN.md`.
2. Define tenant provisioning, deletion, backup, restore, and suspension
   lifecycle.
3. Define control-plane and data-plane separation.
4. Define cloud tenant isolation boundaries and noisy-neighbor policy.
5. Define billing, quotas, and usage metering.
6. Define managed upgrade and rollback procedures.
7. Define support access, audit, and break-glass workflows.
8. Add cloud deployment IaC skeleton only after the design is approved.
9. Add cloud smoke tests in an isolated staging environment.
10. Add incident response and status-page process.

Required gates:

1. `make managed-cloud-design-check`
2. `make cloud-tenant-lifecycle-check`
3. `make cloud-backup-restore-check`
4. `make cloud-upgrade-check`
5. `make public-claims-check`

Acceptance:

- A staging managed instance can provision a tenant, ingest data, retrieve
  context, survive backup/restore, and be deleted without data leakage.
- Billing and quota events are observable and auditable.
- Operational ownership and support boundaries are documented.

## Epic 3 - Enterprise RBAC And Compliance

Goal: move from static roles and token files to enterprise-grade authorization
and compliance controls.

Why this is future:

- Current security model supports static `admin` and `data` roles, token-file
  rotation, optional AgentView binding, audit JSONL, CORS, and rate limits.
- It does not claim enterprise RBAC, compliance certification, or full audit
  integrity.

Current implementation slice:

- `CORTEXDB_AUTH_POLICY_STORE_FILE` supports a local JSON principal policy
  store.
- Active principals can authenticate with `admin` or `data` roles.
- Disabled principals fail closed.
- Optional `agent_id` binds a policy-store principal to a persisted AgentView.
- Invalid policy-store JSON or invalid entries fail closed.
- HTTP audit events now include `principal_id`, `auth_role`, and
  `auth_agent_id` for authenticated requests without logging bearer tokens.
- File-backed audit records now include local chain metadata and can be checked
  with `cortexdb audit --verify-chain`.
- Policy-store principals can set local fixed-window
  `request_quota_per_minute`; one principal exhausting quota does not block
  another principal.

Task pool:

1. Promote `docs/RBAC_POLICY_STORE_DESIGN.md` into an implementation spec.
2. Implement a durable local policy store for principals, roles, scopes, and
   capabilities.
3. Add policy mutation APIs with admin-only access and full audit events.
4. Add disabled-principal and token revocation lifecycle.
5. Add per-token and per-principal quota accounting.
6. Add tamper-evident audit chain with sequence continuity checks.
7. Add audit export for SIEM-oriented pipelines.
8. Add compliance-control mapping docs for the intended target framework.
9. Add dashboard admin views for policy review and audit review.
10. Add migration and rollback for policy-store format changes.

Required gates:

1. `make rbac-policy-store-check`
2. `make quota-policy-check`
3. `make audit-chain-check`
4. `make compliance-boundary-check`
5. `make security-hardening-check`

Acceptance:

- Role/scope/capability changes are durable, auditable, and reversible.
- A disabled or revoked principal cannot access data routes.
- Audit-chain verification detects deletion, reordering, or mutation of audit
  records.
- Public docs state exactly which compliance frameworks are supported, if any.

## Epic 4 - Full Production HNSW Without Fallback

Goal: allow ANN/HNSW to serve selected production workloads without requiring
exact fallback.

Why this is future:

- Current ANN is guarded and evidence-backed, but exact fallback remains part of
  the safety policy for critical workloads.
- Removing fallback changes the failure mode from slower exact retrieval to
  potentially incorrect retrieval.

Task pool:

1. Define allowed workloads where fallback-free ANN can be used.
2. Add corpus-level index build metadata and `.ach` schema upgrade policy.
3. Add recall fixtures for real-domain and public corpora.
4. Add multi-run recall and latency history with machine-profile metadata.
5. Add graph freshness and stale-graph blocking policy.
6. Add online rebuild and rollback behavior.
7. Add degraded-index detection and serving guardrails.
8. Add operational metrics for graph health, rebuild count, stale graph count,
   recall probes, p95/p99 latency, and fallback-disabled requests.
9. Add a rollout flag that defaults to guarded mode.
10. Add incident playbook for recall regression.

Required gates:

1. `make ann-production-no-fallback-check`
2. `make ann-real-domain-history-check`
3. `make ann-public-corpus-history-check`
4. `make ann-graph-freshness-check`
5. `make performance-trend-check`

Acceptance:

- The selected no-fallback profile meets documented recall and latency SLOs
  across repeated runs and corpus changes.
- The system blocks serving degraded ANN results when the graph is stale,
  corrupt, or below recall threshold.
- Public docs do not generalize no-fallback safety beyond the proven profiles.

## Epic 5 - Built-in LLM Inference

Goal: decide whether CortexDB should host model inference directly and, if so,
implement it safely.

Why this is future:

- Current CortexDB is a context database and ContextPack producer for external
  agents and model runtimes.
- Built-in inference adds scheduling, GPU/CPU resource isolation, model
  security, latency, and cost-control responsibilities.

Task pool:

1. Write `docs/LLM_INFERENCE_DESIGN.md` with build-vs-integrate decision.
2. Define model provider interfaces and supported local/remote runtimes.
3. Define prompt assembly from ContextPack without hidden data expansion.
4. Define model resource limits, concurrency, and cancellation.
5. Define safety and audit logging for model calls.
6. Define cost and quota controls.
7. Add an inference endpoint only after API contract approval.
8. Add model-runtime health and metrics.
9. Add deterministic test doubles for CI.
10. Add end-to-end examples that do not require committed secrets.

Required gates:

1. `make llm-inference-design-check`
2. `make llm-inference-contract-check`
3. `make llm-inference-safety-check`
4. `make llm-inference-smoke-check`
5. `make secrets-check`

Acceptance:

- Inference can be enabled or disabled explicitly.
- ContextPack remains the source of retrieved context; inference cannot bypass
  AgentView or tenant boundaries.
- CI never requires real provider keys.

## Epic 6 - External Identity Providers

Goal: integrate CortexDB auth with external identity providers.

Why this is future:

- Current auth supports bearer tokens, token files, static roles, and optional
  AgentView binding.
- External identity adds lifecycle, group mapping, session validation, key
  rotation, and incident response requirements.

Task pool:

1. Write `docs/EXTERNAL_IDENTITY_DESIGN.md`.
2. Choose first protocol target, such as OIDC, before adding others.
3. Define issuer, audience, JWKS, token lifetime, and key rotation behavior.
4. Define identity-to-role and identity-to-AgentView mapping.
5. Define group and tenant mapping rules.
6. Add fail-closed behavior for identity provider outages.
7. Add admin docs for provider configuration and rotation.
8. Add security tests for invalid issuer, invalid audience, expired tokens,
   revoked keys, and missing scope mapping.
9. Add audit events for authenticated identity and policy decisions.
10. Add migration path from static tokens.

Required gates:

1. `make external-identity-design-check`
2. `make oidc-auth-contract-check`
3. `make identity-policy-mapping-check`
4. `make auth-rotation-check`
5. `make security-hardening-check`

Acceptance:

- A configured provider can authenticate users and map them to exact roles,
  tenants, scopes, and AgentViews.
- Incorrect or stale identity data fails closed.
- Static-token deployments continue to work without external identity.

## Epic 7 - Legal-grade Verification

Goal: determine and, if approved, build a legally defensible verification
workflow for selected domains.

Why this is future:

- Current `VERIFY FACT` is deterministic retrieval and evidence analysis, not a
  legal opinion or compliance certification.
- Legal-grade behavior requires domain expertise, provenance rules, review
  workflows, and liability boundaries.

Task pool:

1. Write `docs/LEGAL_VERIFICATION_BOUNDARY.md`.
2. Define the first supported legal domain and jurisdiction, if any.
3. Define admissible source types and citation requirements.
4. Define provenance and chain-of-custody metadata.
5. Define reviewer workflow and human approval requirements.
6. Define contradiction, uncertainty, and insufficiency policy.
7. Add legal-domain labeled evaluation datasets with expert review.
8. Add legal-specific output schema that avoids unsupported legal advice.
9. Add audit and retention policy for verification reports.
10. Add public disclaimers and user-facing limitations.

Required gates:

1. `make legal-verification-design-check`
2. `make legal-verification-dataset-check`
3. `make legal-verification-quality-check`
4. `make legal-citation-policy-check`
5. `make public-claims-check`

Acceptance:

- The system produces reviewed, citation-complete verification reports for the
  selected legal domain and clearly refuses unsupported claims.
- Legal outputs are traceable to source records and reviewer decisions.
- Public docs do not imply legal advice outside the proven scope.

## Suggested Execution Order

1. Enterprise RBAC And Compliance
2. External Identity Providers
3. Full Production HNSW Without Fallback
4. Production Distributed Consensus
5. Managed Cloud
6. Built-in LLM Inference
7. Legal-grade Verification

Rationale:

- RBAC and external identity are prerequisites for managed cloud and enterprise
  deployments.
- Fallback-free ANN should be proven before it is offered as a default product
  behavior.
- Distributed consensus and managed cloud should not start until the
  single-node product surface, security model, and operational evidence are
  stable.
- Built-in inference and legal-grade verification add product liability and
  operational complexity, so they should remain separate opt-in programs.
