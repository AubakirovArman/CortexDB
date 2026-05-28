#!/usr/bin/env python3
"""SDK smoke test against a live cortex-server.

Validates typed response models for Python SDK by issuing real requests.
"""

import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "sdk/python"))
from cortexdb_client import CortexDBClient


def wait_for_server(port: int, timeout: float = 10.0) -> bool:
    import socket
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            s = socket.create_connection(("127.0.0.1", port), timeout=0.1)
            s.close()
            return True
        except OSError:
            time.sleep(0.1)
    return False


def main() -> int:
    repo = Path(__file__).parent.parent
    db_dir = tempfile.mkdtemp()
    port = 18183
    # Prefer release build; fall back to debug
    binary = repo / "target/release/cortex-server"
    if not binary.exists():
        binary = repo / "target/debug/cortex-server"
    if not binary.exists():
        print("ERROR: cortex-server binary not found. Run 'cargo build -p cortex-server' first.")
        return 1

    server = subprocess.Popen(
        [str(binary), db_dir, f"127.0.0.1:{port}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    if not wait_for_server(port):
        print("Server did not start")
        server.kill()
        return 1

    try:
        client = CortexDBClient(f"http://127.0.0.1:{port}")

        # Health typed
        health = client.health_response()
        assert health.status == "ok"
        assert health.version == "v1"
        assert health.server_version
        print("OK: health_response")

        # Put cell typed
        put = client.put_cell_response(1, "scope=default\nstatus=ready\ntype=fact\nsource=smoke\n\nhello world")
        assert put.seq == 1
        assert put.cell_id == 1
        print("OK: put_cell_response")

        # Get cell typed
        lookup = client.get_cell_response(1)
        assert lookup.cell is not None
        assert lookup.cell.cell_id == 1
        print("OK: get_cell_response")

        # Search typed
        search = client.search_response("default", "hello", limit=10)
        assert search.search_mode == "keyword"
        print("OK: search_response")

        # Stats typed
        stats = client.stats_response()
        assert stats.current_seq >= 1
        print("OK: stats_response")

        # Validate typed
        validation = client.validate_response()
        assert validation.ok is True
        print("OK: validate_response")

        # AQL typed
        aql = client.aql_response("default", 'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;')
        assert isinstance(aql.cells, tuple)
        print("OK: aql_response")

        # Context typed
        context = client.context_response("default", 'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;')
        assert context.token_budget_tokens >= 0
        print("OK: context_response")

        # Verify typed
        verify = client.verify_response("default", 'VERIFY FACT "hello world" IN BRAIN default;')
        assert verify.fact == "hello world"
        print("OK: verify_response")

        # Remember typed
        remember = client.remember_response("default", 'REMEMBER "test memory" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;')
        assert remember.seq > 0
        print("OK: remember_response")

        # Ingest text typed
        ingest = client.ingest_text_response("default", "hello world ingestion")
        assert ingest.chunks_ingested >= 1
        print("OK: ingest_text_response")

        print("\nAll SDK smoke tests passed.")
        return 0
    except Exception as e:
        print(f"FAIL: {e}")
        import traceback

        traceback.print_exc()
        return 1
    finally:
        server.terminate()
        try:
            server.wait(timeout=5)
        except Exception:
            server.kill()
        shutil.rmtree(db_dir, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())
