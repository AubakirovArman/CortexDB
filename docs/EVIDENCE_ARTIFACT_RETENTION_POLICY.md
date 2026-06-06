# Evidence Artifact Retention Policy

Status: Core Alpha release governance policy.

This policy defines which CortexDB release evidence artifacts are published,
which artifacts are attached to GitHub Releases, and which artifacts stay
local-only. The machine-readable source is
[`EVIDENCE_ARTIFACT_RETENTION_POLICY.json`](EVIDENCE_ARTIFACT_RETENTION_POLICY.json).

## Why This Exists

Release evidence is only useful if it is reproducible and safe to share. A
release should expose enough reports for another maintainer to verify the
claim, but it must not publish raw provider responses, prompts, secrets, local
temp files, or oversized debug logs.

## GitHub Release Assets

GitHub Release assets are the small set of files users or reviewers need
directly from the release page:

- binary archive and `.sha256` sidecar;
- dashboard archive when included in the release train;
- unified release evidence bundle and `.sha256` sidecar;
- release artifact manifest and manifest report;
- generated release notes draft.

These assets are also allowed to appear in the release artifact manifest.

## Release Manifest Artifacts

Some artifacts are referenced by the release artifact manifest but are not
attached as top-level GitHub Release assets. These include platform/install
reports, beta retrieval reports, and SDK example archives. They remain
machine-readable release evidence, but users should consume them through the
manifest instead of downloading each one from the release page.

## Release Evidence Bundle

The release evidence bundle is the auditable archive built by:

```bash
make release-evidence-bundle-check
```

It may include passed report JSON files and small deterministic HTML/archive
artifacts from:

- production evidence;
- SDK release evidence;
- ContextPack, verification, retrieval, and performance quality gates;
- security/public-claims gates;
- backup, restore, tenant recovery, crash/fault, and chaos restart gates;
- explicitly experimental replication lifecycle evidence;
- dashboard and retrieval HTML summaries when present.

Reports in the bundle must pass the existing bundle validator. The bundle is
evidence for a local release candidate; it is not a production SLA.

## Local-Only Artifacts

The following classes stay ignored/local-only unless a future policy promotes a
redacted derivative:

- raw stdout/stderr logs;
- temp directories and scratch files;
- `.env`, credentials, tokens, provider keys, request bodies, prompts, and raw
  model/provider responses;
- large benchmark working directories such as LongMemEval, EnterpriseRAG-Bench,
  and MultiHop-RAG runs.

Public benchmark claims should publish compact redacted reports, manifests,
checksums, and configuration summaries instead of raw provider artifacts.

## Validation

Run:

```bash
make evidence-artifact-retention-check
```

The check writes:

```text
target/evidence-artifact-retention/report.json
```

It verifies that:

- the policy schema is current;
- release and bundle paths are relative and non-duplicated;
- secret-like paths are not classified as publishable;
- every artifact in the release evidence bundle manifest is classified;
- every artifact in the release artifact manifest is classified.

## Non-Goals

This policy does not provide remote artifact storage, legal retention,
compliance custody, or long-term hosted evidence preservation. Those remain
future managed-cloud/compliance milestones.
