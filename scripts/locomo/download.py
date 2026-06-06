#!/usr/bin/env python3
"""Download the official SNAP Research LoCoMo dataset file."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
from pathlib import Path
from typing import Any


DATASET = "snap-research/locomo"
SOURCE_URL = "https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def download(output: Path, force: bool) -> None:
    if output.exists() and not force:
        return
    tmp = output.with_suffix(output.suffix + ".tmp")
    print(f"downloading {SOURCE_URL} -> {output}", file=sys.stderr)
    request = urllib.request.Request(SOURCE_URL, headers={"User-Agent": "CortexDB-LoCoMo"})
    with urllib.request.urlopen(request, timeout=60) as response, tmp.open("wb") as handle:
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            handle.write(chunk)
    tmp.replace(output)


def session_keys(conversation: dict[str, Any]) -> list[str]:
    keys = []
    for key, value in conversation.items():
        if key.startswith("session_") and not key.endswith("_date_time") and isinstance(value, list):
            keys.append(key)
    return sorted(keys, key=lambda value: int(value.split("_", 1)[1]))


def validate(path: Path) -> dict[str, int]:
    data = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(data, list), f"{path}: expected JSON list")
    qa_count = 0
    evidence_count = 0
    turn_count = 0
    for sample_index, sample in enumerate(data, start=1):
        require(isinstance(sample, dict), f"{path}:{sample_index}: expected object")
        for field in ["sample_id", "conversation", "qa"]:
            require(field in sample, f"{path}:{sample_index}: missing {field}")
        conversation = sample["conversation"]
        require(isinstance(conversation, dict), f"{path}:{sample_index}: conversation must be object")
        require(session_keys(conversation), f"{path}:{sample_index}: no sessions")
        for key in session_keys(conversation):
            for turn in conversation[key]:
                require(isinstance(turn, dict), f"{path}:{sample_index}:{key}: turn must be object")
                for field in ["speaker", "dia_id", "text"]:
                    require(field in turn, f"{path}:{sample_index}:{key}: turn missing {field}")
                turn_count += 1
        qa = sample["qa"]
        require(isinstance(qa, list), f"{path}:{sample_index}: qa must be list")
        for qa_index, row in enumerate(qa, start=1):
            for field in ["question", "category"]:
                require(field in row, f"{path}:{sample_index}:qa:{qa_index}: missing {field}")
            qa_count += 1
            if row.get("evidence"):
                evidence_count += 1
    return {
        "samples": len(data),
        "qa_count": qa_count,
        "qa_with_evidence": evidence_count,
        "turn_count": turn_count,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args(argv)

    args.data_root.mkdir(parents=True, exist_ok=True)
    data_file = args.data_root / "locomo10.json"
    download(data_file, args.force)
    summary = validate(data_file)
    manifest = {
        "schema_version": "cortexdb.locomo.official_data_manifest.v1",
        "dataset": DATASET,
        "files": [{"file": str(data_file), "source_url": SOURCE_URL, "sha256": sha256(data_file), **summary}],
    }
    output = args.manifest or args.data_root / "manifest.json"
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
