#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-target/crash-fault}"
REPORT="${2:-$ROOT/report.json}"

rm -rf "$ROOT"
mkdir -p "$ROOT" "$(dirname "$REPORT")"

run_cli() {
  cargo run -q -p cortex-cli -- "$@"
}

run_test() {
  local name="$1"
  shift
  "$@" >"$ROOT/$name.log" 2>&1
}

run_test crash_matrix cargo test -p cortex-engine --test crash_matrix -- --nocapture
run_test restart_matrix cargo test -p cortex-engine --test restart_matrix -- --nocapture
run_test corruption_matrix cargo test -p cortex-engine --test corruption_matrix -- --nocapture
run_test repair_tests cargo test -p cortex-engine --test repair_tests -- --nocapture

DB="$ROOT/cli-partial-wal/db"
run_cli put "$DB" 1 "scope=ops
status=ready
crash fault payload" >/dev/null

WAL="$DB/db.aclog"
VALID_WAL_BYTES="$(wc -c <"$WAL" | tr -d ' ')"
printf 'partial-tail-after-crash' >>"$WAL"
printf 'orphan temp' >"$DB/db.aclog.tmp"

REPAIR_OUTPUT="$(run_cli repair "$DB")"
VALIDATE_OUTPUT="$(run_cli validate "$DB")"
PAYLOAD="$(run_cli get "$DB" 1)"
WAL_BYTES_AFTER="$(wc -c <"$WAL" | tr -d ' ')"

case "$REPAIR_OUTPUT" in
  *"orphan_temp_files_removed=1"*) ;;
  *)
    echo "repair did not remove exactly one orphan temp file" >&2
    echo "$REPAIR_OUTPUT" >&2
    exit 1
    ;;
esac

case "$REPAIR_OUTPUT" in
  *"wal_truncated=true"*) ;;
  *)
    echo "repair did not truncate partial WAL tail" >&2
    echo "$REPAIR_OUTPUT" >&2
    exit 1
    ;;
esac

if [ -e "$DB/db.aclog.tmp" ]; then
  echo "orphan WAL temp file still exists after repair" >&2
  exit 1
fi

if [ "$WAL_BYTES_AFTER" != "$VALID_WAL_BYTES" ]; then
  echo "WAL size after repair does not match the pre-fault safe size" >&2
  exit 1
fi

if [ "$PAYLOAD" != "scope=ops
status=ready
crash fault payload" ]; then
  echo "readback payload mismatch after repair" >&2
  exit 1
fi

export ROOT REPORT
export VALID_WAL_BYTES WAL_BYTES_AFTER REPAIR_OUTPUT VALIDATE_OUTPUT PAYLOAD

python3 - <<'PY'
import json
import os

report = {
    "status": "ok",
    "root": os.environ["ROOT"],
    "targeted_tests": [
        {"name": "crash_matrix", "log": "crash_matrix.log"},
        {"name": "restart_matrix", "log": "restart_matrix.log"},
        {"name": "corruption_matrix", "log": "corruption_matrix.log"},
        {"name": "repair_tests", "log": "repair_tests.log"},
    ],
    "cli_partial_wal_repair": {
        "valid_wal_bytes": int(os.environ["VALID_WAL_BYTES"]),
        "wal_bytes_after_repair": int(os.environ["WAL_BYTES_AFTER"]),
        "repair_output": os.environ["REPAIR_OUTPUT"],
        "validate_output": os.environ["VALIDATE_OUTPUT"],
        "readback_payload": os.environ["PAYLOAD"],
        "orphan_temp_removed": True,
        "partial_tail_truncated": True,
        "payload_readable_after_repair": True,
    },
}

with open(os.environ["REPORT"], "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "crash/fault evidence written to $REPORT"
