#!/usr/bin/env bash
set -euo pipefail

# Live server smoke test for Core Alpha API contract.
# Requires: cargo, curl, nc

PORT=18181
DB_DIR=$(mktemp -d)
SERVER_PID=""

cleanup() {
    if [[ -n "$SERVER_PID" ]]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -rf "$DB_DIR"
}
trap cleanup EXIT

echo "Building cortex-server..."
cargo build -p cortex-server --bin cortex-server --release >/dev/null 2>&1

echo "Starting server on port $PORT..."
./target/release/cortex-server "$DB_DIR" "127.0.0.1:$PORT" &
SERVER_PID=$!

# Wait for server to be ready
for i in {1..30}; do
    if nc -z 127.0.0.1 "$PORT"; then
        break
    fi
    sleep 0.1
done

BASE="http://127.0.0.1:$PORT"

check_json() {
    local method=${3:-GET}
    local url=$1
    local expected=$2
    local response
    if [[ "$method" == "POST" ]]; then
        response=$(curl -sf -X POST "$url" || true)
    else
        response=$(curl -sf "$url" || true)
    fi
    if [[ -z "$response" ]]; then
        echo "FAIL: $method $url — no response"
        exit 1
    fi
    if echo "$response" | grep -q "$expected"; then
        echo "OK: $method $url"
    else
        echo "FAIL: $method $url — expected '$expected' in response"
        echo "Got: $response"
        exit 1
    fi
}

echo "Running smoke tests..."

check_json "$BASE/v1/health" '"status":"ok"'
check_json "$BASE/v1/stats" '"current_seq"'
check_json "$BASE/v1/validate" '"ok":true'

curl -sf -X POST "$BASE/v1/cell?cell_id=1" -d "scope=project:test\nstatus=ready\n\nhello world" >/dev/null
check_json "$BASE/v1/cell?cell_id=1" '"cell_id":1'
check_json "$BASE/v1/cell?cell_id=999" '"cell":null'

check_json "$BASE/v1/flush" '"checkpoint_seq"' POST
check_json "$BASE/v1/compact" '"checkpoint_seq"' POST

curl -sf -X POST "$BASE/v1/ingest/text?scope=default" -d "hello world" >/dev/null
check_json "$BASE/v1/stats" '"memtable_cells"'

echo ""
echo "All smoke tests passed."
