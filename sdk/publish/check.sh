#!/usr/bin/env sh
set -eu

ROOT="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$ROOT/.." && pwd)"
cleanup() {
  rm -rf "$ROOT/python/__pycache__" "$ROOT/python/build" "$ROOT/python"/*.egg-info
}
trap cleanup EXIT
cleanup

ROOT_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$REPO_ROOT/Cargo.toml" | head -1)"
PY_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/python/pyproject.toml" | head -1)"
TS_VERSION="$(sed -n 's/.*"version": "\(.*\)".*/\1/p' "$ROOT/typescript/package.json" | head -1)"
test "$ROOT_VERSION" = "$PY_VERSION"
test "$ROOT_VERSION" = "$TS_VERSION"

grep -q 'class AnnEvaluationResponse' "$ROOT/python/cortexdb_client.py"
grep -q 'def evaluate_ann_response' "$ROOT/python/cortexdb_client.py"
grep -q 'interface AnnEvaluationResponse' "$ROOT/typescript/cortexdb-client.d.ts"
grep -q 'evaluateAnn(scope' "$ROOT/typescript/cortexdb-client.d.ts"
grep -q '"low_recall"' "$ROOT/typescript/cortexdb-client.d.ts"
grep -q 'min_recall_q16' "$ROOT/python/cortexdb_client.py"
grep -q 'min_recall_q16' "$ROOT/typescript/cortexdb-client.d.ts"
grep -q 'with_tenant' "$ROOT/python/cortexdb_client.py"
grep -q 'withTenant' "$ROOT/typescript/cortexdb-client.d.ts"

python3 -m py_compile "$ROOT/python/cortexdb_client.py"
PYTHONPATH="$ROOT/python" python3 -m unittest discover "$ROOT/python"
PY_WHEEL_DIR="${TMPDIR:-/tmp}/cortexdb-python-wheel-check"
rm -rf "$PY_WHEEL_DIR"
python3 -m pip wheel --no-deps --no-build-isolation --wheel-dir "$PY_WHEEL_DIR" "$ROOT/python" >/dev/null
rm -rf "$PY_WHEEL_DIR"

cargo test -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml"
cargo package -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml" --allow-dirty >/dev/null

if command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/typescript" && node --check cortexdb-client.js && node --input-type=module <<'NODE'
import { CortexDBClient } from "./cortexdb-client.js";

let observedUrl = "";
globalThis.fetch = async (url, init) => {
  observedUrl = String(url);
  if (init.method !== "POST") throw new Error("expected POST");
  return {
    ok: true,
    json: async () => ({
      available: false,
      reason: "requires_persisted_checkpoint_without_wal_tail",
      ann_report: null,
      exact_top_k: [],
      ann_top_k: [],
      overlap_count: 0,
      recall_q16: 0,
    }),
    text: async () => "",
  };
};

const client = new CortexDBClient("http://127.0.0.1:8181").withTenant("tenant:alpha");
const response = await client.evaluateAnn("project:investments", [1, 2, 3], 20);
if (!observedUrl.includes("/v1/search/ann-evaluate?")) throw new Error(observedUrl);
if (!observedUrl.includes("scope=project%3Ainvestments")) throw new Error(observedUrl);
if (!observedUrl.includes("vector=1%2C2%2C3")) throw new Error(observedUrl);
if (!observedUrl.includes("tenant=tenant%3Aalpha")) throw new Error(observedUrl);
if (response.available !== false) throw new Error("unexpected response");
NODE
  npm pack --dry-run >/dev/null)
fi

printf 'sdk publish checks passed\n'
