# RBAC Policy Store Design

CortexDB Core Alpha intentionally ships a small HTTP authorization surface:
static `admin` and `data` token roles, optional file-backed token rotation, and
optional token-to-`AgentView` binding. This document defines the next policy
layer without changing the current contract.

## Current Boundary

Implemented today:

- `admin` tokens can access all authenticated routes.
- `data` tokens can access health and data routes, but not admin, metrics,
  dashboard, flush, compact, stats, or validation routes.
- `role:token:agent_id` binds a token to a persisted `AgentView`.
- AgentView-bound data routes enforce readable/writable scope checks.
- Token files are re-read per request and fail closed on invalid content.
- `CORTEXDB_AUTH_POLICY_STORE_FILE` reads canonical
  `cortexdb.auth_policy.v1` stores and migrates the explicit legacy
  `cortexdb.auth_policy.v0` token-list shape into v1 in memory.
- v1 policy-store principals may set `request_quota_per_minute` for a local
  per-principal fixed-window quota.
- v1 policy-store principals may set `capabilities` to restrict a valid role to
  specific API action classes such as `search`, `read`, `write`, `ingest`,
  `verify`, `metrics`, or `admin`. Invalid, duplicate, or empty capability
  lists fail closed.
- v1 policy-store principals may set `tenants` to restrict a valid principal
  to specific HTTP tenant realms. Invalid, duplicate, or empty tenant lists fail
  closed. Omitting `tenants` preserves the current all-tenant local behavior.
- Admin-only local endpoints can upsert principals, disable principals, and
  roll back the last local policy-store mutation.
- Admin-only local endpoints can list the redacted policy store without
  returning raw bearer tokens.
- Admin-only local endpoints can grant or revoke persisted `AgentView` scopes
  for existing agents. This keeps `AgentView` as the source of scope
  permissions instead of duplicating read/write scopes inside the policy store.
- Every successful admin policy-store mutation is mirrored through the
  tenant-local `DatabaseActor` into redacted durable `_system:auth_policy`
  cells. These cells store principal ID, role, AgentView binding, disabled
  state, quota, capabilities, tenant allowlist, and a token fingerprint, but
  never the raw bearer token.
- Local audit JSONL records include chain metadata and can be verified by CLI.

Not implemented in Core Alpha:

- user accounts, sessions, groups, organizations, or external identity provider
  mapping;
- database-backed raw credential authentication; the JSON policy store remains
  the credential source of truth, while policy cells are a durable redacted
  runtime/audit mirror;
- richer role/capability objects beyond the current action-class capability
  allowlist;
- expiry or revocation events beyond token-file replacement and disabled
  policy-store principals;
- compliance-grade ledger storage or vendor-managed SIEM delivery.

## Target Model

The beta policy store should be a durable, local, explicit policy table layered
over the current route roles:

```text
Principal
-> RoleBinding
-> AgentView binding
-> Tenant allowlist
-> Scope permissions
-> Route class permissions
-> Action capability restrictions
```

Suggested records:

```text
Principal {
  principal_id,
  display_name,
  disabled,
  created_seq,
}

CredentialBinding {
  credential_id,
  principal_id,
  credential_hash,
  expires_at,
  disabled,
}

RoleBinding {
  principal_id,
  role: admin | data | auditor | operator,
  tenant,
}

AgentViewBinding {
  principal_id,
  tenant,
  agent_id,
}
```

## Route Classes

Keep the Core Alpha route classes as the base:

- `public`: health only;
- `data`: cell, AQL, search, ContextPack, verify, memory, ingestion;
- `admin`: stats, validate, flush, compact, dashboard, backup, repair;
- `metrics`: metrics and ANN metrics.

Future roles should be additive and explicit. For example, `auditor` can read
audit summaries and validation reports but cannot write cells or compact.
The current local `capabilities` field is a narrower action-class restriction
layer on top of the existing `admin` and `data` roles, not a full replacement
for richer beta roles.

## AgentView-Backed Scope Management

AgentView remains the source of scope permissions for agent-native operations:

- readable scopes control AQL, search, ContextPack, and verify;
- writable scopes control put, remember, forget, and ingestion;
- allowed modes and memory types remain policy inputs for AQL binding.

The policy store should not duplicate scope lists. It should bind principals to
AgentViews and let the existing AgentView validator enforce scopes.

## Non-Goals Until Beta

- No external IdP integration.
- No OAuth/OIDC sessions.
- No distributed policy replication.
- No dynamic admin UI for policy mutation.
- No claims that tenant realms are zero-trust isolation boundaries.
- No external compliance certification.

## Migration Path

1. Keep existing `CORTEXDB_AUTH_TOKENS` and token-file formats.
2. Add a read-only policy-store preview command that prints effective roles.
3. Add durable policy records in a new system scope. Implemented as
   `_system:auth_policy` redacted policy cells written through `DatabaseActor`.
4. Add policy-store read path behind a feature flag. The current beta path reads
   the JSON policy store for credentials and exposes test-covered runtime
   mapping from the redacted policy cells.
5. Add write APIs only after audit review tooling and rollback behavior are
   stable. The local v1 admin mutation endpoints are implemented and always
   write the canonical v1 shape plus the durable redacted cell mirror.
6. Keep legacy read migrations fail-closed: unsupported schema versions must
   not authenticate.

## Required Tests

- static `data` token cannot access admin or metrics routes;
- file-backed token rotation still fails closed;
- policy-store disabled principal cannot authenticate;
- AgentView-bound principal cannot read or write forbidden scopes;
- capability-restricted principal can access allowed action classes but gets
  `403` for disallowed action classes;
- invalid or duplicate policy-store capabilities fail closed;
- admin policy-store mutations sync durable redacted policy cells;
- durable policy cells do not expose raw bearer tokens;
- auditor role can review audit summaries but cannot mutate data;
- policy-store corruption fails closed.
