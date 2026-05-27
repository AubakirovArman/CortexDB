#!/usr/bin/env python3
"""Ingest Russian dummy data into CortexDB via /v1/cell API (direct put)."""
import json
import os
import urllib.request
import urllib.parse
from pathlib import Path

CORTEX_HOST = os.getenv("CORTEX_HOST", "http://127.0.0.1:8090")
DATA_DIR = Path(__file__).parent / "data"


def put_cell(cell_id: int, payload_text: str) -> dict:
    url = f"{CORTEX_HOST}/v1/cell?cell_id={cell_id}"
    req = urllib.request.Request(
        url,
        data=payload_text.encode("utf-8"),
        headers={"Content-Type": "text/plain"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as res:
            return json.loads(res.read().decode("utf-8"))
    except Exception as e:
        return {"error": str(e), "cell_id": cell_id}


def main():
    jsonl_files = sorted(DATA_DIR.rglob("*.jsonl"))
    if not jsonl_files:
        print(f"No .jsonl files found in {DATA_DIR}")
        return

    total = 0
    success = 0
    errors = []
    cell_id = 1

    for filepath in jsonl_files:
        print(f"Processing {filepath.relative_to(DATA_DIR)} ...")
        with open(filepath, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                total += 1
                try:
                    record = json.loads(line)
                except json.JSONDecodeError as e:
                    errors.append(f"JSON parse error in {filepath}: {e}")
                    continue

                payload = record.get("payload_text", "")
                if not payload:
                    errors.append(f"Empty payload in {filepath}")
                    continue

                result = put_cell(cell_id, payload)
                if "error" in result:
                    errors.append(f"Put failed for cell {cell_id}: {result['error']}")
                else:
                    success += 1
                    print(f"  [{success}] cell_id={cell_id} → ok")
                cell_id += 1

    print(f"\n=== Ingestion complete ===")
    print(f"Total: {total}, Success: {success}, Errors: {len(errors)}")
    if errors:
        print("Errors:")
        for e in errors[:10]:
            print(f"  - {e}")
        if len(errors) > 10:
            print(f"  ... and {len(errors) - 10} more")


if __name__ == "__main__":
    main()
