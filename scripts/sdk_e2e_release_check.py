#!/usr/bin/env python3
"""Validate SDK e2e and release-train evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_FILES = [
    Path("crates/cortex-sdk/Cargo.toml"),
    Path("crates/cortex-sdk/examples/live_contract.rs"),
    Path("sdk/python/pyproject.toml"),
    Path("sdk/python/cortexdb_client.py"),
    Path("sdk/python/examples/basic.py"),
    Path("sdk/typescript/package.json"),
    Path("sdk/typescript/cortexdb-client.ts"),
    Path("sdk/typescript/examples/basic.mjs"),
    Path("sdk/release-manifest.json"),
    Path(".github/workflows/sdk-release.yml"),
    Path("docs/SDK_RELEASE.md"),
    Path("docs/SDK_QUICKSTART.md"),
    Path("docs/SDK_DEPRECATION_POLICY.md"),
]

REQUIRED_MARKERS = {
    "live_sdk_contract": [
        ("scripts/check_sdk_contract.py", "Python SDK smoke test"),
        ("scripts/check_sdk_contract.py", "TypeScript SDK smoke test"),
        ("scripts/check_sdk_contract.py", "Rust SDK smoke test"),
        ("scripts/sdk_smoke_test.py", "missing_auth_error_contract"),
        ("scripts/sdk_ts_smoke_test.mjs", "missing_auth_error_contract"),
        ("crates/cortex-sdk/examples/live_contract.rs", "missing_auth_error_contract"),
    ],
    "release_contract": [
        ("scripts/check_sdk_release_contract.py", "requires_explicit_publish_input"),
        ("scripts/check_sdk_release_contract.py", "npm publish --access public --provenance"),
        ("docs/SDK_RELEASE.md", "publish=true"),
        ("docs/SDK_RELEASE.md", "protected `sdk-release` environment"),
    ],
    "release_artifacts": [
        ("scripts/sdk_release_artifacts_check.py", "Package SDK examples"),
        ("sdk/release-manifest.json", "sdk_examples_archive"),
        ("sdk/python/examples/basic.py", "CortexDBClient"),
        ("sdk/typescript/examples/basic.mjs", "CortexDBClient"),
        ("docs/SDK_RELEASE.md", "SDK examples artifact"),
    ],
    "deprecation_policy": [
        ("scripts/check_sdk_deprecation_policy.py", "SDK clients MUST NOT expose deprecated compatibility aliases"),
        ("docs/SDK_DEPRECATION_POLICY.md", "Breaking SDK/API Changes"),
    ],
    "quickstart": [
        ("docs/SDK_QUICKSTART.md", "Rust"),
        ("docs/SDK_QUICKSTART.md", "Python"),
        ("docs/SDK_QUICKSTART.md", "TypeScript"),
        ("docs/SDK_QUICKSTART.md", "make sdk-contract-check"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate_manifest() -> list[str]:
    failures: list[str] = []
    manifest = json.loads(read(Path("sdk/release-manifest.json")))
    packages = manifest.get("packages")
    if not isinstance(packages, list):
        return ["sdk/release-manifest.json: packages must be a list"]
    languages = {item.get("language") for item in packages if isinstance(item, dict)}
    for language in ("rust", "python", "typescript"):
        if language not in languages:
            failures.append(f"sdk/release-manifest.json: missing {language} package")
    return failures


def validate() -> dict[str, object]:
    failures: list[str] = []
    for path in REQUIRED_FILES:
        if not path.exists():
            failures.append(f"missing required file: {path}")
    failures.extend(validate_manifest())

    checks: dict[str, bool] = {}
    for check_name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{check_name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[check_name] = ok

    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "checks": checks,
        "packages": ["rust", "python", "typescript"],
        "release_artifacts": ["target/sdk-release-artifacts/cortexdb-sdk-examples-<version>.tar.gz"],
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except (OSError, RuntimeError, json.JSONDecodeError) as error:
        print(f"sdk e2e release check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"sdk e2e release check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
