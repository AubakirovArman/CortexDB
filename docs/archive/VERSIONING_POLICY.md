# Versioning Policy

Status: unified public-surface policy for Core Alpha to Beta.

This document unifies the versioning rules for CortexDB public surfaces:

- HTTP API and OpenAPI;
- Python, TypeScript, and Rust SDKs;
- storage file formats;
- AQL grammar and diagnostics.

The machine-readable source is
[`VERSIONING_POLICY.json`](../VERSIONING_POLICY.json).

## Versioning Principle

All public surfaces can evolve independently, but a release is only valid when
every affected surface has a documented compatibility decision, a changelog
entry, and a passing compatibility gate.

Pre-1.0 releases may still contain breaking changes, but they must be explicit.
Breaking changes must not be hidden as routine refactors.

## Surface Matrix

| Surface | Current contract | Changelog | Gate |
| --- | --- | --- | --- |
| HTTP API | `/v1` plus `docs/openapi.yaml` | `docs/API_CHANGELOG.md` | `make openapi-contract-check` |
| SDKs | workspace/package version alignment | `docs/SDK_RELEASE.md` | `make sdk-e2e-release-check` |
| Storage formats | magic/version inventory | `docs/STORAGE_COMPATIBILITY_EVIDENCE.md` | `make storage-compat-check` |
| AQL | AQL v0.4 grammar plus `fixtures/aql/grammar_change_registry_v1.json` | `docs/AQL_CHANGELOG.md` | `make aql-compat-check` |

## Breaking Changes

HTTP API breaking changes include:

- removing or renaming a response field;
- changing a stable error code or status mapping;
- removing an endpoint;
- changing a required query parameter or body field.

SDK breaking changes include:

- removing a typed client method;
- changing a decoded response type in an incompatible way;
- dropping support for a documented error code.

Storage breaking changes include:

- making a previous release fixture unreadable;
- changing file magic or version semantics without migration evidence;
- removing a supported backup/restore or historical restore path.

AQL breaking changes include:

- reinterpreting existing v0.4 syntax;
- changing stable parse diagnostic kinds;
- changing stable bind error codes;
- widening permissions through query syntax.

Every AQL grammar or binder compatibility change also requires a
`fixtures/aql/grammar_change_registry_v1.json` entry with a changelog anchor,
SQL example, and test reference.

## Non-Breaking Changes

The following are normally non-breaking when existing behavior is preserved:

- additive JSON response fields;
- optional query parameters;
- new SDK helper methods;
- new storage sections that older readers can skip or reject with a documented
  forward-compatibility error;
- additive AQL syntax that does not reinterpret v0.4 grammar.

## Breaking-Change Process

Every breaking change must:

1. name every affected public surface in the PR or design note;
2. update the relevant changelog before release;
3. update migration or compatibility documentation;
4. add golden, snapshot, or fixture tests for old and new behavior;
5. run the compatibility gate for every affected surface;
6. appear in generated release notes.

If the change spans multiple surfaces, all affected compatibility gates must be
green in the same release candidate.

## Version Bumps

| Release phase | Breaking change | Additive feature | Bug/security fix |
| --- | --- | --- | --- |
| Core Alpha | allowed only when explicit | patch/prerelease | patch/prerelease |
| Beta | minor/prerelease bump plus migration notes | patch/prerelease | patch/prerelease |
| 1.0+ | major version only | minor | patch |

For SDK packages, Rust and TypeScript use the canonical workspace version.
Python must stay aligned to the same release but may use the PEP 440 prerelease
spelling required by PyPI; for example, workspace `0.2.0-beta.2` maps to
Python `0.2.0b2`. This is a registry spelling difference, not a separate
release train.

## Release Evidence

The local gate is:

```bash
make versioning-policy-check
```

It writes:

```text
target/versioning-policy/report.json
```

The gate validates that each public surface has a version source, changelog,
compatibility gate, and breaking-change examples.
