# External Identity Admin Runbook

Status: future external identity operations runbook. CortexDB still has no live
OIDC/SAML login endpoint or JWT signature verifier.

## Purpose

This runbook defines the operator workflow that future external identity support
must follow without weakening the current static-token authentication model.

## Current Boundary

Current code validates local prerequisites only:

- OIDC is the first protocol target.
- Provider configuration must fail closed.
- JWKS URLs must use HTTPS.
- Allowed algorithms are limited to `RS256`, `ES256`, and `PS256`.
- Provider outage policy must not fail open.
- Static bearer-token deployments remain supported during migration.

## Provider Configuration Checklist

Before enabling a future provider-backed auth path, an operator must validate:

1. `issuer` exactly matches the provider token issuer.
2. `audience` exactly matches the CortexDB API audience.
3. `jwks_url` uses HTTPS and points to the provider key set.
4. `allowed_algorithms` contains only asymmetric production algorithms.
5. `jwks_cache_ttl_seconds` is positive and bounded.
6. `request_timeout_ms` is positive and bounded.
7. `fail_open` is false.
8. Group-to-role mapping is explicit and does not trust provider groups as
   CortexDB scopes.

Local validation coverage:

```bash
make oidc-auth-contract-check
make identity-policy-mapping-check
make auth-rotation-check
make external-identity-design-check
```

## Rotation Procedure

Key rotation must be staged as a fail-closed operation:

1. Add the provider next key to JWKS before tokens are signed with it.
2. Keep the current key valid until old tokens expire.
3. Reject unknown `kid` values.
4. Refresh the JWKS cache on a bounded interval.
5. If the provider is unavailable, deny new external identity tokens instead of
   widening access.
6. Preserve static-token access for configured fallback operators.

The local rotation fixture models this with `current_kid`, `next_kid`,
`unknown_kid_policy=deny`, and
`provider_outage_policy=fail_closed_for_new_tokens`.

## Static Token Migration

During migration, run static tokens and external identity side by side:

1. Keep `CORTEXDB_AUTH_POLICY_STORE_FILE` configured.
2. Add external identity mapping for one low-risk group first.
3. Compare derived role, tenant, scopes, and AgentView id against the existing
   static principal.
4. Use audit records to confirm the derived decision shape.
5. Disable the matching static principal only after the external identity path
   has passed local and staging checks.
6. Keep rollback instructions for re-enabling the static principal.

Static tokens must never be removed automatically by external identity setup.

## Audit Expectations

External identity audit records must include:

- derived `principal_id`;
- derived role;
- tenant;
- scopes;
- optional AgentView id;
- allow/deny outcome;
- failure class for denied decisions.

External identity audit records must not include:

- bearer tokens;
- raw JWTs;
- raw claim payloads;
- provider secrets;
- JWKS contents.

## Incident Response

For invalid issuer, invalid audience, expired token, unknown key, missing
mapping, or provider outage:

1. deny the request;
2. emit a typed failure class;
3. keep static-token auth behavior unchanged;
4. avoid logging secrets or raw claims;
5. require operator action before retrying with changed provider settings.

## Promotion Boundary

This runbook is not a production OIDC/SAML implementation. Promotion requires a
live JWT signature verifier, JWKS fetch/cache logic, revoked-key tests,
provider outage tests, and request-path audit integration.
