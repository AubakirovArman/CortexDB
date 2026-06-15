#!/usr/bin/env python3
"""Validate the mdBook docs-site contract."""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"
SUMMARY = DOCS / "SUMMARY.md"
BOOK = ROOT / "book.toml"
WORKFLOW = ROOT / ".github/workflows/docs-pages.yml"
LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")

BOOK_PATTERNS = {
    "book src": r'(?m)^\s*src\s*=\s*"docs"\s*$',
    "book build dir": r'(?m)^\s*build-dir\s*=\s*"target/mdbook"\s*$',
    "book search": r'(?m)^\s*search\s*=\s*true\s*$',
    "book site url": r'(?m)^\s*site-url\s*=\s*"/CortexDB/"\s*$',
}

WORKFLOW_MARKERS = [
    "name: Docs Pages",
    "pages: write",
    "id-token: write",
    "cargo install mdbook",
    "make docs-site-check",
    "actions/configure-pages",
    "actions/upload-pages-artifact",
    "actions/deploy-pages",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/docs-site/report.json")
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="Do not run mdbook build even when the binary is installed.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []

    check_book(failures)
    summary_links = check_summary(failures)
    check_workflow(failures)

    mdbook_path = shutil.which("mdbook")
    build_ran = False
    if mdbook_path and not args.skip_build:
        build_ran = True
        check_mdbook_build(failures)

    report = {
        "schema_version": "cortexdb.docs_site.report.v1",
        "status": "failed" if failures else "passed",
        "book": str(BOOK.relative_to(ROOT)),
        "summary": str(SUMMARY.relative_to(ROOT)),
        "summary_entries": len(summary_links),
        "required_top_level_docs": len(top_level_docs()),
        "workflow": str(WORKFLOW.relative_to(ROOT)),
        "mdbook_available": bool(mdbook_path),
        "mdbook_build_ran": build_ran,
        "build_dir": "target/mdbook",
        "failures": failures,
    }
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"docs site check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1

    if mdbook_path and build_ran:
        print(f"docs site check passed with mdBook build: {output}")
    else:
        print(f"docs site check passed structurally: {output}")
    return 0


def check_book(failures: list[str]) -> None:
    if not BOOK.is_file():
        failures.append("missing book.toml")
        return
    text = BOOK.read_text(encoding="utf-8")
    for name, pattern in BOOK_PATTERNS.items():
        if not re.search(pattern, text):
            failures.append(f"book.toml: missing {name}")


def check_summary(failures: list[str]) -> list[str]:
    if not SUMMARY.is_file():
        failures.append("missing docs/SUMMARY.md")
        return []
    text = SUMMARY.read_text(encoding="utf-8")
    links = [target for target in LINK_RE.findall(text) if target.endswith(".md")]
    seen: set[str] = set()
    for target in links:
        base = target.split("#", 1)[0]
        if base in seen:
            failures.append(f"docs/SUMMARY.md: duplicate link {base}")
        seen.add(base)
        if base.startswith("../") or base.startswith("archive/"):
            failures.append(f"docs/SUMMARY.md: non-core docs link {base}")
            continue
        candidate = (DOCS / base).resolve()
        if DOCS.resolve() not in candidate.parents or not candidate.is_file():
            failures.append(f"docs/SUMMARY.md: missing linked file {base}")

    for doc in top_level_docs():
        if doc.name not in seen:
            failures.append(f"docs/SUMMARY.md: missing top-level doc {doc.name}")

    if len(links) < 40:
        failures.append(f"docs/SUMMARY.md: expected at least 40 entries, found {len(links)}")
    return links


def top_level_docs() -> list[Path]:
    return sorted(
        path
        for path in DOCS.glob("*.md")
        if path.name != "SUMMARY.md" and path.is_file()
    )


def check_workflow(failures: list[str]) -> None:
    if not WORKFLOW.is_file():
        failures.append("missing .github/workflows/docs-pages.yml")
        return
    text = WORKFLOW.read_text(encoding="utf-8")
    for marker in WORKFLOW_MARKERS:
        if marker not in text:
            failures.append(f"docs-pages workflow: missing {marker!r}")


def check_mdbook_build(failures: list[str]) -> None:
    result = subprocess.run(
        ["mdbook", "build"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if result.returncode != 0:
        failures.append("mdbook build failed:\n" + result.stdout[-4000:])
        return

    build_dir = ROOT / "target/mdbook"
    for relative in ["index.html", "searchindex.js"]:
        if not (build_dir / relative).is_file():
            failures.append(f"mdbook build missing target/mdbook/{relative}")


if __name__ == "__main__":
    raise SystemExit(main())
