#!/bin/bash
set -e

echo "=== CortexDB Alpha Demo: Search + AQL + ContextPack + Verify ==="
echo ""
echo "This demo shows why CortexDB is stronger than plain RAG:"
echo "  - permission-safe scope filtering"
echo "  - ContextPack with token budgets, citations, and anomalies"
echo "  - VERIFY FACT detects numeric conflicts before they reach your agent"
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
value=1200000000
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

cargo run -q -p cortex-cli -- put "$DB_PATH" 3 "scope=private
status=ready
type=fact
source=internal_notes.txt

Private risk note: Solar Plant faces regulatory delays in Q3."

cargo run -q -p cortex-cli -- put "$DB_PATH" 4 "scope=project:investments
status=ready
type=document_block
source=board_minutes.pdf

Board approved Solar Plant expansion with revised timeline."

echo ""
echo "2. Reading cell with ID 1..."
cargo run -q -p cortex-cli -- get "$DB_PATH" 1

echo ""
echo "3. Flushing MemTable to create stable persistent segments..."
cargo run -q -p cortex-cli -- flush "$DB_PATH"

echo ""
echo "4. Checking storage stats..."
cargo run -q -p cortex-cli -- stats "$DB_PATH"

echo ""
echo "5. Plain keyword search for 'Solar' in scope 'project:investments'..."
echo "   (This returns raw matching cells — like a typical RAG retriever)"
cargo run -q -p cortex-cli -- search "$DB_PATH" project:investments Solar

echo ""
echo "6. AQL retrieve — permission-safe, scoped, policy-bound..."
echo "   (Notice: 'private' scope cell does NOT appear)"
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments \
  "RETRIEVE CONTEXT FOR TASK \"What is the Solar Plant budget?\" IN BRAIN default WHERE space = project:investments LIMIT 10 CANDIDATES;"

echo ""
echo "7. ContextPack with --json output..."
echo "   (Shows token_budget, estimated_tokens, citations, anomalies)"
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments \
  "RETRIEVE CONTEXT FOR TASK \"What is the Solar Plant budget?\" IN BRAIN default WHERE space = project:investments LIMIT 10 CANDIDATES;" --json

echo ""
echo "8. VERIFY FACT — 'Solar Plant budget is 1.2B KZT'..."
echo "   (CortexDB detects the Q1 vs Q2 numeric conflict)"
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments \
  "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN default;"

echo ""
echo "9. VERIFY FACT with --json output..."
echo "   (Shows supporting evidence, contradicting evidence, numeric_conflicts)"
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments \
  "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN default;" --json

echo ""
echo "10. Validating the storage files integrity..."
cargo run -q -p cortex-cli -- validate "$DB_PATH"

echo ""
echo "=== Comparison: Plain RAG vs CortexDB ==="
echo ""
echo "  Plain RAG:              CortexDB:"
echo "  - top-k chunks          - ContextPack with token budgets"
echo "  - no conflict detection  - VERIFY FACT with numeric guards"
echo "  - no permission mask     - scope-isolated AgentView"
echo "  - no citation policy     - structured SourceRef + citations"
echo "  - no anomaly reports     - anomaly report per pack"
echo ""

echo "=== Clean up temp db ==="
rm -rf "$DB_PATH"
echo "=== CortexDB Demo Completed Successfully ==="
