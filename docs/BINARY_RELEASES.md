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
  BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.1-local \
  BINARY_RELEASE_PLATFORM=linux-x86_64 \
  BINARY_RELEASE_VERSION=v0.2.0-beta.1
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

From a checkout, the same flow can be run with checksum verification. The
installer accepts local archives and release artifact URLs:

```bash
scripts/install.sh cortexdb-<version>-<platform>.tar.gz --prefix "$HOME/.local"
scripts/install.sh https://github.com/AubakirovArman/CortexDB/releases/download/<version>/cortexdb-<version>-<platform>.tar.gz --prefix "$HOME/.local"
```

`scripts/install.sh` verifies the external `.tar.gz.sha256` file, the
package-internal `SHA256SUMS`, and the executable bits for `bin/cortexdb` and
`bin/cortex-server` before it installs anything. On success, it prints the
post-install commands for PATH setup, version checks, database validation, and
server startup. The release gate runs:

```bash
make install-script-check
```

and writes:

```text
target/install-script/report.json
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

The release matrix is explicit:

```text
linux-x86_64
linux-aarch64
macos-arm64
macos-x86_64
```

Local `make binary-release-check` still validates the archive for the current
machine; the GitHub release workflow is responsible for producing the full
platform set.

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

For the beta evidence bundle, `make beta-release-check` runs the local binary
release gate with:

```text
BINARY_RELEASE_VERSION=v0.2.0-beta.1
BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.1-local
```

and packages the resulting archive plus checksum into the beta evidence
artifact list.

The broader release evidence manifest is generated with:

```bash
make release-artifact-manifest-check
```

See [`RELEASE_ARTIFACT_MANIFEST.md`](RELEASE_ARTIFACT_MANIFEST.md) for the
binary, SDK, OpenAPI, evidence, and git metadata that are bound into the
machine-readable manifest.

## Package Manager Feasibility

Package-manager publication is tracked separately from tarball releases. The
current decision is documented in
[`PACKAGE_MANAGER_FEASIBILITY.md`](PACKAGE_MANAGER_FEASIBILITY.md):

- Homebrew tap packaging is feasible after adding a checked formula template.
- Linux `.deb` and `.rpm` packaging is feasible after adding package metadata
  templates and install/upgrade smoke tests.
- Official package-manager publication is not claimed by the current beta
  artifacts.

## Limits

- No Windows binary artifact is produced yet.
- No package manager formula or `.deb`/`.rpm` package is generated yet.
- No in-place downgrade guarantee; use the restore workflow in
  [`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md).
