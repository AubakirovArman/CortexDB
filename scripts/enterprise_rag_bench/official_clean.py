"""Utilities for oracle-free EnterpriseRAG-Bench runs.

The clean inference path may read only the user-visible question and the
retrieved document IDs. Gold labels remain available only to judge scripts.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any


ORACLE_FIELDS = {
    "answer_facts",
    "expected_doc_ids",
    "gold_answer",
    "question_type",
    "source_types",
}

RETRIEVAL_ALLOWED_FIELDS = {"answer", "document_ids", "question", "question_id"}


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def clean_question(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "question_id": str(row.get("question_id") or ""),
        "question": str(row.get("question") or ""),
    }


def clean_retrieval_row(row: dict[str, Any]) -> dict[str, Any]:
    doc_ids = []
    seen = set()
    for item in row.get("document_ids", []) or []:
        doc_id = str(item)
        if doc_id and doc_id not in seen:
            seen.add(doc_id)
            doc_ids.append(doc_id)
    return {
        "answer": str(row.get("answer") or ""),
        "document_ids": doc_ids,
        "question": str(row.get("question") or ""),
        "question_id": str(row.get("question_id") or ""),
    }


def clean_questions(rows: list[dict[str, Any]], limit: int | None = None) -> list[dict[str, Any]]:
    selected = rows[:limit] if limit is not None else rows
    cleaned = [clean_question(row) for row in selected]
    for index, row in enumerate(cleaned, 1):
        if not row["question_id"] or not row["question"]:
            raise ValueError(f"clean question row {index} missing question_id or question")
    return cleaned


def clean_retrieval(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    cleaned = [clean_retrieval_row(row) for row in rows]
    for index, row in enumerate(cleaned, 1):
        if not row["question_id"] or not row["question"]:
            raise ValueError(f"clean retrieval row {index} missing question_id or question")
    return cleaned


def assert_clean_retrieval(rows: list[dict[str, Any]]) -> None:
    for index, row in enumerate(rows, 1):
        keys = set(row)
        oracle = keys & ORACLE_FIELDS
        extra = keys - RETRIEVAL_ALLOWED_FIELDS
        if oracle or extra:
            raise ValueError(
                "official-clean retrieval row "
                f"{index} has forbidden fields: {sorted(oracle | extra)}"
            )


def read_env_file(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}
    values: dict[str, str] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def normalize_openai_base_url(value: str) -> str:
    base = value.strip().rstrip("/")
    for suffix in ("/chat/completions", "/embeddings"):
        if base.endswith(suffix):
            base = base[: -len(suffix)]
    return base


def write_temp_key(provider: str, key: str) -> Path:
    path = Path("/tmp") / f"cortexdb_enterprise_rag_{provider}_key.txt"
    path.write_text(key.strip() + "\n", encoding="utf-8")
    os.chmod(path, 0o600)
    return path


def model_profile(provider: str) -> dict[str, Any]:
    provider = provider.lower().strip()
    if provider == "gemma":
        env = read_env_file(Path("/mnt/hf_model_weights/arman/3bit/sites/CortexDB/.env"))
        return {
            "provider": provider,
            "api_key": env.get("VLLM_API_KEY", ""),
            "base_url": normalize_openai_base_url(env.get("VLLM_URL", "http://89.106.235.4:8000/v1")),
            "model": env.get("VLLM_MODEL", "google/gemma-4-31B-it"),
            "gemini_native": False,
            "omit_thinking_field": True,
        }
    if provider == "gemini":
        env = read_env_file(Path("/mnt/hf_model_weights/arman/3bit/.env"))
        return {
            "provider": provider,
            "api_key": env.get("GEMINI_API_KEY", ""),
            "base_url": "https://generativelanguage.googleapis.com/v1beta",
            "model": "gemini-3.5-flash",
            "gemini_native": True,
            "omit_thinking_field": True,
        }
    if provider == "deepseek":
        key_path = Path("/mnt/hf_model_weights/arman/3bit/.deepseek")
        return {
            "provider": provider,
            "api_key": key_path.read_text(encoding="utf-8").strip() if key_path.exists() else "",
            "base_url": "https://api.deepseek.com",
            "model": "deepseek-v4-flash",
            "gemini_native": False,
            "omit_thinking_field": False,
        }
    if provider == "openai":
        env = read_env_file(Path("/mnt/hf_model_weights/arman/3bit/.env"))
        return {
            "provider": provider,
            "api_key": env.get("OPENAI_API_KEY", "") or os.environ.get("OPENAI_API_KEY", ""),
            "base_url": "https://api.openai.com/v1",
            "model": os.environ.get("CORTEXDB_OPENAI_JUDGE_MODEL", "gpt-5.2"),
            "gemini_native": False,
            "omit_thinking_field": True,
            "openai_reasoning": True,
        }
    raise ValueError(
        f"unknown provider {provider!r}; expected gemma, gemini, deepseek, or openai"
    )


def require_profile_key(profile: dict[str, Any]) -> None:
    if not str(profile.get("api_key") or "").strip():
        raise ValueError(f"missing API key for provider {profile.get('provider')}")
