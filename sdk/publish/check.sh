#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"

python3 -m py_compile "$ROOT/python/cortexdb_client.py"
PYTHONPATH="$ROOT/python" python3 -m unittest discover "$ROOT/python"
cargo test -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml"

if command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/typescript" && npm pack --dry-run >/dev/null)
fi

printf 'sdk publish checks passed\n'
