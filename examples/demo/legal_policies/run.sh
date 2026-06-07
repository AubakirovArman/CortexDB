#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT_DIR"

DB_PATH="./examples/demo/legal_policies/db_temp"
rm -rf "$DB_PATH"

echo "=== CortexDB Legal Policy Demo ==="
echo "1. Loading legal policy fixture..."
cargo run -q -p cortex-cli -- load-fixture "$DB_PATH" examples/datasets/legal_policies

echo ""
echo "2. Search demo: affiliate contract approval"
cargo run -q -p cortex-cli -- search --json "$DB_PATH" project:legal "affiliate contract approval"

echo ""
echo "3. ContextPack demo with citation requirement"
cargo run -q -p cortex-cli -- context --format json "$DB_PATH" project:legal \
  'RETRIEVE CONTEXT FOR TASK "affiliate approval policy with citations" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

echo ""
echo "4. VERIFY supported policy"
cargo run -q -p cortex-cli -- verify --format json "$DB_PATH" project:legal \
  'VERIFY FACT "All affiliate contracts must be approved by legal department before signature" IN BRAIN default;'

echo ""
echo "5. VERIFY contradiction demo"
cargo run -q -p cortex-cli -- verify --format json "$DB_PATH" project:legal \
  'VERIFY FACT "Low-risk affiliate contracts below 50000 USD could be approved by procurement without legal department approval before signature" IN BRAIN default;'

echo ""
echo "6. Validate storage"
cargo run -q -p cortex-cli -- validate "$DB_PATH"

rm -rf "$DB_PATH"
echo "=== Legal Policy Demo Completed Successfully ==="
