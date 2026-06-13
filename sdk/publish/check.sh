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
PY_EXPECTED_VERSION="$(python3 - "$ROOT_VERSION" <<'PY'
import re
import sys

version = sys.argv[1]
match = re.fullmatch(r"(\d+\.\d+\.\d+)-beta\.(\d+)", version)
print(f"{match.group(1)}b{match.group(2)}" if match else version)
PY
)"
test "$PY_EXPECTED_VERSION" = "$PY_VERSION"
test "$ROOT_VERSION" = "$TS_VERSION"

python3 "$REPO_ROOT/scripts/check_sdk_release_contract.py"

PY_CLIENT="$ROOT/python/_cortexdb_client/client.py"
PY_MODELS="$ROOT/python/_cortexdb_client/model_types"
PY_GENERATED="$ROOT/python/_cortexdb_client/generated/openapi_types.py"
TS_TYPES="$ROOT/typescript/cortexdb-client/types"
TS_GENERATED="$ROOT/typescript/cortexdb-client/generated/openapi-types.ts"
TS_CLIENT="$ROOT/typescript/cortexdb-client/client.ts"
grep -R -q 'class HealthResponse' "$PY_MODELS"
grep -q 'OpenApiHealthResponse' "$PY_GENERATED"
grep -q 'def health_response' "$PY_CLIENT"
grep -q 'interface HealthResponse' "$TS_TYPES/core.ts"
grep -q 'interface OpenApiHealthResponse' "$TS_GENERATED"
grep -q 'server_version' "$TS_TYPES/core.ts"
grep -R -q 'class AnnEvaluationResponse' "$PY_MODELS"
grep -q 'def evaluate_ann_response' "$PY_CLIENT"
grep -q 'interface AnnEvaluationResponse' "$TS_TYPES/search.ts"
grep -q 'evaluateAnn(' "$TS_CLIENT"
grep -q '"low_recall"' "$TS_TYPES/search.ts"
grep -R -q 'min_recall_q16' "$PY_MODELS"
grep -q 'min_recall_q16' "$TS_TYPES/search.ts"
grep -R -q 'hnsw_ef_construction' "$PY_MODELS"
grep -q 'hnsw_ef_construction' "$TS_TYPES/search.ts"
grep -q 'with_tenant' "$PY_CLIENT"
grep -q 'withTenant' "$TS_CLIENT"
grep -F -q 'export * from "./generated/openapi-types";' "$ROOT/typescript/cortexdb-client/types.ts"
grep -F -q 'export * from "./cortexdb-client/types";' "$ROOT/typescript/cortexdb-client.d.ts"
grep -q 'cortexdb-client.cjs' "$ROOT/typescript/package.json"

python3 -m py_compile "$ROOT/python/cortexdb_client.py"
python3 -m py_compile "$PY_GENERATED"
PYTHONPATH="$ROOT/python" python3 -m unittest discover "$ROOT/python"
PY_WHEEL_DIR="${TMPDIR:-/tmp}/cortexdb-python-wheel-check"
rm -rf "$PY_WHEEL_DIR"
python3 -m pip wheel --no-deps --wheel-dir "$PY_WHEEL_DIR" "$ROOT/python" >/dev/null
rm -rf "$PY_WHEEL_DIR"

cargo test -p cortex-api-types -p cortex-sdk --all-features --manifest-path "$REPO_ROOT/Cargo.toml"
cargo package -p cortex-api-types --manifest-path "$REPO_ROOT/Cargo.toml" --allow-dirty >/dev/null
# cortex-sdk depends on cortex-api-types. Cargo resolves versioned dependencies
# against crates.io before packaging, so verify the SDK package only after
# cortex-api-types has been published.
if [ "${CORTEX_API_TYPES_PUBLISHED:-0}" = "1" ]; then
  cargo package -p cortex-sdk --manifest-path "$REPO_ROOT/Cargo.toml" --allow-dirty >/dev/null
else
  printf 'skipping cortex-sdk package verification until cortex-api-types is published\n'
fi

if command -v npm >/dev/null 2>&1; then
  (cd "$ROOT/typescript" &&
  npm test >/dev/null &&
  npm run typecheck >/dev/null &&
  node --check cortexdb-client.js &&
  node --check cortexdb-client.cjs &&
  node --input-type=module <<'NODE'
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
