#!/usr/bin/env python3
"""Generate CortexDB release notes from local evidence reports."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def git_value(repo: Path, args: list[str]) -> str:
    result = subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def section_lines(markdown: str, heading: str) -> list[str]:
    lines = markdown.splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.strip() == heading:
            start = index + 1
            break
    if start is None:
        return []
    out: list[str] = []
    for line in lines[start:]:
        if line.startswith("## "):
            break
        if line.strip():
            out.append(line)
    return out


def bullet_items(lines: list[str]) -> list[str]:
    items: list[str] = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith("- "):
            items.append(stripped[2:])
        elif items and stripped:
            items[-1] = f"{items[-1]} {stripped}"
    return items


def production_steps(report: dict[str, Any]) -> list[str]:
    steps = report.get("steps")
    if not isinstance(steps, list):
        return []
    out: list[str] = []
    for step in steps:
        if not isinstance(step, dict):
            continue
        name = step.get("name", "unknown")
        status = step.get("status", "unknown")
        log = step.get("log", "")
        out.append(f"- `{name}`: `{status}`" + (f" ({log})" if log else ""))
    return out


def bundle_summary(report: dict[str, Any]) -> list[str]:
    categories = report.get("artifact_count_by_category")
    lines = [
        f"- status: `{report.get('status', 'unknown')}`",
        f"- artifact_count: `{report.get('artifact_count', 0)}`",
    ]
    if isinstance(categories, dict):
        category_text = ", ".join(f"{key}={value}" for key, value in sorted(categories.items()))
        lines.append(f"- artifact categories: `{category_text}`")
    if report.get("archive"):
        lines.append(f"- archive: `{report['archive']}`")
    if report.get("archive_sha256_sidecar"):
        lines.append(f"- archive checksum sidecar: `{report['archive_sha256_sidecar']}`")
    return lines


def manifest_summary(manifest: dict[str, Any]) -> list[str]:
    sdk_versions = manifest.get("sdk_versions", {})
    storage_formats = manifest.get("storage_format_versions", [])
    lines = [
        f"- artifact_count: `{manifest.get('artifact_count', 0)}`",
        f"- OpenAPI version: `{manifest.get('openapi', {}).get('version', 'unknown')}`",
    ]
    if isinstance(sdk_versions, dict):
        lines.append(f"- SDK workspace version: `{sdk_versions.get('workspace', 'unknown')}`")
    if isinstance(storage_formats, list):
        lines.append(f"- storage formats: `{len(storage_formats)}`")
    has_bundle = any(isinstance(item, dict) and item.get("kind") == "release_evidence_bundle" for item in manifest.get("artifacts", []))
    lines.append(f"- release evidence bundle bound: `{str(has_bundle).lower()}`")
    return lines


def render(args: argparse.Namespace) -> str:
    repo = repo_root()
    production = read_json(repo / args.production_evidence_report)
    bundle = read_json(repo / args.evidence_bundle_report)
    manifest = read_json(repo / args.release_manifest)
    beta_doc = (repo / "docs/archive/BETA_RELEASE.md").read_text(encoding="utf-8")
    migration_doc = (repo / "docs/archive/UPGRADE_MIGRATION.md").read_text(encoding="utf-8")

    non_goals = bullet_items(section_lines(beta_doc, "## Explicit Non-Goals For Beta"))
    limitations = bullet_items(section_lines(migration_doc, "## Current Limitations"))
    compatibility = re.search(r"v0\.1\.0-core-alpha\.5\s*->\s*v0\.2\.0-beta\.1", migration_doc)

    lines = [
        f"# CortexDB {args.version} Generated Release Notes",
        "",
        f"Generated at: `{utc_now()}`",
        f"Git commit: `{git_value(repo, ['rev-parse', 'HEAD'])}`",
        "",
        "These notes are generated from local evidence reports. They are a release",
        "draft, not a public production SLA.",
        "",
        "## Evidence Gates",
        "",
        *production_steps(production),
        "",
        "## Evidence Bundle",
        "",
        *bundle_summary(bundle),
        "",
        "## Release Manifest",
        "",
        *manifest_summary(manifest),
        "",
        "## Migration Notes",
        "",
        f"- release compatibility pair: `{'v0.1.0-core-alpha.5 -> v0.2.0-beta.2' if compatibility else 'not detected'}`",
        "- upgrade policy: `docs/archive/UPGRADE_MIGRATION.md`",
        "- rollback policy: restore from immutable pre-upgrade backup; no in-place downgrade guarantee.",
        "",
        "## Known Limitations",
        "",
    ]
    lines.extend(f"- {item}" for item in limitations)
    lines.extend(["", "## Explicit Non-Goals", ""])
    lines.extend(f"- {item}" for item in non_goals)
    lines.extend(["", "## Required Release Artifacts", ""])
    lines.extend(
        [
            "- `target/release-evidence-bundle/release-evidence.tar.gz`",
            "- `target/release-evidence-bundle/release-evidence.tar.gz.sha256`",
            "- `target/release-artifact-manifest/manifest.json`",
            "- `target/release-artifact-manifest/report.json`",
        ]
    )
    return "\n".join(lines).rstrip() + "\n"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", default="dev")
    parser.add_argument("--production-evidence-report", default="target/production-evidence/report.json")
    parser.add_argument("--evidence-bundle-report", default="target/release-evidence-bundle/report.json")
    parser.add_argument("--release-manifest", default="target/release-artifact-manifest/manifest.json")
    parser.add_argument("--output", default="target/release-notes/generated.md")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    repo = repo_root()
    try:
        text = render(args)
    except Exception as error:  # noqa: BLE001 - release note gate reports failures.
        print(f"error: {error}", file=sys.stderr)
        return 1
    output = repo / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")
    print(f"generated release notes: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
