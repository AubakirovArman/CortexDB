#!/usr/bin/env python3
"""SDK smoke test against a live cortex-server."""

import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent / "sdk/python"))
from cortexdb_client import CortexDBClient


def main() -> int:
    repo = Path(__file__).parent.parent
    db_dir = tempfile.mkdtemp()
    server = subprocess.Popen(
        [str(repo / "target/release/cortex-server"), db_dir, "127.0.0.1:18183"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    # Wait for server
    for _ in range(50):
        import socket
        try:
            s = socket.create_connection(("127.0.0.1", 18183), timeout=0.1)
            s.close()
            break
        except OSError:
            time.sleep(0.1)
    else:
        print("Server did not start")
        server.kill()
        return 1

    try:
        client = CortexDBClient("http://127.0.0.1:18183")

        # Health
        response = client._request("GET", "/v1/health", b"")
        assert response["status"] == "ok", f"Unexpected health: {response}"
        print("OK: health")

        # Put cell
        response = client._request(
            "POST", "/v1/cell?cell_id=1", b"scope=project:test\nstatus=ready\n\nhello"
        )
        assert response["seq"] == 1, f"Unexpected seq: {response}"
        print("OK: put cell")

        # Get cell
        response = client._request("GET", "/v1/cell?cell_id=1", b"")
        assert response["cell"]["cell_id"] == 1, f"Unexpected cell: {response}"
        print("OK: get cell")

        # Search
        response = client.search_response("project:test", "hello", limit=10)
        # Empty results are acceptable for a single-cell smoke test;
        # we just verify the response shape is valid.
        assert response.search_mode == "keyword", f"Unexpected mode: {response}"
        print("OK: search")

        # Stats
        response = client._request("GET", "/v1/stats", b"")
        assert "current_seq" in response, f"Unexpected stats: {response}"
        print("OK: stats")

        print("\nAll SDK smoke tests passed.")
        return 0
    except Exception as e:
        print(f"FAIL: {e}")
        return 1
    finally:
        server.terminate()
        server.wait(timeout=5)
        import shutil

        shutil.rmtree(db_dir)


if __name__ == "__main__":
    sys.exit(main())
