#!/usr/bin/env python3
"""Download official LongMemEval v1 cleaned dataset files."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
from pathlib import Path


FILES = {
    "longmemeval_oracle.json": (
        "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/"
        "longmemeval_oracle.json"
    ),
    "longmemeval_s_cleaned.json": (
        "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/"
        "longmemeval_s_cleaned.json"
    ),
    "longmemeval_m_cleaned.json": (
        "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/"
        "longmemeval_m_cleaned.json"
    ),
}

SPLITS = {
    "oracle": ["longmemeval_oracle.json"],
    "s": ["longmemeval_s_cleaned.json"],
    "m": ["longmemeval_m_cleaned.json"],
    "all": list(FILES),
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_json_list(path: Path) -> int:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, list):
        raise ValueError(f"{path}: expected JSON list")
    for index, row in enumerate(value):
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{index}: expected object")
        for field in [
            "question_id",
            "question_type",
            "question",
            "answer",
            "haystack_session_ids",
            "haystack_sessions",
            "haystack_dates",
            "answer_session_ids",
        ]:
            if field not in row:
                raise ValueError(f"{path}:{index}: missing {field}")
    return len(value)


def download(url: str, output: Path, force: bool) -> None:
    if output.exists() and not force:
        return
    tmp = output.with_suffix(output.suffix + ".tmp")
    print(f"downloading {url} -> {output}", file=sys.stderr)
    with urllib.request.urlopen(url) as response, tmp.open("wb") as handle:
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            handle.write(chunk)
    tmp.replace(output)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--split", choices=sorted(SPLITS), default="s")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--manifest", type=Path)
    args = parser.parse_args(argv)

    args.data_root.mkdir(parents=True, exist_ok=True)
    manifest_rows = []
    for name in SPLITS[args.split]:
        url = FILES[name]
        path = args.data_root / name
        download(url, path, args.force)
        row_count = validate_json_list(path)
        manifest_rows.append(
            {
                "file": str(path),
                "source_url": url,
                "rows": row_count,
                "sha256": sha256(path),
            }
        )
    manifest = {
        "schema_version": "cortexdb.longmemeval.v1.official_data_manifest.v1",
        "dataset": "xiaowu0162/longmemeval-cleaned",
        "split": args.split,
        "files": manifest_rows,
    }
    output = args.manifest or (args.data_root / "manifest.json")
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
