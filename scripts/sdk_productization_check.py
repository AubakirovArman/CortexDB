#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


LANGUAGE_FILES = {
    "python": [
        "sdk/python/pyproject.toml",
        "sdk/python/cortexdb_client.py",
        "sdk/python/examples/basic.py",
        "sdk/python/test_cortexdb_client.py",
    ],
    "typescript": [
        "sdk/typescript/package.json",
        "sdk/typescript/cortexdb-client.ts",
        "sdk/typescript/cortexdb-client.d.ts",
        "sdk/typescript/examples/basic.mjs",
        "sdk/typescript/test.js",
    ],
    "rust": [
        "crates/cortex-sdk/Cargo.toml",
        "crates/cortex-sdk/src/lib.rs",
        "crates/cortex-sdk/src/types.rs",
        "crates/cortex-sdk/examples/basic.rs",
        "crates/cortex-sdk/examples/live_contract.rs",
    ],
}


def require_marker(path: Path, marker: str, errors: list[str]) -> None:
    if marker not in path.read_text(encoding="utf-8"):
        errors.append(f"{path.relative_to(ROOT)} missing marker: {marker}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/sdk-productization/report.json")
    args = parser.parse_args()

    errors = []
    for language, files in LANGUAGE_FILES.items():
        for file_name in files:
            if not (ROOT / file_name).exists():
                errors.append(f"{language}: missing {file_name}")

    for report_name in [
        "target/sdk-release-artifacts/report.json",
        "target/sdk-registry-gate/report.json",
        "target/sdk-e2e-release/report.json",
    ]:
        path = ROOT / report_name
        if not path.exists():
            errors.append(f"missing SDK report: {report_name}")
            continue
        report = json.loads(path.read_text(encoding="utf-8"))
        if report.get("status") != "passed":
            errors.append(f"{report_name} status is {report.get('status')!r}")

    require_marker(ROOT / "docs" / "archive" / "SDK_PRODUCTIZATION.md", "published public registry evidence", errors)
    require_marker(ROOT / "docs" / "archive" / "NEXT_60_EPICS.md", "| 7 | Python SDK Productization | closed |", errors)
    require_marker(ROOT / "docs" / "archive" / "NEXT_60_EPICS.md", "| 8 | TypeScript SDK Productization | closed |", errors)
    require_marker(ROOT / "docs" / "archive" / "NEXT_60_EPICS.md", "| 9 | Rust SDK Productization | closed |", errors)

    summary = {
        "schema_version": "cortexdb.sdk.productization.v1",
        "status": "passed" if not errors else "failed",
        "public_registry_publication_claimed": True,
        "languages": sorted(LANGUAGE_FILES),
        "release_gates": [
            "sdk-release-contract-check",
            "sdk-deprecation-check",
            "sdk-release-artifacts-check",
            "sdk-registry-gate-check",
            "sdk-contract-check",
        ],
        "errors": errors,
    }
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
