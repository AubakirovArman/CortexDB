#!/usr/bin/env python3
"""Read one text from stdin and print one embedding vector as JSON."""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import threading
import unittest
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any


DEFAULT_URL_ENV = "CORTEXDB_EMBEDDING_URL"
DEFAULT_MODEL_ENV = "CORTEXDB_EMBEDDING_MODEL"
DEFAULT_KEY_ENV = "CORTEXDB_EMBEDDING_API_KEY"


def numeric_vector(value: Any, label: str) -> list[float]:
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected a non-empty vector array")
    output = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            raise ValueError(f"{label}: vector values must be numeric")
        if not math.isfinite(float(item)):
            raise ValueError(f"{label}: vector values must be finite")
        output.append(float(item))
    return output


def extract_embedding(response: dict) -> list[float]:
    data = response.get("data")
    if isinstance(data, list) and data:
        first = data[0]
        if isinstance(first, dict) and "embedding" in first:
            return numeric_vector(first["embedding"], "data[0].embedding")
    for key in ("embedding", "vector"):
        if key in response:
            return numeric_vector(response[key], key)
    embeddings = response.get("embeddings")
    if isinstance(embeddings, list) and embeddings:
        return numeric_vector(embeddings[0], "embeddings[0]")
    raise ValueError("embedding response did not contain a supported vector field")


def request_embedding(args: argparse.Namespace, text: str) -> list[float]:
    if not text.strip():
        raise ValueError("stdin text is empty")
    url = args.url or os.environ.get(args.url_env)
    if not url:
        raise ValueError(f"--url or {args.url_env} is required")
    model = args.model or os.environ.get(args.model_env)
    if args.require_model and not model:
        raise ValueError(f"--model or {args.model_env} is required")
    payload: dict[str, Any] = {"input": text}
    if model:
        payload["model"] = model
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    headers = {"Content-Type": "application/json"}
    api_key = os.environ.get(args.api_key_env) if args.api_key_env else None
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"
    request = urllib.request.Request(url, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=args.timeout_seconds) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")[:500]
        raise ValueError(f"embedding endpoint returned HTTP {error.code}: {detail}") from error
    except urllib.error.URLError as error:
        raise ValueError(f"embedding endpoint request failed: {error.reason}") from error
    try:
        decoded = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ValueError("embedding endpoint returned invalid JSON") from error
    if not isinstance(decoded, dict):
        raise ValueError("embedding endpoint response must be a JSON object")
    vector = extract_embedding(decoded)
    if args.dimension is not None and len(vector) != args.dimension:
        raise ValueError(f"embedding dimension {len(vector)}, expected {args.dimension}")
    return vector


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url")
    parser.add_argument("--url-env", default=DEFAULT_URL_ENV)
    parser.add_argument("--model")
    parser.add_argument("--model-env", default=DEFAULT_MODEL_ENV)
    parser.add_argument("--api-key-env", default=DEFAULT_KEY_ENV)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--dimension", type=int)
    parser.add_argument("--require-model", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be greater than zero")
    if args.dimension is not None and args.dimension <= 0:
        parser.error("--dimension must be greater than zero")
    return args


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    vector = request_embedding(parse_args(argv), sys.stdin.read())
    sys.stdout.write(json.dumps(vector, separators=(",", ":")) + "\n")
    return 0


class FakeEmbeddingHandler(BaseHTTPRequestHandler):
    seen_authorization = False

    def do_POST(self) -> None:
        raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        body = json.loads(raw.decode("utf-8"))
        FakeEmbeddingHandler.seen_authorization = (
            self.headers.get("Authorization") == "Bearer test-key"
        )
        text = body.get("input", "")
        vector = [1.0, 0.0, 0.5] if "alpha" in text else [0.0, 1.0, 0.25]
        response = json.dumps({"data": [{"embedding": vector}]}).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, _format: str, *_args: object) -> None:
        return


class SelfTests(unittest.TestCase):
    def test_extract_embedding_shapes(self) -> None:
        self.assertEqual(extract_embedding({"embedding": [1, 2]}), [1.0, 2.0])
        self.assertEqual(extract_embedding({"vector": [3]}), [3.0])
        self.assertEqual(extract_embedding({"embeddings": [[4]]}), [4.0])
        self.assertEqual(extract_embedding({"data": [{"embedding": [5]}]}), [5.0])
        with self.assertRaises(ValueError):
            extract_embedding({"embedding": [True]})
        with self.assertRaises(ValueError):
            extract_embedding({"embedding": [float("nan")]})

    def test_http_embedding_command(self) -> None:
        server = ThreadingHTTPServer(("127.0.0.1", 0), FakeEmbeddingHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        old_key = os.environ.get(DEFAULT_KEY_ENV)
        os.environ[DEFAULT_KEY_ENV] = "test-key"
        try:
            args = parse_args([
                "--url",
                f"http://127.0.0.1:{server.server_port}/embeddings",
                "--model",
                "test-model",
                "--dimension",
                "3",
            ])
            self.assertEqual(request_embedding(args, "alpha"), [1.0, 0.0, 0.5])
            self.assertTrue(FakeEmbeddingHandler.seen_authorization)
        finally:
            server.shutdown()
            server.server_close()
            if old_key is None:
                os.environ.pop(DEFAULT_KEY_ENV, None)
            else:
                os.environ[DEFAULT_KEY_ENV] = old_key


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
