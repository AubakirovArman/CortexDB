# External Identity Design

Status: future design gate, not implemented.

## Goal

Define external identity provider integration without weakening existing static
token deployments.

## Protocol Target

The first implementation should choose one protocol, preferably OIDC, before
adding SAML or other provider-specific behavior.

## Issuer And Audience

Every token must validate issuer, audience, expiration, not-before time, and
signature. Incorrect issuer or audience must fail closed.

## JWKS And Rotation

JWKS retrieval needs cache TTLs, refresh behavior, key rotation, and outage
policy. A provider outage must not silently widen access.

## Role And Scope Mapping

Identity claims must map to explicit CortexDB roles, tenants, scopes, and
AgentViews. Missing mappings fail closed. Group names from the identity provider
must not be trusted as CortexDB scopes without a configured mapping.

## Fail-closed Behavior

Invalid tokens, expired tokens, unknown keys, disabled principals, missing
scope mappings, and provider errors must deny access with stable typed errors.

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
