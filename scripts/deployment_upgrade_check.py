#!/usr/bin/env python3
"""Validate CortexDB deployment and upgrade documentation coverage."""

from __future__ import annotations

import json
from pathlib import Path


REQUIRED_DOC_MARKERS = {
    "docs/INSTALL.md": [
        "Binary Tarball",
        "Source Build",
        "First Database",
        "Install Verification",
        "make binary-release-check",
        "BINARY_PLATFORM_MATRIX.md",
    ],
    "docs/archive/SYSTEMD.md": [
        "[Service]",
        "ExecStart=/usr/local/bin/cortex-server",
        "CORTEXDB_AUTH_TOKENS_FILE",
        "/v1/validate",
    ],
    "docs/archive/LAUNCHD.md": [
        "launchctl bootstrap",
        "launchctl kickstart",
        "launchctl bootout",
        "/v1/validate",
    ],
    "docs/archive/UPGRADE_ROLLBACK.md": [
        "Pre-Upgrade Checklist",
        "cortexdb upgrade prepare",
        "cortexdb upgrade validate",
        "cortexdb upgrade rollback",
        "backup-drill",
        "Rollback",
        "make migration-policy-check",
        "make binary-release-check",
    ],
    "docs/OPERATIONS.md": [
        "First 10 Minutes",
        "Operational Runbooks",
        "Stale lock or `database_busy`",
        "Corrupt WAL or partial WAL tail",
        "Corrupt segment or index bundle",
        "Failed authentication",
        "Tenant errors",
        "make deployment-upgrade-check",
        "make observability-check",
        "make public-claims-check",
    ],
    "docs/archive/BINARY_RELEASES.md": [
        "GitHub Release Workflow",
        "Binary Platform Matrix",
        "target/release-artifacts",
        "SHA256SUMS",
    ],
    "docs/archive/BINARY_PLATFORM_MATRIX.md": [
        "Windows is unsupported",
        "Clean Install Smoke",
        "launchd",
    ],
    "docs/deployment/com.cortexdb.server.plist": [
        "/usr/local/bin/cortex-server",
        "/usr/local/var/cortexdb",
        "CORTEXDB_AUTH_TOKENS_FILE",
        "KeepAlive",
    ],
    "docs/deployment/cortexdb.service": [
        "EnvironmentFile=/etc/cortexdb/cortexdb.env",
        "NoNewPrivileges=true",
        "ProtectSystem=strict",
    ],
}

LINK_MARKERS = {
    "README.md": ["docs/INSTALL.md", "docs/archive/SYSTEMD.md", "docs/archive/UPGRADE_ROLLBACK.md"],
    "docs/DOCUMENTATION_INDEX.md": ["INSTALL.md", "SYSTEMD.md", "LAUNCHD.md", "UPGRADE_ROLLBACK.md", "BINARY_PLATFORM_MATRIX.md"],
    "docs/archive/PL_EXTRACTED_EPICS.md": ["DEPLOYMENT_UPGRADE_EVIDENCE.md"],
}


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def read_make_surface() -> str:
    parts = [read("Makefile")]
    parts.extend(path.read_text(encoding="utf-8") for path in sorted(Path("mk").glob("*.mk")))
    return "\n".join(parts)


def require_contains(path: str, markers: list[str], failures: list[str]) -> None:
    if not Path(path).is_file():
        failures.append(f"missing {path}")
        return
    text = read(path)
    for marker in markers:
        if marker not in text:
            failures.append(f"{path}: missing marker {marker!r}")


def main() -> int:
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/deployment-upgrade/report.json")
    args = parser.parse_args()

    failures: list[str] = []
    for path, markers in REQUIRED_DOC_MARKERS.items():
        require_contains(path, markers, failures)
    for path, markers in LINK_MARKERS.items():
        require_contains(path, markers, failures)

    release = read(".github/workflows/release.yml")
    if "gh release upload" not in release:
        failures.append("release workflow does not upload GitHub release assets")
    if ".tar.gz" not in release or ".sha256" not in release:
        failures.append("release workflow does not mention tar.gz and sha256 assets")

    makefile = read_make_surface()
    for marker in ("deployment-upgrade-check", "service-manager-smoke-check", "binary-release-check", "migration-policy-check"):
        if marker not in makefile:
            failures.append(f"make surface: missing {marker}")

    cli = "\n".join(
        [
            read("crates/cortex-cli/src/cli/args/commands/subcommands.rs"),
            read("crates/cortex-cli/src/cli/args/commands.rs"),
            read("crates/cortex-cli/src/cli/dispatch/upgrade_flow.rs"),
        ]
    )
    for marker in ("UpgradeCommand", "Prepare", "Validate", "Rollback"):
        if marker not in cli:
            failures.append(f"CLI upgrade flow missing {marker}")

    cli_upgrade = read("crates/cortex-cli/src/cli_upgrade.rs")
    for marker in ("upgrade_prepare", "upgrade_validate", "upgrade_rollback"):
        if marker not in cli_upgrade:
            failures.append(f"CLI upgrade implementation missing {marker}")

    report = {
        "schema_version": 1,
        "status": "failed" if failures else "passed",
        "docs_checked": sorted(REQUIRED_DOC_MARKERS),
        "links_checked": sorted(LINK_MARKERS),
        "release_workflow_checked": True,
        "failures": failures,
    }
    path = Path(args.report)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"deployment upgrade check failed: {path}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"deployment upgrade check passed: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
