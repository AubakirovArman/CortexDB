#!/usr/bin/env sh
set -eu

ROOT="${1:-./data}"
HOST="${2:-http://127.0.0.1:8181}"

curl -fsS "$HOST/v1/health"
curl -fsS "$HOST/v1/stats"
curl -fsS "$HOST/v1/validate"
printf '\nroot=%s\n' "$ROOT"
