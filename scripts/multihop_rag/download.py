#!/usr/bin/env python3
"""Download official MultiHop-RAG JSON files."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
from pathlib import Path
from typing import Any


FILES = {
    "MultiHopRAG.json": (
        "https://huggingface.co/datasets/yixuantt/MultiHopRAG/resolve/main/"
        "MultiHopRAG.json"
    ),
    "corpus.json": (
        "https://huggingface.co/datasets/yixuantt/MultiHopRAG/resolve/main/"
        "corpus.json"
    ),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, output: Path, force: bool) -> None:
    if output.exists() and not force:
        return
    tmp = output.with_suffix(output.suffix + ".tmp")
    print(f"downloading {url} -> {output}", file=sys.stderr)
    request = urllib.request.Request(url, headers={"User-Agent": "CortexDB-MultiHopRAG"})
    with urllib.request.urlopen(request) as response, tmp.open("wb") as handle:
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            handle.write(chunk)
    tmp.replace(output)


def read_list(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, list):
        raise ValueError(f"{path}: expected JSON list")
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(value):
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{index}: expected object")
        rows.append(row)
    return rows


def validate_queries(path: Path) -> int:
    rows = read_list(path)
    required = ["query", "answer", "question_type", "evidence_list"]
    for index, row in enumerate(rows):
        for field in required:
            if field not in row:
                raise ValueError(f"{path}:{index}: missing {field}")
        if not isinstance(row["evidence_list"], list):
            raise ValueError(f"{path}:{index}: evidence_list must be a list")
    return len(rows)


def validate_corpus(path: Path) -> int:
    rows = read_list(path)
    required = ["title", "source", "published_at", "url", "body"]
    for index, row in enumerate(rows):
        for field in required:
            if field not in row:
                raise ValueError(f"{path}:{index}: missing {field}")
        if not isinstance(row["body"], str) or not row["body"].strip():
            raise ValueError(f"{path}:{index}: body must be non-empty")
    return len(rows)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)

    args.data_root.mkdir(parents=True, exist_ok=True)
    files = []
    for name, url in FILES.items():
        path = args.data_root / name
        download(url, path, args.force)
        if name == "MultiHopRAG.json":
            row_count = validate_queries(path)
        else:
            row_count = validate_corpus(path)
        files.append({"file": str(path), "rows": row_count, "sha256": sha256(path), "source_url": url})

    manifest = {
        "schema_version": "cortexdb.multihop_rag.official_data_manifest.v1",
        "dataset": "yixuantt/MultiHopRAG",
        "files": files,
    }
    output = args.manifest or (args.data_root / "manifest.json")
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
