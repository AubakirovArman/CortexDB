#!/usr/bin/env python3
"""Regenerate queries and ground truth by running build_corpus.py."""

from __future__ import annotations

import subprocess
import sys


if __name__ == "__main__":
    raise SystemExit(subprocess.call([sys.executable, "scripts/build_corpus.py"]))
