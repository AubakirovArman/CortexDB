# Package Manager Feasibility

Status: feasibility decision for beta packaging.

This document evaluates whether CortexDB is ready for package-manager
distribution. It does not claim that CortexDB is already published through any
package manager.

## Current Inputs

CortexDB already has:

- binary tarball packaging through `make binary-release-check`;
- archive and sidecar checksums;
- package-internal `package_manifest.json` and `SHA256SUMS`;
- checksum-verifying install script through `scripts/install.sh`;
- systemd and launchd service examples;
- release artifact manifest with binary, evidence, SDK, OpenAPI, and storage
  format metadata.

## Homebrew Formula Evaluation

Decision: feasible as the first package-manager path, preferably through a
project-owned tap.

Why:

- Homebrew formulae can target a versioned URL and require a SHA-256 checksum.
- CortexDB already produces versioned tarballs with external `.sha256`
  sidecars and package-internal checksums.
- Homebrew formula tests can run installed commands such as
  `cortexdb --version` and a small CLI smoke check.
- macOS service integration can reuse the existing launchd plist conventions.

Candidate formula shape:

```ruby
class Cortexdb < Formula
  desc "Single-node durable context database with AQL and Context Packs"
  homepage "https://github.com/AubakirovArman/CortexDB"
  url "https://github.com/AubakirovArman/CortexDB/releases/download/vX.Y.Z/cortexdb-vX.Y.Z-macos-arm64.tar.gz"
  sha256 "<release checksum>"
  license "Apache-2.0"

  def install
    bin.install "bin/cortexdb"
    bin.install "bin/cortex-server"
  end

  test do
    system "#{bin}/cortexdb", "--version"
  end
end
```

Readiness:

| Requirement | Status | Evidence |
| --- | --- | --- |
| Versioned archive | ready | `target/release-artifacts/*.tar.gz` |
| SHA-256 checksum | ready | external `.tar.gz.sha256` sidecar |
| CLI smoke command | ready | `cortexdb --version`, CLI tests |
| macOS service convention | ready | `docs/LAUNCHD.md` |
| Tap formula file | not created | future packaging PR |
| Bottle CI | deferred | requires tap/release workflow integration |

Go/no-go: Homebrew tap is feasible after the release workflow publishes stable
macOS archives and the project adds a checked formula template. Publishing to
`homebrew-core` is deferred until public adoption and policy review.

## Linux Package Evaluation

Decision: feasible after adding package metadata templates. Prefer project-owned
`.deb` and `.rpm` artifacts before attempting official distribution repos.

Why:

- Linux packaging needs more than a tarball: control metadata, install paths,
  permissions, maintainer scripts, service integration, and upgrade/rollback
  behavior.
- CortexDB already has the core ingredients: Linux binary archive, systemd
  service example, config/data/log path conventions, and upgrade/rollback CLI.
- The missing work is packaging-specific metadata and CI validation with package
  tools.

Candidate `.deb` contents:

```text
/usr/bin/cortexdb
/usr/bin/cortex-server
/usr/lib/systemd/system/cortexdb.service
/etc/cortexdb/auth.tokens.example
/usr/share/doc/cortexdb/
```

Candidate `.rpm` contents should mirror the same paths and service contract.

Readiness:

| Requirement | Status | Evidence |
| --- | --- | --- |
| Linux binary archive | ready | `make binary-release-check` |
| Checksum manifest | ready | `package_manifest.json`, `SHA256SUMS` |
| systemd service convention | ready | `docs/SYSTEMD.md` |
| Upgrade/rollback flow | ready | `cortexdb upgrade prepare/validate/rollback` |
| Debian control metadata | missing | future `packaging/debian/` |
| RPM spec metadata | missing | future `packaging/rpm/` |
| Package install smoke | missing | future CI gate |

Go/no-go: Linux `.deb` and `.rpm` packaging is feasible, but not release-ready
until package templates and install/upgrade smoke tests exist. Official Debian,
Ubuntu, Fedora, or distro repository submission is out of scope for beta.

## Decision

The recommended order is:

1. Keep tarball releases as the source of truth.
2. Add a checked Homebrew tap formula template.
3. Add local `.deb` and `.rpm` metadata templates.
4. Add package install smoke tests for clean install, upgrade, rollback, and
   uninstall.
5. Only then consider public package-manager publication.

## References

- Homebrew Formula Cookbook: https://docs.brew.sh/Formula-Cookbook
- Homebrew Bottles: https://docs.brew.sh/Bottles
- Debian binary package policy: https://www.debian.org/doc/debian-policy/ch-binary.html
- Debian operating system policy for systemd units: https://www.debian.org/doc/debian-policy/ch-opersys.html
