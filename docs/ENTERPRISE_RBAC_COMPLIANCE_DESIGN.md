# Enterprise RBAC And Compliance Design

Status: future design gate, not implemented.

## Goal

Define the authorization and compliance layer required beyond current static
`admin` and `data` token roles.

## Policy Store

The policy store must be durable and versioned. It owns principals, roles,
capabilities, scope bindings, AgentView bindings, disabled principal state, and
policy revision metadata.

Policy changes must be applied through admin-only APIs and must generate audit
events before they become effective.

Current implementation slice:

- `CORTEXDB_AUTH_POLICY_STORE_FILE` loads a local JSON policy store with
  `schema_version = cortexdb.auth_policy.v1`.
- Entries contain `principal_id`, `token`, `role`, optional `agent_id`, and
  optional `disabled`, plus optional `request_quota_per_minute`.
- Disabled principals are ignored.
- Invalid policy-store files fail closed.
- `cortexdb auth-review` reports local policy-store/token-file state without
  printing bearer tokens.
- `cortexdb audit-export-siem` exports normalized local audit JSONL.
- `docs/COMPLIANCE_BOUNDARY_MAPPING.md` states that no external compliance
  framework is currently certified or supported as a product claim.
- This slice is a local principal policy store and evidence boundary, not the
  full admin mutation API or compliance control layer.

## Principal Lifecycle

The lifecycle must support:

1. create principal;
2. bind credentials or external identity;
3. assign roles and scopes;
4. rotate or revoke credentials;
5. disable principal;
6. audit all changes;
7. migrate policies across format versions.

## Quota Model

Quota accounting must support safe token fingerprints, principal id, tenant,
route class, and rolling windows. Distributed quotas remain out of scope until
distributed consensus is production-ready.

## Tamper-evident Audit

Audit records must carry a monotonic sequence and hash chain. Verification must
detect record deletion, mutation, and reordering. Export formats must preserve
enough metadata for external review.

## Compliance Boundary

Compliance support must be named explicitly. The current boundary mapping is in
[`COMPLIANCE_BOUNDARY_MAPPING.md`](COMPLIANCE_BOUNDARY_MAPPING.md). A release
may include controls that help compliance reviews, but it must not imply
certification without an external assessment and a framework-specific control
map.

Release notes and public docs must not imply certification without that
framework-specific map and external review status.

## Required Gates

1. `make rbac-policy-store-check`
2. `make quota-policy-check`
3. `make audit-chain-check`
4. `make compliance-boundary-check`
5. `make security-hardening-check`

## Acceptance

1. Policy changes are durable, auditable, and reversible.
2. Disabled principals fail closed.
3. Audit-chain verification detects tampering.
4. Public docs state the exact compliance boundary.

## Non-goals

1. External identity provider integration; that is a separate epic.
2. Distributed quota guarantees before distributed consensus.
3. Claiming enterprise compliance without a control map and review.
