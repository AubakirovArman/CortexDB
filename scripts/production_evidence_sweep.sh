#!/usr/bin/env sh
set -eu

ROOT="${1:-target/production-evidence}"
REPORT="${2:-$ROOT/report.json}"

mkdir -p "$ROOT"

GIT_SHA="$(git rev-parse HEAD 2>/dev/null || printf 'unknown')"
STARTED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
STEPS_FILE="$ROOT/steps.jsonl"
rm -f "$STEPS_FILE"

OVERALL_STATUS="passed"

run_step() {
  name="$1"
  shift
  log="$ROOT/$name.log"
  started_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  if "$@" >"$log" 2>&1; then
    status="passed"
    exit_code=0
  else
    exit_code=$?
    status="failed"
    OVERALL_STATUS="failed"
  fi
  finished_at="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
  printf '{"name":"%s","status":"%s","exit_code":%s,"started_at":"%s","finished_at":"%s","log":"%s"}\n' \
    "$name" "$status" "$exit_code" "$started_at" "$finished_at" "$log" >>"$STEPS_FILE"
}

run_step openapi_contract make openapi-contract-check
run_step backup_drill make backup-drill-check
run_step ann_release_evidence make ann-release-evidence-check
run_step ann_real_embedding_readiness make ann-real-embedding-readiness
run_step replication_partition make replication-partition-check

FINISHED_AT="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
{
  printf '{\n'
  printf '  "schema_version": 1,\n'
  printf '  "status": "%s",\n' "$OVERALL_STATUS"
  printf '  "git_sha": "%s",\n' "$GIT_SHA"
  printf '  "started_at": "%s",\n' "$STARTED_AT"
  printf '  "finished_at": "%s",\n' "$FINISHED_AT"
  printf '  "steps": [\n'
  awk 'NR > 1 { print "," } { printf "    %s", $0 } END { if (NR > 0) print "" }' "$STEPS_FILE"
  printf '  ],\n'
  printf '  "artifacts": {\n'
  printf '    "openapi_contract_log": "%s",\n' "$ROOT/openapi_contract.log"
  printf '    "backup_drill_report": "target/backup-drill/report.json",\n'
  printf '    "ann_release_evidence_root": "target/ann/release-evidence",\n'
  printf '    "ann_real_embedding_readiness_report": "target/ann/real-embedding/readiness.json",\n'
  printf '    "replication_partition_report": "target/replication-partition/report.json"\n'
  printf '  }\n'
  printf '}\n'
} >"$REPORT"

if [ "$OVERALL_STATUS" != "passed" ]; then
  printf 'production evidence sweep failed; see %s\n' "$REPORT" >&2
  exit 1
fi

printf 'production evidence sweep passed: %s\n' "$REPORT"
