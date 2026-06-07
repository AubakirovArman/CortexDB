#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT_DIR"

DB_PATH="./examples/demo/support_tickets/db_temp"
rm -rf "$DB_PATH"

echo "=== CortexDB Support Ticket Demo ==="
echo "1. Loading support ticket fixture..."
cargo run -q -p cortex-cli -- load-fixture "$DB_PATH" examples/datasets/support_tickets

echo ""
echo "2. Customer issue retrieval"
cargo run -q -p cortex-cli -- search --json "$DB_PATH" support:tickets \
  "authentication outage signing key drift"

echo ""
echo "3. ContextPack for repeated authentication incidents"
cargo run -q -p cortex-cli -- context --format json "$DB_PATH" support:tickets \
  'RETRIEVE CONTEXT FOR TASK "Find repeated authentication incidents and successful remediation steps" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

echo ""
echo "4. Memory update"
cargo run -q -p cortex-cli -- --json remember "$DB_PATH" support:tickets \
  'REMEMBER "For repeated authentication failures, check token issuer drift before cache invalidation" IN SCOPE support:tickets AS TYPE workflow_result TTL 1209600 SECONDS;'

echo ""
echo "5. Resolution verification"
cargo run -q -p cortex-cli -- verify --format json "$DB_PATH" support:tickets \
  'VERIFY FACT "The authentication outage was mitigated by rotating the signing key" IN BRAIN default;'

echo ""
echo "6. Validate storage"
cargo run -q -p cortex-cli -- validate "$DB_PATH"

rm -rf "$DB_PATH"
echo "=== Support Ticket Demo Completed Successfully ==="
