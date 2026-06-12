# Security Policy

CortexDB is a Core Alpha project. The current security posture is suitable for
local experiments and tightly controlled early integrations, not untrusted
multi-user production deployment.

## Reporting

Report security issues privately to the repository owner.

Do not open public issues for:

- access-control or AgentView bypasses;
- tenant path traversal or realm isolation bugs;
- data exposure through HTTP, CLI, SDK, dashboard, logs, or errors;
- corrupt recovery behavior that can lose or resurrect data;
- unsafe persistence bugs in WAL, manifest, segment, or index files;
- secret leaks in source, examples, generated artifacts, or history.

## Current Security Boundary

CortexDB Core Alpha currently provides:

- optional HTTP Bearer token authentication with `CORTEXDB_AUTH_TOKEN`,
  `CORTEXDB_AUTH_TOKENS`, or `CORTEXDB_AUTH_TOKENS_FILE`;
- static `admin`/`data` route roles and optional AgentView binding per data
  token;
- file-backed token rotation that fails closed when the policy file is missing,
  empty, or invalid;
- tenant realm path validation after URL percent-decoding;
- request body size limits on the Axum server;
- bounded actor queues with explicit `database_busy` backpressure;
- optional fixed-window request rate limiting;
- CORS disabled by default with one exact trusted-origin allowlist when
  configured;
- optional route-level audit events through `tracing` and a synced local JSONL
  sink;
- local database lock files to prevent concurrent writers;
- permission-safe AQL binding and runtime AgentView masks;
- checksummed WAL, manifest, segment, bitmap, lexical, vector, and HNSW files;
- strict and best-effort recovery modes with safe WAL truncate offsets;
- validated local backup, restore, restore-drill, retention pruning, and
  offsite-staging commands;
- typed HTTP error codes that avoid exposing internal scope/brain names for
  policy denials.

See:

- [`docs/SECURITY_THREAT_MODEL.md`](docs/archive/SECURITY_THREAT_MODEL.md)
- [`docs/AUTH.md`](docs/AUTH.md)
- [`docs/TENANT_NAMING_RULES.md`](docs/archive/TENANT_NAMING_RULES.md)
- [`docs/SAFETY_INVARIANTS.md`](docs/archive/SAFETY_INVARIANTS.md)
- [`docs/CORE_INVARIANTS.md`](docs/CORE_INVARIANTS.md)

## Non-Goals In Core Alpha

CortexDB Core Alpha does not yet provide:

- TLS termination;
- user accounts, sessions, dynamic RBAC policy stores, or external identity
  providers;
- per-user or distributed quotas;
- wildcard, multi-origin, or per-token CORS policy management;
- tamper-evident audit logs or SIEM export;
- at-rest encryption;
- encrypted backups;
- secure secret rotation workflow beyond local token file replacement;
- production-grade multi-tenant isolation;
- hardened distributed consensus security;
- remote object-store backup upload or full disaster-recovery automation.

Put the HTTP server behind a trusted reverse proxy if exposing it outside a
local development network. Use network-level isolation for tenant realms and
multi-user deployments.

## Secret Handling

- Pass tokens through environment variables or deployment secret stores.
- Do not commit tokens, `.env` files, generated wheels with embedded secrets,
  screenshots containing credentials, or local demo data with real customer
  content.
- Rotate any token that appears in git history, CI logs, issue text, or chat
  transcripts.

## Supported Versions

Only the current `main` branch and tagged Core Alpha releases receive security
fixes. Older experimental commits are not supported.
