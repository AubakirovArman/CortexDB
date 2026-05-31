#!/usr/bin/env python3
"""Smoke test the standalone dashboard static build over HTTP."""

from __future__ import annotations

import contextlib
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.request import urlopen


ROOT = Path(__file__).resolve().parents[1]
DIST_DIR = ROOT / "web" / "dashboard" / "dist"


class QuietHandler(SimpleHTTPRequestHandler):
    def log_message(self, _format: str, *_args: object) -> None:
        return


@contextlib.contextmanager
def static_server():
    handler = lambda *args, **kwargs: QuietHandler(  # noqa: E731
        *args,
        directory=str(DIST_DIR),
        **kwargs,
    )
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        host, port = server.server_address
        yield f"http://{host}:{port}"
    finally:
        server.shutdown()
        thread.join(timeout=5)
        server.server_close()


def fetch_text(url: str) -> str:
    with urlopen(url, timeout=5) as response:
        if response.status != 200:
            raise RuntimeError(f"{url} returned {response.status}")
        return response.read().decode("utf-8")


def main() -> int:
    if not (DIST_DIR / "index.html").is_file():
        raise SystemExit("missing web/dashboard/dist/index.html; run make dashboard-build")
    with static_server() as base:
        index = fetch_text(f"{base}/")
        route_index = fetch_text(f"{base}/dashboard/search/")
        style = fetch_text(f"{base}/dashboard/assets/v1/style.css")
        reporting = fetch_text(f"{base}/dashboard/assets/v1/reporting.js")
        script = fetch_text(f"{base}/dashboard/assets/v1/app.js")
        manifest = fetch_text(f"{base}/dashboard/assets/v1/dashboard_manifest.json")
    required = [
        ("index title", "CortexDB Console" in index),
        ("stylesheet link", "/dashboard/assets/v1/style.css" in index),
        ("script link", "/dashboard/assets/v1/app.js" in index),
        ("reporting script link", "/dashboard/assets/v1/reporting.js" in index),
        ("route link", 'href="/dashboard/search"' in index),
        ("route page", "CortexDB Console" in route_index),
        ("panel css", ".panel.active" in style),
        ("route css", '.tab[aria-current="page"]' in style),
        ("stats bootstrap", 'run("stats"' in script),
        ("ann report renderer", "renderAnnEvaluation" in reporting),
        ("context report renderer", "renderContextPack" in reporting),
        ("history router", "pushState" in script),
        ("frontend stack manifest", "dependency-free-static-html-css-js" in manifest),
        ("memory-only token policy", '"token_persistence": "memory-only"' in manifest),
        ("route manifest", '"ann-eval"' in manifest),
    ]
    failed = [name for name, ok in required if not ok]
    if failed:
        raise SystemExit("standalone dashboard smoke failed: " + ", ".join(failed))
    print("OK: standalone dashboard dist serves expected assets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
