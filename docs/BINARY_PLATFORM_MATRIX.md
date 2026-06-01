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

The release workflow currently builds Linux and macOS artifacts from tag pushes.
Each archive must contain `bin/cortexdb`, `bin/cortex-server`,
`package_manifest.json`, `SHA256SUMS`, and install notes.

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

## macOS launchd

macOS is a serious local target for the binary tarball. The launchd example is
checked in at:

```text
docs/deployment/com.cortexdb.server.plist
```

It is an operator example, not an installer. Operators still need to create the
data/log directories and configure authentication secrets outside the plist.

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
