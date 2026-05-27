#!/usr/bin/env bash
set -euo pipefail

PORT=18182
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

for i in {1..30}; do
    if nc -z 127.0.0.1 "$PORT"; then
        break
    fi
    sleep 0.1
done

export CORTEX_PORT="$PORT"
echo "Running Playwright tests..."
npx playwright test --config=playwright.config.ts "$@"
