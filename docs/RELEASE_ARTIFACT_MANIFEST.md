# Release Artifact Manifest

Status: beta release evidence gate.

`make release-artifact-manifest-check` writes:

```text
target/release-artifact-manifest/manifest.json
target/release-artifact-manifest/report.json
```

The manifest binds together the release evidence that should travel with a beta
release:

- git branch and commit;
- OpenAPI path, version, and SHA-256 hash;
- binary archive, external `.sha256`, and package-internal manifest metadata;
- release evidence bundle archive and external `.sha256` when the production
  manifest gate is used;
- explicit Rust, Python, and TypeScript SDK package versions;
- storage format version inventory from `docs/STORAGE_FORMATS.md`;
- SDK e2e, SDK artifact, and SDK registry-gate reports;
- ContextPack, verification, retrieval, binary platform, and install-script
  reports;
- optional HTML/dashboard or SDK example archives when they exist locally.

The validator fails closed when:

- a required report is missing;
- a required report does not have `status: passed`;
- the binary archive sidecar checksum does not match;
- the binary archive is missing `package_manifest.json`;
- the production manifest gate requires a release evidence bundle and it is
  missing or its sidecar checksum does not match;
- SDK package versions drift from the workspace version;
- storage format inventory cannot be read;
- `docs/openapi.yaml` does not expose an `info.version`.

This manifest is not a package manager index and does not prove public registry
publication. It is a local, machine-readable bill of release evidence.
