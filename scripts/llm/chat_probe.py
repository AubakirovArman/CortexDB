#!/usr/bin/env python3
"""Chat-endpoint probe for the A6 answer-half harness.

Reads an OpenAI-compatible chat endpoint from the environment (mirroring the
existing CORTEXDB_EMBEDDING_* convention) and reports whether it can complete a
one-token request — WITHOUT printing the key or the model's reply. This is the
handshake that unblocks A6.1 (Gemma/other answerer), A6.2 (judge integration),
and the ERB/LME answer halves: once it prints WORKS, the harness has a usable
client.

Environment:
  CORTEXDB_CHAT_URL      base URL, e.g. https://api.deepseek.com  (or /v1 root)
  CORTEXDB_CHAT_MODEL    model id, e.g. deepseek-chat, gpt-4o-mini, gemma-2-9b-it
  CORTEXDB_CHAT_API_KEY  bearer token (keep in a gitignored file / .env)

Exit 0 = WORKS, 1 = unusable (prints the coarse failure class only).
Stdlib only; no third-party deps.
"""

from __future__ import annotations

import json
import os
import sys
import urllib.error
import urllib.request


def endpoint() -> str:
    base = os.environ.get("CORTEXDB_CHAT_URL", "").rstrip("/")
    if not base:
        return ""
    # Accept either a bare host or one already ending in /v1 or the full path.
    if base.endswith("/chat/completions"):
        return base
    if base.endswith("/v1"):
        return base + "/chat/completions"
    return base + "/v1/chat/completions"


def main() -> int:
    url = endpoint()
    model = os.environ.get("CORTEXDB_CHAT_MODEL", "")
    key = os.environ.get("CORTEXDB_CHAT_API_KEY", "")
    missing = [
        n for n, v in [
            ("CORTEXDB_CHAT_URL", url),
            ("CORTEXDB_CHAT_MODEL", model),
            ("CORTEXDB_CHAT_API_KEY", key),
        ] if not v
    ]
    if missing:
        print(f"chat-probe: UNUSABLE — unset: {', '.join(missing)}")
        return 1

    payload = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": "reply with the single word OK"}],
        "max_tokens": 5,
        "stream": False,
        "temperature": 0,
    }).encode()
    req = urllib.request.Request(
        url, data=payload, method="POST",
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            body = json.loads(resp.read())
        choices = body.get("choices") or []
        if choices and "message" in choices[0]:
            print(f"chat-probe: WORKS — model={model} responded (choices present)")
            return 0
        print(f"chat-probe: UNUSABLE — 200 but no chat choice (keys={list(body.keys())[:6]})")
        return 1
    except urllib.error.HTTPError as e:
        cls = "unknown"
        try:
            err = json.loads(e.read())
            cls = (err.get("error") or {}).get("type") or (err.get("error") or {}).get("code") or "unknown"
        except Exception:  # noqa: BLE001
            pass
        print(f"chat-probe: UNUSABLE — HTTP {e.code} ({cls})")
        return 1
    except Exception as e:  # noqa: BLE001
        print(f"chat-probe: UNUSABLE — {type(e).__name__}: {e}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
