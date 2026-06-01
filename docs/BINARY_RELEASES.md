# Binary Releases

CortexDB binary release artifacts are local Core Alpha packages for:

- `cortexdb`
- `cortex-server`

They are not a managed deployment system. They give operators a repeatable
tarball with checksums and install notes.

## Local Packaging

Build and validate a package for the current machine:

```bash
make binary-release-check
```

The default archive path is:

```text
target/release-artifacts/cortexdb-local.tar.gz
target/release-artifacts/cortexdb-local.tar.gz.sha256
```

Override the release metadata:

```bash
make binary-release-check \
  BINARY_RELEASE_ID=cortexdb-v0.1.0-core-alpha-linux-x86_64 \
  BINARY_RELEASE_PLATFORM=linux-x86_64 \
  BINARY_RELEASE_VERSION=v0.1.0-core-alpha
```

## Archive Contents

Each archive contains:

```text
<package-id>/
  bin/cortexdb
  bin/cortex-server
  install/INSTALL.md
  package_manifest.json
  SHA256SUMS
```

`package_manifest.json` records file sizes and SHA-256 checksums. `SHA256SUMS`
can be checked with:

```bash
tar -xzf cortexdb-v0.1.0-core-alpha-linux-x86_64.tar.gz
cd cortexdb-v0.1.0-core-alpha-linux-x86_64
sha256sum -c SHA256SUMS
```

## Install

```bash
install -m 0755 bin/cortexdb ~/.local/bin/cortexdb
install -m 0755 bin/cortex-server ~/.local/bin/cortex-server
```

Before replacing a binary that touches existing data:

```bash
cortexdb backup-drill ./db ./backups/cortexdb-pre-upgrade ./drills/cortexdb-pre-upgrade
cortexdb validate ./db
```

Then install the new binaries and run:

```bash
cortexdb validate ./db
cortexdb stats ./db
```

## GitHub Release Workflow

`.github/workflows/release.yml` packages Linux and macOS artifacts for tag
pushes. The workflow validates each tarball before uploading it to the GitHub
Release and also uploads workflow artifact copies.

## Binary Platform Matrix

The supported local single-node matrix is documented in
[`BINARY_PLATFORM_MATRIX.md`](BINARY_PLATFORM_MATRIX.md). Linux and macOS are
the release artifact targets. Windows is explicitly unsupported until native
path, service, packaging, and clean-install smoke gates exist.

`make binary-release-check` now validates:

```text
package -> archive validation -> clean install -> fixture load -> query
-> backup -> restore -> server health/query
```

## Limits

- No Windows binary artifact is produced yet.
- No installer, package manager formula, or service unit is generated.
- No in-place downgrade guarantee; use the restore workflow in
  [`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md).
