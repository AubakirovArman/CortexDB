#!/bin/bash
set -e

echo "=== Starting CortexDB Alpha Demo ==="
echo ""

# Ensure we are in the workspace root or sites/CortexDB
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
cd "$ROOT_DIR"

DB_PATH="./examples/demo/investment_projects/db_temp"
rm -rf "$DB_PATH"

echo "1. Putting fact cells into the database..."
cargo run -q -p cortex-cli -- put "$DB_PATH" 1 "scope=project:investments
status=ready
type=fact
source=report_q1.pdf#page=3
project=Solar Plant
metric=budget
value=1.2
currency=KZT

Solar Plant report highlights. The total approved budget for the Solar Plant project in first quarter is 1.2B KZT."

cargo run -q -p cortex-cli -- put "$DB_PATH" 2 "scope=project:investments
status=ready
type=fact
source=report_q2.pdf#page=5
project=Solar Plant
metric=budget
value=1400000000
currency=KZT

Solar Plant Q2 update. Following recent expansions, the budget for Solar Plant has been adjusted to 1.4B KZT."

cargo run -q -p cortex-cli -- put "$DB_PATH" 3 "scope=project:legal
status=ready
type=fact
source=legal_clearance.pdf#page=1

All projects have successfully cleared environmental and legal audits."

echo ""
echo "2. Reading cell with ID 1..."
cargo run -q -p cortex-cli -- get "$DB_PATH" 1

echo ""
echo "3. Flushing MemTable to create stable persistent segments..."
cargo run -q -p cortex-cli -- flush "$DB_PATH"

echo ""
echo "4. Checking storage stats after restart check..."
cargo run -q -p cortex-cli -- stats "$DB_PATH"

echo ""
echo "5. Searching for keyword 'Solar' in scope 'project:investments'..."
cargo run -q -p cortex-cli -- search "$DB_PATH" project:investments Solar

echo ""
echo "6. Retrieving Context Pack for Solar Plant query..."
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;"

echo ""
echo "7. Retrieving Context Pack with --json output..."
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;" --json

echo ""
echo "8. Running verification of fact 'Solar Plant budget is 1.2B KZT'..."
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"

echo ""
echo "9. Running verification with --json output..."
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;" --json

echo ""
echo "10. Validating the storage files integrity..."
cargo run -q -p cortex-cli -- validate "$DB_PATH"

echo ""
echo "=== Clean up temp db ==="
rm -rf "$DB_PATH"
echo "=== CortexDB Demo Completed Successfully ==="
