#!/usr/bin/env python3
import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def require_text(path: Path, markers: list[str]) -> list[str]:
    text = path.read_text(encoding="utf-8")
    missing = [marker for marker in markers if marker not in text]
    return [f"{path.relative_to(ROOT)} missing marker: {marker}" for marker in missing]


def run_tests() -> dict:
    command = [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "tool_registry_tests",
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return {
        "command": " ".join(command),
        "returncode": completed.returncode,
        "passed": completed.returncode == 0,
        "output_tail": completed.stdout[-4000:],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/tool-registry/report.json")
    args = parser.parse_args()

    checks = []
    checks.extend(
        require_text(
            ROOT / "docs" / "TOOL_REGISTRY.md",
            [
                "ToolDescriptor",
                "KnowledgeCellType::Tool",
                "permissions=read,execute,approval_required",
                "Database::list_tools(view)",
                "make tool-registry-check",
            ],
        )
    )
    checks.extend(
        require_text(
            ROOT / "docs" / "NEXT_60_EPICS.md",
            ["| 57 | Tool Registry | closed |"],
        )
    )
    checks.extend(
        require_text(
            ROOT / "crates" / "cortex-engine" / "src" / "tool_registry.rs",
            ["pub struct ToolDescriptor", "pub enum ToolPermission", "pub fn register_tool"],
        )
    )

    test_result = run_tests()
    report = {
        "tool_registry_docs_ok": not checks,
        "test_result": test_result,
        "errors": checks,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if checks or not test_result["passed"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
