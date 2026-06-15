#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def check_markers(errors: list[str]) -> None:
    required = {
        "docs/TIERED_STORAGE_V2.md": [
            "Status: accepted design and guarded prototype.",
            "Placement Policy",
            "Compression Policy",
            "Query-Plan Prefetch",
            "resident_bytes <= max_bytes",
            "make tiered-storage-v2-check",
        ],
        "crates/cortex-engine/src/options.rs": [
            "pub enum TieredStorageCompressionPolicy",
            "pub struct TieredStorageOptions",
            "pub tiered_storage: TieredStorageOptions",
        ],
        "crates/cortex-engine/src/config.rs": [
            "CORTEXDB_TIERED_STORAGE_V2",
            "CORTEXDB_TIERED_STORAGE_COMPRESSION",
        ],
        "crates/cortex-engine/src/database/payload_cache.rs": [
            "pub max_bytes: usize",
            "pub hits: u64",
            "pub misses: u64",
            "pub evictions: u64",
        ],
        "crates/cortex-engine/tests/tiered_storage_v2.rs": [
            "tiered_storage_v2_serves_cold_payloads_with_bounded_hot_cache",
            "PayloadResidency::Lazy",
            "resident_bytes <= 64",
        ],
    }
    for rel, markers in required.items():
        text = read(rel)
        for marker in markers:
            if marker not in text:
                errors.append(f"{rel}: missing marker {marker!r}")


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "output_tail": completed.stdout[-4000:],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/tiered-storage-v2/report.json")
    args = parser.parse_args()

    errors: list[str] = []
    check_markers(errors)

    commands = [
        [
            "cargo",
            "test",
            "-p",
            "cortex-engine",
            "cache_returns_payload_and_evicts_least_recently_used_entry",
            "--lib",
            "--all-features",
        ],
        [
            "cargo",
            "test",
            "-p",
            "cortex-engine",
            "--test",
            "tiered_storage_v2",
            "--all-features",
        ],
    ]
    results = []
    for command in commands:
        result = run(command)
        results.append(result)
        if result["returncode"] != 0:
            errors.append(f"command failed: {' '.join(command)}")
            break

    report = {
        "status": "ok" if not errors else "failed",
        "errors": errors,
        "commands": results,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"tiered storage v2 check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
