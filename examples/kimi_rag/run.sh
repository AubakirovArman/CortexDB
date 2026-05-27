#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

echo "🚀 [Kimi RAG App] Setting up isolated virtual environment (venv)..."
python3 -m venv "$DIR/venv"
source "$DIR/venv/bin/activate"

echo "📦 [Kimi RAG App] Installing dependencies..."
pip install -q fastapi uvicorn

# Build cortex-server if needed
echo "⚙️ [CortexDB] Ensuring CortexDB binary is compiled..."
cargo build -q --bin cortex-server

# Start CortexDB server if not running
if lsof -pi :8090 -t >/dev/null; then
    echo "💾 [CortexDB] Server is already running on port 8090!"
else
    echo "💾 [CortexDB] Starting server on port 8090..."
    mkdir -p "$DIR/cortex_data"
    "$REPO_ROOT/target/debug/cortex-server" "$DIR/cortex_data" 127.0.0.1:8090 &
    SERVER_PID=$!
    # Ensure background process is killed on exit
    trap 'kill $SERVER_PID' EXIT
    sleep 2
fi

echo "📊 [Kimi RAG App] Running data generation and multi-tenant ingestion..."
python3 "$DIR/generate_data.py"

echo "⚡ [Kimi RAG App] Launching Chatbot Assistant on http://127.0.0.1:8085..."
uvicorn app:app --host 127.0.0.1 --port 8085 --log-level info
