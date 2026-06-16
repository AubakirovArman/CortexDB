# CortexDB v0.2.0-beta.2 Release Notes

Status: current workspace beta boundary; R04 release-readiness evidence is
recorded in [`RELEASE_EVIDENCE_v0.2.0-beta.2.md`](RELEASE_EVIDENCE_v0.2.0-beta.2.md).
The historical `v0.2.0-beta.2` tag already existed before R04 and was not
force-moved.

`0.2.0-beta.2` is the current workspace version and local single-node
developer/API beta for CortexDB. It is not a production distributed database,
managed cloud service, enterprise compliance release, or legal-grade
verification product.

The F-block items in this release are research/prototype slices unless noted
otherwise: tiered storage v2, agent transaction semantics, learned ranking,
semantic compression, value-per-token planning, multi-agent consistency, and
formal-invariant modeling.

## What Is In Scope

- Durable local write path: WAL, MemTable MVCC, checkpoint, compact, and restart
  recovery.
- AQL retrieval over the local database.
- ContextPack v1 with citations, citation-aware token budgeting,
  deterministic ordering, stable prompt/Markdown exports, and quality evidence.
- Deterministic `VERIFY FACT` with supported, contradicted, mixed, and
  insufficient verdicts.
- HTTP API, CLI, and Rust/Python/TypeScript SDK beta contracts.
- Local auth, tenant validation, rate limiting, CORS allowlist, audit redaction,
  and AgentView scope gates.
- Guarded lexical, vector, hybrid, and ANN/HNSW retrieval with exact fallback.
- Local backup, restore, validation, repair, and beta operations runbook.

## Evidence Gates

The R04 evidence run used:

```bash
make beta-release-check
```

The evidence bundle was written to:

```text
target/beta-release/report.json
target/beta-release/evidence.tar.gz
```

The bundle includes:

- SDK e2e report;
- OpenAPI and SDK contract reports;
- ContextPack quality report;
- VERIFY FACT quality report;
- retrieval quality and beta multi-domain report;
- security beta and hardening reports;
- tenant recovery and backup-drill reports;
- local binary package validation report;
- RAG demo smoke evidence;
- public claims and beta delta reports.

## Binary Artifacts

Local beta packaging uses:

```bash
make binary-release-check \
  BINARY_RELEASE_VERSION=v0.2.0-beta.2 \
  BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.2-local
```

Expected local artifacts:

```text
target/release-artifacts/cortexdb-v0.2.0-beta.2-local.tar.gz
target/release-artifacts/cortexdb-v0.2.0-beta.2-local.tar.gz.sha256
```

GitHub release packaging may publish platform-specific names such as:

```text
cortexdb-linux-x86_64.tar.gz
cortexdb-linux-aarch64.tar.gz
cortexdb-macos-arm64.tar.gz
cortexdb-macos-x86_64.tar.gz
```

Those artifacts must pass package validation before upload.

## SDK Artifacts

SDK publication remains tag-gated. For beta evidence:

```bash
make sdk-release-contract-check
make sdk-release-artifacts-check
make sdk-e2e-release-check
```

See [`SDK_PUBLICATION_STATUS.md`](archive/SDK_PUBLICATION_STATUS.md). These gates prove
local dry-run/package/e2e readiness; they do not claim public registry
publication unless the release job publishes from the tag.

## Demo

The product demo gate is:

```bash
make rag-demo-smoke
```

Expected behavior:

- search returns relevant rows;
- AQL returns cells;
- ContextPack returns cited evidence;
- VERIFY FACT returns `mixed_evidence` for the budget conflict scenario;
- output remains deterministic.

## Non-Goals

This beta does not claim:

- production distributed consensus;
- managed cloud readiness;
- enterprise RBAC/compliance certification;
- legal advice or legal-grade verification;
- unrestricted production HNSW without exact fallback;
- built-in production LLM inference;
- external identity provider production rollout;
- availability or performance SLA.

## Release Checklist

1. Confirm `git status --short` only contains intended release changes.
2. Run `cargo fmt --check`.
3. Run `cargo test --workspace --all-features`.
4. Run `cargo clippy --workspace --all-targets -- -D warnings`.
5. Run `make beta-release-check`.
6. Inspect `target/beta-release/report.json`.
7. Attach `target/beta-release/evidence.tar.gz` to the GitHub release.
8. Tag only after the report status is `passed`.
