#!/usr/bin/env python3
"""Emit the production receipt operator evidence handoff report."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from receipt_production_evidence_handoff_payload import operator_handoff


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(operator_handoff(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"receipt production evidence handoff written: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
