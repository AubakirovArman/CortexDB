#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
DB_PATH="${1:-$ROOT/target/use-case-packs/technical-runbook-triage/demo-db}"
FIXTURE="$ROOT/examples/datasets/technical_docs"
SCOPE="docs:technical"

rm -rf "$DB_PATH"

echo "== Load technical docs fixture =="
cargo run -q -p cortex-cli -- load-fixture "$DB_PATH" "$FIXTURE"

echo "== Docs retrieval =="
cargo run -q -p cortex-cli -- search --json "$DB_PATH" "$SCOPE" "compatibility endpoint SDK contract"

echo "== Tool hints =="
cargo run -q -p cortex-cli -- context --format json "$DB_PATH" "$SCOPE" \
  'RETRIEVE CONTEXT FOR TASK "Find tool hints for compatibility diagnostics" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

echo "== Version conflict verification =="
cargo run -q -p cortex-cli -- verify --format json "$DB_PATH" "$SCOPE" \
  'VERIFY FACT "SDK contract v1.4 is incompatible with API contract v1.3" IN BRAIN default;'

echo "== Source refs =="
cargo run -q -p cortex-cli -- context --format json "$DB_PATH" "$SCOPE" \
  'RETRIEVE CONTEXT FOR TASK "List source refs for compatibility and migration matrix docs" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

echo "== Validate =="
cargo run -q -p cortex-cli -- validate "$DB_PATH"
