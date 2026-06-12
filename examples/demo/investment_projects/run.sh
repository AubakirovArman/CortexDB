#!/bin/bash
set -e

if [ -t 1 ]; then
  BOLD="\033[1m"
  CYAN="\033[36m"
  GREEN="\033[32m"
  YELLOW="\033[33m"
  RED="\033[31m"
  RESET="\033[0m"
else
  BOLD=""
  CYAN=""
  GREEN=""
  YELLOW=""
  RED=""
  RESET=""
fi

section() {
  printf "\n${BOLD}${CYAN}%s${RESET}\n" "$1"
}

success() {
  printf "${GREEN}%s${RESET}\n" "$1"
}

warn() {
  printf "${YELLOW}%s${RESET}\n" "$1"
}

echo "=== CortexDB Flagship Demo: Permissions + ContextPack + Verify ==="
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

section "1. Putting finance and private fact cells into the database..."
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

section "2. Reading cell with ID 1..."
cargo run -q -p cortex-cli -- get "$DB_PATH" 1

section "3. Flushing MemTable to create stable persistent segments..."
cargo run -q -p cortex-cli -- flush "$DB_PATH"

section "4. Checking storage stats..."
cargo run -q -p cortex-cli -- stats "$DB_PATH"

section "5. Finance agent: keyword search in scope 'project:investments'..."
echo "   (This returns raw matching cells — like a typical RAG retriever)"
cargo run -q -p cortex-cli -- search "$DB_PATH" project:investments Solar

section "6. Finance agent: AQL retrieve builds a scoped ContextPack..."
echo "   (Notice: 'private' scope cell does NOT appear)"
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments \
  "RETRIEVE CONTEXT FOR TASK \"What is the Solar Plant budget?\" IN BRAIN default WHERE space = project:investments LIMIT 10 CANDIDATES;"

section "7. HR agent: the same investment scope is denied before retrieval..."
set +e
DENIED_OUTPUT=$(cargo run -q -p cortex-cli -- context "$DB_PATH" agent:hr \
  "RETRIEVE CONTEXT FOR TASK \"What is the Solar Plant budget?\" IN BRAIN default WHERE space = project:investments LIMIT 10 CANDIDATES;" 2>&1)
DENIED_STATUS=$?
set -e
if [ "$DENIED_STATUS" -eq 0 ]; then
  printf "${RED}expected HR agent denial, but command succeeded${RESET}\n"
  exit 1
fi
echo "$DENIED_OUTPUT"
if echo "$DENIED_OUTPUT" | grep -q "ScopeNotReadable"; then
  success "HR agent denied as expected: ScopeNotReadable"
else
  printf "${RED}expected ScopeNotReadable denial${RESET}\n"
  exit 1
fi

section "8. ContextPack with --json output..."
echo "   (Shows token_budget, estimated_tokens, citations, anomalies)"
cargo run -q -p cortex-cli -- context "$DB_PATH" project:investments \
  "RETRIEVE CONTEXT FOR TASK \"What is the Solar Plant budget?\" IN BRAIN default WHERE space = project:investments LIMIT 10 CANDIDATES;" --json

section "9. VERIFY FACT — 'Solar Plant budget is 1.2B KZT'..."
echo "   (CortexDB detects the Q1 vs Q2 numeric conflict)"
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments \
  "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN default;"

section "10. VERIFY FACT with --json output..."
echo "   (Shows supporting evidence, contradicting evidence, numeric_conflicts)"
cargo run -q -p cortex-cli -- verify "$DB_PATH" project:investments \
  "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN default;" --json

section "11. Validating the storage files integrity..."
cargo run -q -p cortex-cli -- validate "$DB_PATH"

section "=== Comparison: Plain RAG vs CortexDB ==="
echo ""
echo "  Plain RAG:              CortexDB:"
echo "  - top-k chunks          - ContextPack with token budgets"
echo "  - no conflict detection  - VERIFY FACT with numeric guards"
echo "  - no permission mask     - scope-isolated AgentView"
echo "  - no citation policy     - structured SourceRef + citations"
echo "  - no anomaly reports     - anomaly report per pack"
echo ""

warn "=== Clean up temp db ==="
rm -rf "$DB_PATH"
success "=== CortexDB Demo Completed Successfully ==="
