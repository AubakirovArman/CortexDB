#!/usr/bin/env python3
"""Read one text from stdin and print one embedding vector as JSON."""

from __future__ import annotations

import argparse
import json
import os
import sys
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

from embedding_provider import (
    DEFAULT_KEY_ENV,
    DEFAULT_MODEL_ENV,
    DEFAULT_URL_ENV,
    EmbeddingProviderConfig,
    embed_text,
    extract_embedding,
)


def request_embedding(args: argparse.Namespace, text: str) -> list[float]:
    return embed_text(
        EmbeddingProviderConfig(
            provider=args.provider,
            command=args.embedding_command or "",
            url=args.url or "",
            url_env=args.url_env,
            model=args.model or "",
            model_env=args.model_env,
            api_key_env=args.api_key_env,
            embedding_file=args.embedding_file,
            timeout_seconds=args.timeout_seconds,
            require_model=args.require_model,
            dimension=args.dimension,
            hash_dimension=args.hash_dimension,
        ),
        text,
        "stdin",
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--provider",
        choices=["openai-compatible", "local", "file", "command", "hash-smoke"],
        default="openai-compatible",
    )
    parser.add_argument("--url")
    parser.add_argument("--url-env", default=DEFAULT_URL_ENV)
    parser.add_argument("--model")
    parser.add_argument("--model-env", default=DEFAULT_MODEL_ENV)
    parser.add_argument("--api-key-env", default=DEFAULT_KEY_ENV)
    parser.add_argument("--embedding-command")
    parser.add_argument("--embedding-file", type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--dimension", type=int)
    parser.add_argument("--hash-dimension", type=int, default=64)
    parser.add_argument("--require-model", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return args
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be greater than zero")
    if args.dimension is not None and args.dimension <= 0:
        parser.error("--dimension must be greater than zero")
    if args.hash_dimension <= 0:
        parser.error("--hash-dimension must be greater than zero")
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
                "--provider",
                "openai-compatible",
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

    def test_file_embedding_provider(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as raw_dir:
            path = Path(raw_dir) / "embeddings.jsonl"
            path.write_text(
                json.dumps({"text": "alpha", "embedding": [1, 2, 3]}) + "\n",
                encoding="utf-8",
            )
            args = parse_args(["--provider", "file", "--embedding-file", str(path)])
            self.assertEqual(request_embedding(args, "alpha"), [1.0, 2.0, 3.0])


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
