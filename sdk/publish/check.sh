#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
cleanup() {
  rm -rf "$ROOT/python/__pycache__" "$ROOT/python/build" "$ROOT/python"/*.egg-info
}
trap cleanup EXIT
cleanup

python3 -m py_compile "$ROOT/python/cortexdb_client.py"
PYTHONPATH="$ROOT/python" python3 -m unittest discover "$ROOT/python"
PY_WHEEL_DIR="${TMPDIR:-/tmp}/cortexdb-python-wheel-check"
rm -rf "$PY_WHEEL_DIR"
python3 -m pip wheel --no-deps --no-build-isolation --wheel-dir "$PY_WHEEL_DIR" "$ROOT/python" >/dev/null
rm -rf "$PY_WHEEL_DIR"

cargo test -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml"
cargo package -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml" --allow-dirty >/dev/null

if command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/typescript" && node --check cortexdb-client.js && npm pack --dry-run >/dev/null)
fi

printf 'sdk publish checks passed\n'
