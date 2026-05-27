#!/usr/bin/env bash
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DIR/../.." && pwd)"

echo "🚀 [CortexDB RAG Demo] Starting setup..."

# Create venv
if [[ ! -d "$DIR/venv" ]]; then
    echo "📦 Creating virtual environment..."
    python3 -m venv "$DIR/venv"
fi
source "$DIR/venv/bin/activate"
pip install -q -r "$DIR/requirements.txt"

# Build cortex-server
echo "⚙️ Building cortex-server..."
cd "$REPO_ROOT" && cargo build -q --bin cortex-server

# Start cortex-server if not running
CORTEX_PID=""
if lsof -pi :8090 -t >/dev/null 2>&1; then
    echo "💾 CortexDB server already running on port 8090"
else
    echo "💾 Starting CortexDB server on port 8090..."
    mkdir -p "$DIR/cortex_db"
    "$REPO_ROOT/target/debug/cortex-server" "$DIR/cortex_db" "127.0.0.1:8090" &
    CORTEX_PID=$!
    sleep 2
fi

cleanup() {
    if [[ -n "$CORTEX_PID" ]]; then
        kill "$CORTEX_PID" 2>/dev/null || true
        wait "$CORTEX_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

# Ingest data
echo "📊 Ingesting Russian dummy data into CortexDB..."
python3 "$DIR/ingest.py"

# Check vLLM
echo "🔌 Checking vLLM at http://127.0.0.1:8018 ..."
if curl -sf http://127.0.0.1:8018/v1/models >/dev/null 2>&1; then
    echo "✅ vLLM is available on port 8018"
else
    echo "⚠️ vLLM not detected at port 8018. The chatbot will show LLM errors but CortexDB retrieval will still work."
    echo "   To start vLLM with Gemma-4, run something like:"
    echo "   vllm serve google/gemma-4-31B-it --port 8018"
fi

# Start FastAPI
echo "⚡ Starting RAG chatbot on http://127.0.0.1:8085"
cd "$DIR"
exec uvicorn app:app --host 127.0.0.1 --port 8085 --log-level info
