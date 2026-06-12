# Binary Platform Matrix

Status: Production v1.0 local single-node platform boundary.

## Supported Release Artifacts

| Platform | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `linux-x86_64` | `cortexdb-<version>-linux-x86_64.tar.gz` | supported | Primary release artifact. |
| `linux-aarch64` | `cortexdb-<version>-linux-aarch64.tar.gz` | supported when built by release workflow | Same tarball contract. |
| `macos-arm64` | `cortexdb-<version>-macos-arm64.tar.gz` | supported when built by release workflow | Includes CLI and server binaries. |
| `macos-x86_64` | `cortexdb-<version>-macos-x86_64.tar.gz` | supported when built by release workflow | Same tarball contract. |
| Windows | none | unsupported | Windows is unsupported until native path, service, and CI gates exist. |

The release workflow builds Linux and macOS artifacts from tag pushes with an
explicit platform matrix:

```text
ubuntu-latest -> linux-x86_64
ubuntu-24.04-arm -> linux-aarch64
macos-latest -> macos-arm64
macos-13 -> macos-x86_64
```

Each archive must contain `bin/cortexdb`, `bin/cortex-server`,
`package_manifest.json`, `SHA256SUMS`, and install notes. The workflow passes
the expected platform string into `make binary-release-check`; archive naming is
not inferred from `uname`.

## Clean Install Smoke

`make binary-release-check` validates a local archive by extracting it and
running this flow with the packaged binaries:

```text
clean install -> load fixture -> validate -> query -> backup -> restore
-> start server -> health -> HTTP query
```

The smoke report is:

```text
target/binary-platform-matrix/report.json
```

## Filesystem Requirements

CortexDB release artifacts are validated for local single-node use on
POSIX-style filesystems. Production-like local data directories must use a
filesystem that supports:

- append writes and file `fsync` for WAL durability;
- atomic `rename` within the same directory for segment, index, manifest, and
  dashboard/package artifact publication;
- parent-directory durability after rename where the platform exposes it;
- exclusive lock-file creation semantics for `db.lock`;
- regular files and directories with executable bits for installed binaries.

Recommended local filesystems:

| Platform | Recommended filesystem | Notes |
| --- | --- | --- |
| Linux | ext4 or XFS | Primary validation target for local smoke and CI. |
| macOS | APFS | Release workflow builds artifacts; operators should validate data on the target machine before upgrade. |

Avoid network filesystems, cloud-sync folders, container overlay paths, and
shared volumes for production-like data unless a separate operator validation
run proves `fsync`, rename, and lock semantics for that environment.

## macOS launchd

macOS is a serious local target for the binary tarball. The launchd example is
checked in at:

```text
docs/deployment/com.cortexdb.server.plist
```

It is an operator example, not an installer. Operators still need to create the
data/log directories and configure authentication secrets outside the plist.
`make service-manager-smoke-check` validates the checked-in launchd plist and
Linux systemd unit before they are referenced by deployment evidence.

## Windows Boundary

Windows is unsupported because the current release process does not validate:

- native path rules;
- service lifecycle;
- binary package contents;
- clean install smoke;
- backup/restore behavior;
- HTTP server lifecycle.

Do not publish Windows artifacts or claim Windows support until those gates
exist and pass.
