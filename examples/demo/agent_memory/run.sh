#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

DB_PATH="${1:-target/agent-memory-demo/db}"
rm -rf "$DB_PATH"

cargo run -q -p cortex-cli -- --json remember "$DB_PATH" project:investments \
  'REMEMBER "Prefer cited budget evidence" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;'

cargo run -q -p cortex-cli -- context --format json "$DB_PATH" project:investments \
  'RETRIEVE CONTEXT FOR TASK "memory preference" IN BRAIN default WHERE scope = project:investments AND type = "memory" AND memory_type = "decision" REQUIRE citations LIMIT 10 CANDIDATES;'

cargo run -q -p cortex-cli -- verify --format json "$DB_PATH" project:investments \
  'VERIFY FACT "Prefer cited budget evidence" IN BRAIN default;'
