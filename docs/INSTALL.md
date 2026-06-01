# CortexDB Install Guide

Status: Core Alpha single-node install guide.

This guide covers local Linux and macOS installs from release tarballs or from
source. CortexDB Core Alpha is still experimental; validate a staging copy
before pointing a new binary at important data.

## Binary Tarball

Download the platform archive from the GitHub release:

```text
cortexdb-<version>-linux-x86_64.tar.gz
cortexdb-<version>-linux-aarch64.tar.gz
cortexdb-<version>-macos-arm64.tar.gz
cortexdb-<version>-macos-x86_64.tar.gz
```

Verify the archive checksum:

```bash
sha256sum -c cortexdb-<version>-<platform>.tar.gz.sha256
```

Extract and verify internal package checksums:

```bash
tar -xzf cortexdb-<version>-<platform>.tar.gz
cd cortexdb-<version>-<platform>
sha256sum -c SHA256SUMS
```

Install the binaries:

```bash
install -m 0755 bin/cortexdb ~/.local/bin/cortexdb
install -m 0755 bin/cortex-server ~/.local/bin/cortex-server
```

Confirm the commands are available:

```bash
cortexdb --version
cortex-server --help
```

## Source Build

Use this when testing a local checkout:

```bash
cargo build --release -p cortex-cli --bin cortexdb
cargo build --release -p cortex-server --bin cortex-server
install -m 0755 target/release/cortexdb ~/.local/bin/cortexdb
install -m 0755 target/release/cortex-server ~/.local/bin/cortex-server
```

## First Database

Create a database directory and run a smoke check:

```bash
cortexdb put ./data 1 "hello"
cortexdb get ./data 1
cortexdb validate ./data
cortexdb stats ./data
```

Run the HTTP server:

```bash
CORTEXDB_AUTH_TOKEN=change-me cortex-server ./data 127.0.0.1:8181
curl -H 'authorization: Bearer change-me' http://127.0.0.1:8181/v1/health
curl -H 'authorization: Bearer change-me' http://127.0.0.1:8181/v1/validate
```

## Install Verification

For a local checkout, validate release packaging with:

```bash
make binary-release-check
make deployment-upgrade-check
```

`make binary-release-check` also runs the binary platform matrix smoke:

```text
clean install -> load fixture -> query -> backup/restore -> server health
```

For an installed binary, validate the target database with:

```bash
cortexdb validate ./data
cortexdb stats ./data
```

## Related Docs

- Binary package format: [`BINARY_RELEASES.md`](BINARY_RELEASES.md).
- Binary platform matrix: [`BINARY_PLATFORM_MATRIX.md`](BINARY_PLATFORM_MATRIX.md).
- Systemd service example: [`SYSTEMD.md`](SYSTEMD.md).
- Upgrade and rollback: [`UPGRADE_ROLLBACK.md`](UPGRADE_ROLLBACK.md).
- Storage migration policy: [`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md).
