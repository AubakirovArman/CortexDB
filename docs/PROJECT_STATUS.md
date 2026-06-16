# CortexDB Project Status & Honesty Manifest (v0.2.0-beta.2)

CortexDB is a **single-node agent-native database beta**. The workspace version
is `0.2.0-beta.2`, and the roadmap snapshot has 96 closed epics. This document
states what is real today, what remains non-production, and which long-term
research slices are prototypes.

This is not a production HA database, managed cloud service, enterprise IAM
platform, or legal-grade verification product.

## Stable Beta Surface

- **Durable single-node storage:** WAL, commit sequences, MVCC MemTable,
  checkpoint, compaction, restore/recovery, validation, and repair gates.
- **AQL and ContextPack:** permission-rewritten AQL retrieval, budgeted
  ContextPack v1, citations, answerability signals, deterministic prompt and
  Markdown exports.
- **Deterministic verification:** `VERIFY FACT` with indexed evidence,
  numeric-conflict handling, temporal/freshness guards, and no LLM dependency
  in the engine.
- **Retrieval indexes:** compact lexical postings, fixed-point BM25, field
  weights, Unicode analyzer profiles, disk-resident vector rows, hybrid RRF,
  guarded ANN/HNSW with exact fallback, and performance gates.
- **Local API and tooling:** HTTP API, CLI, OpenAPI, Rust/Python/TypeScript SDK
  contracts, mdBook docs, Docker quickstart, and MCP adapter.
- **Operational local controls:** token auth, AgentView scope gates, tenant
  filesystem realms, quotas, audit log, metrics, route timeouts, backup/restore,
  and point-in-time restore to sequence.

## Research Prototypes

The F-block work is useful product research, but it is not a production claim:

- tiered storage v2 hot/cold payload cache is an opt-in prototype;
- agent transaction semantics are guarded research semantics;
- learned ranking and semantic compression are opt-in/external-worker paths;
- value-per-token ContextPack planning is opt-in;
- multi-agent memory consistency defines private/shared handoff semantics;
- formal invariants are bounded executable models, not a full external formal
  verification program.

## Frozen Or Not Production

These are intentionally not claimed as production-ready:

- **Distributed replication and consensus:** `F02` and `F03` remain frozen until
  single-node production evidence, stable formats, and real HA demand exist.
- **Managed cloud:** `F09` remains frozen until Level 3 maturity and explicit
  demand.
- **IAM / external identity:** no production IAM federation, SAML/OIDC
  lifecycle, or distributed policy service.
- **TLS / mTLS:** use a reverse proxy for HTTPS/TLS offload; CortexDB does not
  manage TLS lifecycle in-process.
- **Encrypted-at-rest:** local backups and storage are not a production
  encrypted-at-rest system.
- **Compliance security:** no certification claim, zero-trust multi-process
  isolation, or legal-grade verification guarantee.

## Evidence And Release Boundary

- Current workspace version: `0.2.0-beta.2` in `Cargo.toml`.
- Release notes: `docs/RELEASE_NOTES_v0.2.0-beta.2.md`.
- Ordered epic tracker: `docs/DATABASE_GRADE_EXECUTION_PLAN.md`.
- Security boundary: `docs/SECURITY_MODEL.md`.
- Public claim boundary: `docs/PUBLIC_CLAIMS_POLICY.md`.

Before a public release tag, run the release gates and record the evidence
bundle. Until that freeze exists, treat `v0.2.0-beta.2` as the current workspace
beta boundary, not a production release.
