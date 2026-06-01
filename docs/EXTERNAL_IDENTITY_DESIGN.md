# External Identity Design

Status: future phase 2 local claim-mapping verifier started, no live OIDC or
SAML provider integration implemented.

## Goal

Define external identity provider integration without weakening existing static
token deployments.

## Protocol Target

The first implementation should choose one protocol, preferably OIDC, before
adding SAML or other provider-specific behavior.

## OIDC Contract Boundary

Core Alpha does not expose `/v1/oidc`, `/v1/saml`, `/v1/identity/callback`, or
`/v1/login` routes. External identity is a future authentication layer, not a
new data API.

The first provider implementation must be OIDC-only unless a later design
explicitly adds SAML. OIDC acceptance requires typed configuration for issuer,
audience, JWKS URL, allowed signature algorithms, token lifetime, clock skew,
request timeout, fail-open policy, and mapping policy.

## Issuer And Audience

Every token must validate issuer, audience, expiration, not-before time, and
signature. Incorrect issuer or audience must fail closed.

## JWKS And Rotation

JWKS retrieval needs cache TTLs, refresh behavior, key rotation, and outage
policy. A provider outage must not silently widen access.

The local `validate_oidc_provider_config` helper now validates provider
configuration before any future token acceptance path can trust it. It requires
an HTTPS JWKS URL, non-empty issuer and audience, a positive bounded cache TTL,
a positive bounded request timeout, asymmetric production algorithms
(`RS256`, `ES256`, or `PS256`), and `fail_open=false`.

## Role And Scope Mapping

Identity claims must map to explicit CortexDB roles, tenants, scopes, and
AgentViews. Missing mappings fail closed. Group names from the identity provider
must not be trusted as CortexDB scopes without a configured mapping.

The local `verify_oidc_claims` verifier currently accepts already-validated OIDC
claims and enforces issuer, audience, expiration, not-before, explicit group
mapping, role, tenant, scope, and AgentView constraints. It is intentionally not
a JWT signature verifier and does not fetch JWKS.

The local verifier also validates the mapping configuration before issuing a
decision. Empty issuer or audience values, empty mapping lists, duplicate
provider groups, empty scopes, invalid roles, and invalid AgentView ids fail
closed.

The local `ExternalIdentityAuditRecord` contract records the outcome of that
mapping decision without tokens or raw claim payloads. Successful decisions
record the derived principal id, role, tenant, scopes, and optional AgentView
id. Failed decisions record only the failure class and keep identity payload
fields empty.

## Mapping Fixture

The local policy-mapping fixture models the future mapping contract:

- provider group strings are inputs, not CortexDB scopes;
- each allowed group maps to explicit role, tenant, scopes, and AgentView id;
- missing mappings deny access;
- static bearer-token deployments remain supported.

## Fail-closed Behavior

Invalid tokens, expired tokens, unknown keys, disabled principals, missing
scope mappings, and provider errors must deny access with stable typed errors.

## Rotation Fixture

The rotation fixture models expected JWKS behavior without a live provider:

- `unknown_kid` is denied;
- invalid issuer, invalid audience, expired token, and missing mapping are
  denied;
- provider outage fails closed for new tokens;
- audit metadata may identify the principal but must not log raw bearer tokens.

Operator-facing rotation and provider configuration steps are tracked in
[`EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md`](EXTERNAL_IDENTITY_ADMIN_RUNBOOK.md).

## Migration From Static Tokens

Static-token deployments must continue to work while external identity is
introduced. Operators should be able to run both modes during migration and
revoke either mode independently. The current local fixtures keep
`static_tokens_supported=true` to make that compatibility rule explicit.

## Current Evidence Boundary

The current gates prove local prerequisites only:

| Gate | Evidence |
| --- | --- |
| `make oidc-auth-contract-check` | OpenAPI/server routes do not expose external identity login/callback endpoints yet, and the design keeps OIDC as the first protocol target. |
| `make identity-policy-mapping-check` | A fixture and Rust verifier validate explicit group-to-role/tenant/scope/AgentView mapping and reject direct group-as-scope trust. |
| `make auth-rotation-check` | A fixture validates JWKS rotation and provider-outage fail-closed policy. |
| `make security-hardening-check` | Existing auth, AgentView, audit, quota, and local policy-store evidence remains green. |

Reports are written under `target/external-identity/` and keep
`external_identity_ready=false`. They prove local claim-mapping behavior only
and do not claim live OIDC, SAML, session, JWT signature verification, JWKS
fetching, or external provider integration.

## Required Gates

1. `make external-identity-design-check`
2. `make oidc-auth-contract-check`
3. `make identity-policy-mapping-check`
4. `make auth-rotation-check`
5. `make security-hardening-check`

## Acceptance

1. A configured provider maps users to exact roles, tenants, scopes, and
   AgentViews.
2. Static-token deployments remain supported.
3. Key rotation and provider outage behavior is tested.
4. Audit records identify the authenticated principal safely.

## Non-goals

1. Implementing every identity protocol at once.
2. Trusting identity-provider group strings as scopes directly.
3. Managed-cloud tenant lifecycle; that is a separate epic.
