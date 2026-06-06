#!/usr/bin/env bash
set -euo pipefail

ROOT="${1:-target/backup-drill}"
REPORT="${2:-$ROOT/report.json}"
KEEP_LATEST="${3:-2}"
PREFIX="${4:-cortexdb-}"

if [ "$KEEP_LATEST" -lt 1 ]; then
  echo "KEEP_LATEST must be greater than zero" >&2
  exit 2
fi

DB="$ROOT/db"
BACKUPS="$ROOT/backups"
DRILLS="$ROOT/drills"

rm -rf "$ROOT"
mkdir -p "$BACKUPS" "$DRILLS" "$(dirname "$REPORT")"

run_cli() {
  cargo run -q -p cortex-cli -- "$@"
}

write_cell() {
  local id="$1"
  local payload="$2"
  run_cli put "$DB" "$id" "$payload" >/dev/null
}

run_drill() {
  local stamp="$1"
  run_cli backup-drill "$DB" "$BACKUPS/$PREFIX$stamp" "$DRILLS/$PREFIX$stamp"
}

write_cell 1 "scope=ops
status=ready
backup drill payload 1"
DRILL_1="$(run_drill 20260530T000001Z)"

write_cell 2 "scope=ops
status=ready
backup drill payload 2"
run_cli flush "$DB" >/dev/null
DRILL_2="$(run_drill 20260530T000002Z)"

write_cell 3 "scope=ops
status=ready
backup drill payload 3"
DRILL_3="$(run_drill 20260530T000003Z)"

PRUNE_DRY_RUN_OUTPUT="$(run_cli backup-prune "$BACKUPS" "$PREFIX" "$KEEP_LATEST" --dry-run)"
if [ ! -d "$BACKUPS/${PREFIX}20260530T000001Z" ]; then
  echo "dry-run prune removed the oldest backup" >&2
  exit 1
fi

PRUNE_OUTPUT="$(run_cli backup-prune "$BACKUPS" "$PREFIX" "$KEEP_LATEST")"
LATEST_VALIDATE="$(run_cli validate "$DRILLS/${PREFIX}20260530T000003Z")"
LATEST_PAYLOAD="$(run_cli get "$DRILLS/${PREFIX}20260530T000003Z" 3)"
GIT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

OLDEST_BACKUP_PRUNED=false
if [ ! -d "$BACKUPS/${PREFIX}20260530T000001Z" ]; then
  OLDEST_BACKUP_PRUNED=true
fi

if [ "$KEEP_LATEST" -lt 3 ] && [ "$OLDEST_BACKUP_PRUNED" != true ]; then
  echo "oldest backup was not pruned" >&2
  exit 1
fi

if [ "$KEEP_LATEST" -ge 3 ] && [ "$OLDEST_BACKUP_PRUNED" != false ]; then
  echo "oldest backup should be retained when keep_latest >= backup count" >&2
  exit 1
fi

if [ "$KEEP_LATEST" -ge 2 ] && [ ! -d "$BACKUPS/${PREFIX}20260530T000002Z" ]; then
  echo "expected retained backup 20260530T000002Z is missing" >&2
  exit 1
fi

if [ ! -d "$BACKUPS/${PREFIX}20260530T000003Z" ]; then
  echo "expected retained backup 20260530T000003Z is missing" >&2
  exit 1
fi

export ROOT REPORT KEEP_LATEST PREFIX
export DRILL_1 DRILL_2 DRILL_3 PRUNE_DRY_RUN_OUTPUT PRUNE_OUTPUT LATEST_VALIDATE LATEST_PAYLOAD
export OLDEST_BACKUP_PRUNED GIT_SHA

python3 - <<'PY'
import json
import os

report = {
    "status": "ok",
    "root": os.environ["ROOT"],
    "git_sha": os.environ["GIT_SHA"],
    "backup_prefix": os.environ["PREFIX"],
    "keep_latest": int(os.environ["KEEP_LATEST"]),
    "release_artifacts": [
        os.environ["REPORT"],
        os.environ["ROOT"] + "/backups",
        os.environ["ROOT"] + "/drills",
    ],
    "drills": [
        os.environ["DRILL_1"],
        os.environ["DRILL_2"],
        os.environ["DRILL_3"],
    ],
    "restore_drill_trend": [
        {
            "release": "local-current",
            "git_sha": os.environ["GIT_SHA"],
            "drill_id": "20260530T000001Z",
            "validated": True,
        },
        {
            "release": "local-current",
            "git_sha": os.environ["GIT_SHA"],
            "drill_id": "20260530T000002Z",
            "validated": True,
        },
        {
            "release": "local-current",
            "git_sha": os.environ["GIT_SHA"],
            "drill_id": "20260530T000003Z",
            "validated": True,
        },
    ],
    "prune_dry_run": os.environ["PRUNE_DRY_RUN_OUTPUT"],
    "prune": os.environ["PRUNE_OUTPUT"],
    "latest_validate": os.environ["LATEST_VALIDATE"],
    "latest_payload": os.environ["LATEST_PAYLOAD"],
    "evidence": {
        "oldest_backup_pruned": os.environ["OLDEST_BACKUP_PRUNED"] == "true",
        "latest_backup_restored_and_readable": True,
    },
}

with open(os.environ["REPORT"], "w", encoding="utf-8") as handle:
    json.dump(report, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY

echo "backup drill evidence written to $REPORT"
