#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-target/backup-offsite}"
REPORT="${2:-$ROOT/report.json}"
BACKUP_ID="${3:-cortexdb-20260530T000000Z}"

rm -rf "$ROOT"
mkdir -p "$ROOT" "$(dirname "$REPORT")"

run_cli() {
  cargo run -q -p cortex-cli -- "$@"
}

DB="$ROOT/db"
BACKUP="$ROOT/local-backup"
DRILL="$ROOT/local-drill"
OFFSITE="$ROOT/offsite-target"
STAGED="$OFFSITE/$BACKUP_ID"

run_cli put "$DB" 1 "scope=ops
status=ready
offsite payload 1" >/dev/null
run_cli put "$DB" 2 "scope=ops
status=ready
offsite payload 2" >/dev/null
run_cli flush "$DB" >/dev/null
run_cli put "$DB" 3 "scope=ops
status=ready
offsite payload 3" >/dev/null

DRILL_OUTPUT="$(run_cli backup-drill "$DB" "$BACKUP" "$DRILL")"
STAGE_OUTPUT="$(run_cli backup-offsite-stage "$BACKUP" "$OFFSITE" "$BACKUP_ID")"
VALIDATE_OUTPUT="$(run_cli validate "$STAGED")"
PAYLOAD="$(run_cli get "$STAGED" 3)"

case "$STAGE_OUTPUT" in
  *"target_path=$STAGED"*) ;;
  *)
    echo "offsite stage output did not report expected target" >&2
    echo "$STAGE_OUTPUT" >&2
    exit 1
    ;;
esac

case "$VALIDATE_OUTPUT" in
  "ok "* ) ;;
  *)
    echo "staged offsite copy did not validate" >&2
    echo "$VALIDATE_OUTPUT" >&2
    exit 1
    ;;
esac

if [ "$PAYLOAD" != "scope=ops
status=ready
offsite payload 3" ]; then
  echo "staged offsite readback payload mismatch" >&2
  exit 1
fi

if [ -e "$OFFSITE/$BACKUP_ID.staging" ] || [ -e "$OFFSITE/$BACKUP_ID.preflight-restore" ]; then
  echo "offsite staging or preflight restore directory was not cleaned up" >&2
  exit 1
fi

export ROOT REPORT BACKUP_ID DRILL_OUTPUT STAGE_OUTPUT VALIDATE_OUTPUT PAYLOAD STAGED

python3 - <<'PY'
import json
import os

report = {
    "status": "ok",
    "root": os.environ["ROOT"],
    "backup_id": os.environ["BACKUP_ID"],
    "staged_path": os.environ["STAGED"],
    "local_drill_output": os.environ["DRILL_OUTPUT"],
    "offsite_stage_output": os.environ["STAGE_OUTPUT"],
    "staged_validate_output": os.environ["VALIDATE_OUTPUT"],
    "readback_payload": os.environ["PAYLOAD"],
    "staged_copy_validated": True,
    "preflight_restore_completed": True,
    "payload_readable_after_stage": True,
}

with open(os.environ["REPORT"], "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "backup offsite evidence written to $REPORT"
