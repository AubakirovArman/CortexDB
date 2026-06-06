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


def main() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
    return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
