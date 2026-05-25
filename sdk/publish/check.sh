#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

python3 -m py_compile "$ROOT/python/cortexdb_client.py"

if command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/typescript" && npm pack --dry-run >/dev/null)
fi

printf 'sdk publish checks passed\n'
