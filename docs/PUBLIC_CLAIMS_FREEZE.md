# Public Claims Freeze

Status: Production v1.0 local single-node public wording freeze.

This document is the release-facing checklist for what CortexDB may and may not
claim publicly after the 21-epic plan.

## Allowed Public Claim

CortexDB may be described as:

```text
an experimental agent-native context database with a repeatably checked local
single-node durable core, typed HTTP/CLI/SDK contracts, ContextPack, VERIFY
FACT, search/ANN guardrails with exact fallback, and documented backup/restore.
```

That claim is local single-node only. It is not a public SLA.

## Forbidden Public Claims

Do not claim that CortexDB has:

- production distributed consensus;
- managed cloud readiness;
- enterprise compliance;
- enterprise RBAC or dynamic identity integration;
- legal-grade verification;
- production HNSW without exact fallback;
- tamper-evident audit compliance;
- built-in encrypted or remote object-store backups;
- Windows support.

## Required Boundary Wording

Any public document that mentions those future areas must also include boundary
wording such as:

- `experimental`;
- `future`;
- `blocked`;
- `unsupported`;
- `out of scope`;
- `not production`;
- `without exact fallback`.

## Release Gate

Run:

```bash
make public-claims-check
```

The gate scans public markdown for risky claim phrases and writes:

```text
target/public-claims/report.json
```

The production release should not be cut if this report is missing or failed.
