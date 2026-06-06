#!/usr/bin/env python3
"""Self-tests for embedding_provider.py."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

from embedding_provider import (
    DEFAULT_KEY_ENV,
    EmbeddingProviderConfig,
    embed_text,
    extract_embedding,
    load_embedding_file,
    provider_profile,
    text_sha256,
    validate_provider_config,
)


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

    def test_file_embedding_provider_uses_text_and_sha_keys(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            path = Path(raw_dir) / "embeddings.jsonl"
            path.write_text(
                json.dumps({"text": "alpha", "embedding": [1, 2, 3]}) + "\n",
                encoding="utf-8",
            )
            config = EmbeddingProviderConfig(provider="file", embedding_file=path, dimension=3)
            self.assertEqual(embed_text(config, "alpha"), [1.0, 2.0, 3.0])
            self.assertEqual(load_embedding_file(path)[text_sha256("alpha")], [1.0, 2.0, 3.0])

    def test_command_provider_rejects_hash_smoke_command(self) -> None:
        with self.assertRaises(ValueError):
            validate_provider_config(EmbeddingProviderConfig(provider="command", command="hash-smoke"))

    def test_hash_smoke_provider_is_deterministic(self) -> None:
        config = EmbeddingProviderConfig(provider="hash-smoke", hash_dimension=8)
        self.assertEqual(embed_text(config, "alpha beta"), embed_text(config, "alpha beta"))
        self.assertEqual(len(embed_text(config, "alpha beta")), 8)

    def test_provider_profile_hides_secret_value(self) -> None:
        old_value = os.environ.get(DEFAULT_KEY_ENV)
        os.environ[DEFAULT_KEY_ENV] = "secret-value"
        try:
            profile = provider_profile(
                EmbeddingProviderConfig(
                    provider="openai-compatible",
                    url="https://example.test/v1/embeddings",
                    model="model-a",
                )
            )
            self.assertEqual(profile["endpoint_origin"], "https://example.test")
            self.assertEqual(profile["api_key_env"], DEFAULT_KEY_ENV)
            self.assertTrue(profile["api_key_present"])
            self.assertNotIn("secret-value", json.dumps(profile))
        finally:
            if old_value is None:
                os.environ.pop(DEFAULT_KEY_ENV, None)
            else:
                os.environ[DEFAULT_KEY_ENV] = old_value

    def test_cache_reuses_vector_and_invalidates_model_and_dimension(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            counter = root / "count.txt"
            counter.write_text("0", encoding="utf-8")
            helper = root / "embed.py"
            helper.write_text(
                "\n".join([
                    "import json, os, pathlib, sys",
                    "counter = pathlib.Path(os.environ['COUNT_FILE'])",
                    "counter.write_text(str(int(counter.read_text()) + 1))",
                    "dim = int(os.environ.get('DIM', '2'))",
                    "print(json.dumps([float(i) for i in range(1, dim + 1)]))",
                ])
                + "\n",
                encoding="utf-8",
            )
            old_count = os.environ.get("COUNT_FILE")
            old_dim = os.environ.get("DIM")
            os.environ["COUNT_FILE"] = str(counter)
            os.environ["DIM"] = "2"
            try:
                base = {
                    "provider": "command",
                    "command": f"{sys.executable} {helper}",
                    "cache_file": root / "embeddings.cache.jsonl",
                }
                first = EmbeddingProviderConfig(**base, model="model-a", dimension=2)
                self.assertEqual(embed_text(first, "alpha"), [1.0, 2.0])
                self.assertEqual(embed_text(first, "alpha"), [1.0, 2.0])
                self.assertEqual(counter.read_text(encoding="utf-8"), "1")

                self.assertEqual(embed_text(EmbeddingProviderConfig(**base, model="model-b", dimension=2), "alpha"), [1.0, 2.0])
                self.assertEqual(counter.read_text(encoding="utf-8"), "2")

                os.environ["DIM"] = "3"
                self.assertEqual(embed_text(EmbeddingProviderConfig(**base, model="model-b", dimension=3), "alpha"), [1.0, 2.0, 3.0])
                self.assertEqual(counter.read_text(encoding="utf-8"), "3")
            finally:
                restore_env("COUNT_FILE", old_count)
                restore_env("DIM", old_dim)


def restore_env(name: str, value: str | None) -> None:
    if value is None:
        os.environ.pop(name, None)
    else:
        os.environ[name] = value


def main() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
    return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
