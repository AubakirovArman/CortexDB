"""Feature extraction for EnterpriseRAG hybrid reranking."""

from __future__ import annotations

import json
import math
import re
from collections import Counter
from pathlib import Path
from typing import Any

from evidence_digest import evidence_digest_score
from rerank_with_embeddings import extract_document_content


STOPWORDS = {
    "about",
    "according",
    "after",
    "before",
    "during",
    "from",
    "handling",
    "including",
    "into",
    "what",
    "when",
    "where",
    "which",
    "while",
    "with",
}


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9_./:%-]+", " ", text.lower()).strip()


def tokens(text: str) -> list[str]:
    return [
        token
        for token in normalize(text).split()
        if len(token) > 1 and token not in STOPWORDS
    ]


def precise_anchors(question: str) -> list[str]:
    anchors: set[str] = set()
    patterns = [
        r"`([^`]+)`",
        r"\b[a-zA-Z0-9_./:-]+\.[a-zA-Z0-9_./:-]+\b",
        r"\b[A-Z]{2,}[A-Z0-9_-]*\b",
        r"\b[A-Z][a-z]+(?:[A-Z][a-z0-9]+)+\b",
        r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+){1,3}\b",
        r"\b[a-z]+-[a-z0-9-]+(?:-[a-z0-9]+)*\b",
        r"\bp(?:50|90|95|99)\b",
        r"\b\d+(?:\.\d+)?(?:%|ms|s|mib|gib|gb|mb|hours?|minutes?)?\b",
    ]
    for pattern in patterns:
        for match in re.findall(pattern, question):
            value = match if isinstance(match, str) else match[0]
            cleaned = normalize(value)
            if cleaned:
                anchors.add(cleaned)
    return sorted(anchors, key=lambda value: (-len(value), value))


def soft_phrases(question: str) -> list[str]:
    parts = [token for token in tokens(question) if len(token) > 2]
    phrases: set[str] = set()
    for width in (4, 3, 2):
        for index in range(0, max(0, len(parts) - width + 1)):
            phrase = " ".join(parts[index : index + width])
            if len(phrase) >= 12:
                phrases.add(phrase)
    return sorted(phrases, key=lambda value: (-len(value), value))


def source_type(path: str) -> str:
    return path.split("/", 1)[0] if path else "unknown"


def load_embedding_cache(path: Path | None) -> dict[str, list[float]]:
    values: dict[str, list[float]] = {}
    if path is None or not path.exists():
        return values
    for row in read_jsonl(path):
        key = row.get("key")
        vector = row.get("vector")
        if isinstance(key, str) and isinstance(vector, list):
            values[key] = [float(item) for item in vector]
    return values


def cosine(left: list[float] | None, right: list[float] | None) -> float:
    if not left or not right or len(left) != len(right):
        return 0.0
    dot = sum(a * b for a, b in zip(left, right))
    lnorm = math.sqrt(sum(a * a for a in left))
    rnorm = math.sqrt(sum(b * b for b in right))
    if lnorm == 0.0 or rnorm == 0.0:
        return 0.0
    return dot / (lnorm * rnorm)


class DocumentCache:
    def __init__(self, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.values: dict[str, dict[str, Any]] = {}

    def get(self, doc_id: str) -> dict[str, Any]:
        if doc_id in self.values:
            return self.values[doc_id]
        rel_path = self.uuid_index.get(doc_id, "")
        if not rel_path:
            value = self._value(doc_id, "", "", "")
            self.values[doc_id] = value
            return value
        document = read_json(self.sources_dir / rel_path)
        title, content = extract_document_content(document)
        value = self._value(doc_id, rel_path, title, content)
        self.values[doc_id] = value
        return value

    @staticmethod
    def _value(doc_id: str, rel_path: str, title: str, content: str) -> dict[str, Any]:
        combined = f"{rel_path}\n{title}\n{content}"
        doc_tokens = tokens(combined)
        return {
            "doc_id": doc_id,
            "rel_path": rel_path,
            "title": title,
            "content": content,
            "combined": combined,
            "normalized": normalize(combined),
            "tokens": doc_tokens,
            "token_counts": Counter(doc_tokens),
            "token_set": set(doc_tokens),
            "source_type": source_type(rel_path),
        }


def query_idf(question_tokens: list[str], docs: list[dict[str, Any]]) -> dict[str, float]:
    total = max(len(docs), 1)
    values: dict[str, float] = {}
    for token in set(question_tokens):
        df = sum(1 for doc in docs if token in doc["token_set"])
        values[token] = math.log((total + 1) / (df + 1)) + 1.0
    return values


def score_doc(
    *,
    question: dict[str, Any],
    doc: dict[str, Any],
    rank: int,
    embeddings: dict[str, list[float]],
    idf: dict[str, float],
    weights: dict[str, float],
) -> dict[str, float]:
    question_text = str(question.get("question", ""))
    q_tokens = [token for token in tokens(question_text) if token not in STOPWORDS]
    q_unique = set(q_tokens)
    token_set = doc["token_set"]
    token_counts = doc["token_counts"]
    weighted_overlap = sum(idf.get(token, 1.0) for token in q_unique & token_set)
    repeat_overlap = sum(min(token_counts[token], 3) for token in q_unique & token_set)
    coverage = len(q_unique & token_set) / max(len(q_unique), 1)
    anchors = precise_anchors(question_text)
    anchor_hits = sum(1 for anchor in anchors if anchor in doc["normalized"])
    anchor_ratio = anchor_hits / len(anchors) if anchors else 0.0
    phrases = soft_phrases(question_text)
    phrase_hits = sum(1 for phrase in phrases if phrase in doc["normalized"])
    title_hits = len(q_unique & set(tokens(str(doc["title"]))))
    path_hits = len(q_unique & set(tokens(str(doc["rel_path"]))))
    digest = evidence_digest_score(str(doc["content"]), question_text)
    source_boost = (
        1.0
        if doc["source_type"] in {str(item) for item in question.get("source_types", [])}
        else 0.0
    )
    embedding = cosine(
        embeddings.get(f"q:{question.get('question_id')}"),
        embeddings.get(f"d:{doc['doc_id']}"),
    )
    raw_rank = 1.0 / max(rank, 1)
    top20_rank = max(0.0, (21.0 - rank) / 20.0)
    score = (
        weighted_overlap * weights["weighted_overlap"]
        + repeat_overlap * weights["repeat_overlap"]
        + coverage * weights["coverage"]
        + anchor_hits * weights["anchor"]
        + anchor_ratio * weights["anchor_ratio"]
        + phrase_hits * weights["phrase"]
        + title_hits * weights["title"]
        + path_hits * weights["path"]
        + digest * weights["digest"]
        + source_boost * weights["source"]
        + embedding * weights["embedding"]
        + raw_rank * weights["raw_rank"]
        + top20_rank * weights["top20_rank"]
    )
    return {
        "score": score,
        "weighted_overlap": weighted_overlap,
        "repeat_overlap": float(repeat_overlap),
        "coverage": coverage,
        "anchor_hits": float(anchor_hits),
        "anchor_ratio": anchor_ratio,
        "phrase_hits": float(phrase_hits),
        "title_hits": float(title_hits),
        "path_hits": float(path_hits),
        "digest": digest,
        "source": source_boost,
        "embedding": embedding,
        "raw_rank": raw_rank,
        "top20_rank": top20_rank,
    }
