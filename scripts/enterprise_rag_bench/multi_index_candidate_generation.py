#!/usr/bin/env python3
"""Generate EnterpriseRAG candidates from multiple local indexes.

This is a retrieval-only stage: it uses benchmark question metadata, existing
candidate rows, and the corpus UUID/path index. It does not call an LLM.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from enterprise_neighbors import enterprise_neighbor_keys
from question_decomposition import evidence_units, precise_anchors, tokens


PATH_STOPWORDS = {
    "and",
    "for",
    "the",
    "with",
    "json",
    "sources",
    "users",
    "shared",
    "drives",
    "team",
    "wiki",
}


QUERY_TYPE_ROUTE_PRESETS = {
    "basic": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "semantic": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "project_related": {
        "content_boost_limit": 80,
        "content_weight": 0.012,
        "path_weight": 1.15,
    },
    "completeness": {
        "content_boost_limit": 80,
        "content_weight": 0.012,
        "path_weight": 1.0,
    },
    "conflicting_info": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
    "constrained": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "content_existing_only": True,
        "path_weight": 1.0,
    },
    "intra_document_reasoning": {
        "content_boost_limit": 40,
        "content_weight": 0.01,
        "path_weight": 1.0,
    },
}


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    if not isinstance(title_field, str) or title_field not in doc:
        return ("", json.dumps(doc, ensure_ascii=False))
    title = str(doc.get(title_field, ""))
    if not isinstance(content_fields, list) or not content_fields:
        return (title, json.dumps(doc, ensure_ascii=False))
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return (title, "\n\n".join(parts))


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in indexed:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        indexed[qid] = row
    return indexed


def source_type(path: str) -> str:
    return path.split("/", 1)[0] if path else "unknown"


def path_tokens(path: str, *, expand_ngrams: bool = False) -> list[str]:
    base_tokens = [
        token
        for token in tokens(path.replace(".json", " "))
        if token not in PATH_STOPWORDS and len(token) > 1
    ]
    if not expand_ngrams:
        return base_tokens
    expanded: set[str] = set(base_tokens)
    for token in base_tokens:
        pieces = [piece for piece in re.split(r"[-_/.:]+", token) if piece and piece not in PATH_STOPWORDS]
        expanded.update(piece for piece in pieces if len(piece) > 1)
        for width in (2, 3, 4):
            for index in range(0, max(0, len(pieces) - width + 1)):
                expanded.add("-".join(pieces[index : index + width]))
                expanded.add("_".join(pieces[index : index + width]))
    return sorted(expanded)


def enterprise_entities(question_text: str) -> list[str]:
    entities: set[str] = set()
    patterns = [
        r"`([^`]+)`",
        r"\b[A-Z]{2,}[A-Z0-9_-]*-\d+[A-Z0-9_-]*\b",
        r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z0-9]+){1,5}\b",
        r"\b[A-Z][a-z]+(?:[A-Z][a-z0-9]+)+\b",
        r"\b[a-z]+-[a-z0-9-]+(?:-[a-z0-9]+)*\b",
        r"\b[A-Za-z0-9_./:-]+/[A-Za-z0-9_./:-]+\b",
        r"\b\d+(?:\.\d+)?(?:%|ms|s|kb|mb|gb|mib|gib|hours?|minutes?)\b",
    ]
    for pattern in patterns:
        for match in re.findall(pattern, question_text):
            value = match if isinstance(match, str) else match[0]
            normalized = " ".join(tokens(value))
            if not normalized:
                continue
            entities.add(normalized)
            if " " in normalized:
                entities.add(normalized.replace(" ", "-"))
                entities.add(normalized.replace(" ", "_"))
            if "/" in normalized:
                entities.add(normalized.replace("/", "-"))
                entities.add(normalized.replace("/", "_"))
    return sorted(entities, key=lambda value: (-len(value), value))


def query_terms(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    values = tokens(question_text)
    for anchor in precise_anchors(question_text):
        values.extend(tokens(anchor))
    for entity in enterprise_entities(question_text):
        values.extend(tokens(entity))
        values.append(entity)
    return sorted(set(values), key=lambda value: (-len(value), value))


def strong_uncapped_terms(question: dict[str, Any]) -> set[str]:
    strong: set[str] = set()
    question_text = str(question.get("question", ""))
    for anchor in precise_anchors(question_text):
        for token in tokens(anchor):
            if "-" in token or any(char.isdigit() for char in token):
                strong.add(token)
    for token in tokens(question_text):
        if "-" in token or any(char.isdigit() for char in token):
            strong.add(token)
        elif token in {"p50", "p90", "p95", "p99", "rpo", "rto"}:
            strong.add(token)
    return strong


def phrase_ngrams(values: list[str], *, widths: tuple[int, ...] = (2, 3, 4)) -> set[str]:
    phrases: set[str] = set()
    for width in widths:
        for index in range(0, max(0, len(values) - width + 1)):
            phrase_tokens = values[index : index + width]
            if not phrase_tokens:
                continue
            if not any(
                "-" in token or any(char.isdigit() for char in token) or len(token) >= 6
                for token in phrase_tokens
            ):
                continue
            phrases.add(" ".join(phrase_tokens))
    return phrases


def query_phrases(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    phrases = phrase_ngrams(tokens(question_text))
    for unit in evidence_units(question_text):
        unit_tokens = [str(token) for token in unit.get("tokens", []) if str(token)]
        phrases.update(phrase_ngrams(unit_tokens))
    for anchor in precise_anchors(question_text):
        anchor_tokens = tokens(anchor)
        if anchor_tokens and (
            len(anchor_tokens) > 1
            or "-" in anchor
            or any(char.isdigit() for char in anchor)
        ):
            phrases.add(" ".join(anchor_tokens))
    return sorted(phrases, key=lambda value: (-len(value), value))


def path_query_terms(question: dict[str, Any]) -> list[str]:
    question_text = str(question.get("question", ""))
    values: list[str] = []
    for entity in enterprise_entities(question_text):
        values.append(entity)
        values.extend(tokens(entity))
    for anchor in precise_anchors(question_text):
        if any(char in anchor for char in "-_/.:") or any(char.isdigit() for char in anchor):
            values.append(anchor)
            values.extend(tokens(anchor))
    if not values:
        values = [
            term
            for term in query_terms(question)
            if any(char in term for char in "-_/.:") or any(char.isdigit() for char in term)
        ]
    return sorted(set(values), key=lambda value: (-len(value), value))


class PathIndex:
    def __init__(self, uuid_index: dict[str, str], *, max_posting: int, expand_ngrams: bool) -> None:
        self.uuid_index = uuid_index
        self.max_posting = max_posting
        self.expand_ngrams = expand_ngrams
        self.doc_tokens: dict[str, set[str]] = {}
        self.doc_source: dict[str, str] = {}
        self.token_to_docs: dict[str, list[str]] = defaultdict(list)
        self.source_counts: Counter[str] = Counter()
        self._build()

    def _build(self) -> None:
        for doc_id, rel_path in self.uuid_index.items():
            src = source_type(rel_path)
            self.doc_source[doc_id] = src
            self.source_counts[src] += 1
            doc_terms = set(path_tokens(rel_path, expand_ngrams=self.expand_ngrams))
            self.doc_tokens[doc_id] = doc_terms
            for token in doc_terms:
                self.token_to_docs[token].append(doc_id)

    def candidate_ids_for_terms(
        self,
        terms: list[str],
        source_types: set[str],
        *,
        max_docs: int,
    ) -> set[str]:
        counts: Counter[str] = Counter()
        for token in terms:
            docs = self.token_to_docs.get(token, [])
            if not docs or len(docs) > self.max_posting:
                continue
            for doc_id in docs:
                if source_types and self.doc_source.get(doc_id) not in source_types:
                    continue
                counts[doc_id] += 1
        return {doc_id for doc_id, _count in counts.most_common(max_docs)}

    def path_score(self, question_terms: set[str], source_types: set[str], doc_id: str) -> float:
        terms = self.doc_tokens.get(doc_id, set())
        if not terms:
            return 0.0
        overlap = question_terms & terms
        if not overlap:
            return 0.0
        src = self.doc_source.get(doc_id, "")
        source_boost = 25.0 if source_types and src in source_types else 0.0
        rare_bonus = 0.0
        for token in overlap:
            posting_len = len(self.token_to_docs.get(token, []))
            if posting_len:
                rare_bonus += math.log((len(self.uuid_index) + 1) / (posting_len + 1))
        return len(overlap) * 12.0 + rare_bonus * 4.0 + source_boost


class ContentPreviewIndex:
    def __init__(
        self,
        uuid_index: dict[str, str],
        sources_dir: Path,
        *,
        target_terms: set[str],
        target_phrases: set[str],
        uncapped_terms: set[str],
        max_posting: int,
        phrase_max_posting: int,
        preview_chars: int,
        include_source_links: bool,
    ) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.target_terms = target_terms
        self.target_phrases = target_phrases
        self.uncapped_terms = uncapped_terms
        self.max_posting = max_posting
        self.phrase_max_posting = phrase_max_posting
        self.preview_chars = preview_chars
        self.include_source_links = include_source_links
        self.doc_tokens: dict[str, set[str]] = {}
        self.doc_phrases: dict[str, set[str]] = {}
        self.title_tokens: dict[str, set[str]] = {}
        self.doc_source: dict[str, str] = {}
        self.doc_neighbor_keys: dict[str, set[str]] = {}
        self.token_to_docs: dict[str, list[str]] = defaultdict(list)
        self.phrase_to_docs: dict[str, list[str]] = defaultdict(list)
        self.neighbor_to_docs: dict[str, list[str]] = defaultdict(list)
        self.skipped_files = 0
        self.indexed_docs = 0
        self.neighbor_indexed_docs = 0
        self._build()

    def _build(self) -> None:
        capped_tokens: set[str] = set()
        for doc_id, rel_path in self.uuid_index.items():
            src = source_type(rel_path)
            self.doc_source[doc_id] = src
            path = self.sources_dir / rel_path
            if path.suffix != ".json":
                self.skipped_files += 1
                continue
            try:
                document = read_json(path)
                title, content = extract_document_content(document)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                self.skipped_files += 1
                continue
            neighbor_keys = enterprise_neighbor_keys(
                document,
                rel_path,
                include_source_links=self.include_source_links,
            )
            if neighbor_keys:
                self.neighbor_indexed_docs += 1
                self.doc_neighbor_keys[doc_id] = neighbor_keys
                for key in neighbor_keys:
                    self.neighbor_to_docs[key].append(doc_id)
            title_terms = set(tokens(title)) & self.target_terms
            preview = f"{title}\n{content[: self.preview_chars]}"
            preview_tokens = tokens(preview)
            doc_terms = set(preview_tokens) & self.target_terms
            doc_phrases = phrase_ngrams(preview_tokens) & self.target_phrases if self.target_phrases else set()
            if not doc_terms:
                continue
            self.indexed_docs += 1
            self.doc_tokens[doc_id] = doc_terms
            self.doc_phrases[doc_id] = doc_phrases
            self.title_tokens[doc_id] = title_terms
            for token in doc_terms:
                is_uncapped = token in self.uncapped_terms
                if not is_uncapped and token in capped_tokens:
                    continue
                posting = self.token_to_docs[token]
                if not is_uncapped and len(posting) >= self.max_posting:
                    capped_tokens.add(token)
                    posting.clear()
                    continue
                posting.append(doc_id)
            for phrase in doc_phrases:
                posting = self.phrase_to_docs[phrase]
                if len(posting) < self.phrase_max_posting:
                    posting.append(doc_id)

    def candidate_ids_for_terms(
        self,
        terms: list[str],
        source_types: set[str],
        *,
        max_docs: int,
    ) -> set[str]:
        counts: Counter[str] = Counter()
        for token in terms:
            docs = self.token_to_docs.get(token, [])
            if not docs:
                continue
            for doc_id in docs:
                if source_types and self.doc_source.get(doc_id) not in source_types:
                    continue
                counts[doc_id] += 1
        return {doc_id for doc_id, _count in counts.most_common(max_docs)}

    def content_score(self, question_terms: set[str], source_types: set[str], doc_id: str) -> float:
        terms = self.doc_tokens.get(doc_id, set())
        if not terms:
            return 0.0
        overlap = question_terms & terms
        if not overlap:
            return 0.0
        title_overlap = question_terms & self.title_tokens.get(doc_id, set())
        src = self.doc_source.get(doc_id, "")
        source_boost = 30.0 if source_types and src in source_types else 0.0
        rare_bonus = 0.0
        for token in overlap:
            posting_len = len(self.token_to_docs.get(token, []))
            if posting_len:
                rare_bonus += math.log((len(self.uuid_index) + 1) / (posting_len + 1))
        return (
            len(overlap) * 16.0
            + len(title_overlap) * 38.0
            + rare_bonus * 5.0
            + source_boost
        )

    def candidate_ids_for_phrases(
        self,
        phrases: list[str],
        source_types: set[str],
        *,
        max_docs: int,
    ) -> set[str]:
        counts: Counter[str] = Counter()
        for phrase in phrases:
            docs = self.phrase_to_docs.get(phrase, [])
            if not docs:
                continue
            for doc_id in docs:
                if source_types and self.doc_source.get(doc_id) not in source_types:
                    continue
                counts[doc_id] += 1
        return {doc_id for doc_id, _count in counts.most_common(max_docs)}

    def phrase_score(self, question_phrases: set[str], source_types: set[str], doc_id: str) -> float:
        phrases = self.doc_phrases.get(doc_id, set())
        if not phrases:
            return 0.0
        overlap = question_phrases & phrases
        if not overlap:
            return 0.0
        src = self.doc_source.get(doc_id, "")
        source_boost = 24.0 if source_types and src in source_types else 0.0
        rare_bonus = 0.0
        specificity = 0.0
        for phrase in overlap:
            posting_len = len(self.phrase_to_docs.get(phrase, []))
            if posting_len:
                rare_bonus += math.log((len(self.uuid_index) + 1) / (posting_len + 1))
            specificity += len(phrase.split()) * 10.0
            if "-" in phrase or any(char.isdigit() for char in phrase):
                specificity += 18.0
        return len(overlap) * 30.0 + specificity + rare_bonus * 8.0 + source_boost

    def neighbor_scores(
        self,
        seed_doc_ids: list[str],
        source_types: set[str],
        *,
        max_docs: int,
        max_per_seed: int,
        max_posting: int,
    ) -> list[tuple[float, str]]:
        scores: Counter[str] = Counter()
        for seed_rank, seed_doc_id in enumerate(seed_doc_ids, 1):
            seed_weight = 1.0 / math.sqrt(seed_rank)
            seed_neighbors: Counter[str] = Counter()
            for key in self.doc_neighbor_keys.get(seed_doc_id, set()):
                docs = self.neighbor_to_docs.get(key, [])
                if not docs or len(docs) > max_posting:
                    continue
                rarity = math.log((len(self.uuid_index) + 1) / (len(docs) + 1))
                for doc_id in docs:
                    if doc_id == seed_doc_id:
                        continue
                    if source_types and self.doc_source.get(doc_id) not in source_types:
                        continue
                    seed_neighbors[doc_id] += seed_weight * (8.0 + rarity * 3.0)
            for doc_id, score in seed_neighbors.most_common(max_per_seed):
                scores[doc_id] += score
        return [(score, doc_id) for doc_id, score in scores.most_common(max_docs)]

    def report(self) -> dict[str, Any]:
        return {
            "indexed_docs": self.indexed_docs,
            "neighbor_indexed_docs": self.neighbor_indexed_docs,
            "posting_terms": len(self.token_to_docs),
            "posting_phrases": len(self.phrase_to_docs),
            "posting_neighbor_keys": len(self.neighbor_to_docs),
            "skipped_files": self.skipped_files,
        }


def doc_ids(row: dict[str, Any] | None) -> list[str]:
    if not row:
        return []
    return [str(item) for item in row.get("document_ids", []) if str(item)]


def add_rrf(scores: dict[str, float], doc_ids_: list[str], *, weight: float, k: int) -> None:
    for rank, doc_id in enumerate(doc_ids_, 1):
        scores[doc_id] = scores.get(doc_id, 0.0) + weight / (k + rank)


def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)


def route_settings(question_type: str, args: argparse.Namespace) -> dict[str, Any]:
    settings = {
        "policy": "multi_index_candidates_v4_content_safe",
        "content_boost_limit": args.content_boost_limit,
        "content_existing_only": question_type in args.content_existing_only_question_type,
        "content_weight": args.weight_content,
        "path_weight": args.weight_path,
    }
    if args.enable_query_type_router:
        preset = QUERY_TYPE_ROUTE_PRESETS.get(question_type, {})
        settings.update(preset)
        settings["policy"] = "multi_index_candidates_query_type_router_v1"
    return settings


def run(args: argparse.Namespace) -> dict[str, Any]:
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    base_rows = rows_by_id(read_jsonl(args.base_retrieval_file), "base retrieval")
    extra_rows = [
        rows_by_id(read_jsonl(path), f"extra retrieval {path}")
        for path in args.extra_retrieval_file
    ]
    path_index = PathIndex(
        uuid_index,
        max_posting=args.max_posting,
        expand_ngrams=args.enable_path_ngrams,
    )
    all_query_terms = {
        term
        for question in questions.values()
        for term in query_terms(question)
        if len(term) > 1
    }
    all_uncapped_terms = {
        term
        for question in questions.values()
        for term in strong_uncapped_terms(question)
    }
    all_query_phrases = (
        {
            phrase
            for question in questions.values()
            for phrase in query_phrases(question)
            if len(phrase) > 3
        }
        if args.phrase_candidate_limit > 0 and args.phrase_boost_limit > 0
        else set()
    )
    content_index = (
        ContentPreviewIndex(
            uuid_index,
            args.sources_dir,
            target_terms=all_query_terms,
            target_phrases=all_query_phrases,
            uncapped_terms=all_uncapped_terms,
            max_posting=args.max_posting,
            phrase_max_posting=args.phrase_max_posting,
            preview_chars=args.content_preview_chars,
            include_source_links=args.enable_source_link_neighbors,
        )
        if args.sources_dir
        else None
    )

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    source_counts: Counter[str] = Counter()
    route_counts: Counter[str] = Counter()
    diagnostics: dict[str, Any] = {}

    for qid, question in questions.items():
        question_type = str(question.get("question_type", ""))
        settings = route_settings(question_type, args)
        route_counts[str(settings["policy"]) + ":" + (question_type or "unknown")] += 1
        source_types = {str(item) for item in question.get("source_types", []) if str(item)}
        terms = query_terms(question)
        term_set = set(terms)
        phrases = query_phrases(question)
        phrase_set = set(phrases)
        path_terms = path_query_terms(question) if args.path_terms_mode == "entity" else terms
        path_term_set = set(path_terms)
        scores: dict[str, float] = {}
        score_sources: dict[str, set[str]] = defaultdict(set)

        base_doc_ids = doc_ids(base_rows.get(qid))
        add_rrf(scores, base_doc_ids[: args.base_limit], weight=args.weight_base_rrf, k=args.rrf_k)
        for doc_id in base_doc_ids[: args.base_limit]:
            score_sources[doc_id].add("base")

        for extra_index, rows in enumerate(extra_rows, 1):
            ids = doc_ids(rows.get(qid))[: args.extra_limit]
            add_rrf(scores, ids, weight=args.weight_extra_rrf, k=args.rrf_k)
            for doc_id in ids:
                score_sources[doc_id].add(f"extra_{extra_index}")

        path_docs = path_index.candidate_ids_for_terms(
            path_terms,
            source_types,
            max_docs=args.path_candidate_limit,
        )
        for doc_id in path_docs:
            if args.path_existing_only and doc_id not in scores:
                continue
            score = path_index.path_score(path_term_set, source_types, doc_id)
            if score <= 0.0:
                continue
            scores[doc_id] = scores.get(doc_id, 0.0) + score * float(settings["path_weight"])
            score_sources[doc_id].add("path")

        if content_index:
            content_existing_only = bool(settings["content_existing_only"])
            content_docs = content_index.candidate_ids_for_terms(
                terms,
                source_types,
                max_docs=args.content_candidate_limit,
            )
            scored_content: list[tuple[float, str]] = []
            for doc_id in content_docs:
                if content_existing_only and doc_id not in scores:
                    continue
                score = content_index.content_score(term_set, source_types, doc_id)
                if score < args.content_score_threshold:
                    continue
                scored_content.append((score, doc_id))
            scored_content.sort(key=lambda item: (-item[0], path_index.uuid_index.get(item[1], ""), item[1]))
            for score, doc_id in scored_content[: int(settings["content_boost_limit"])]:
                scores[doc_id] = scores.get(doc_id, 0.0) + score * float(settings["content_weight"])
                score_sources[doc_id].add("content_preview")

            if args.phrase_candidate_limit > 0 and args.phrase_boost_limit > 0 and phrases:
                phrase_docs = content_index.candidate_ids_for_phrases(
                    phrases,
                    source_types,
                    max_docs=args.phrase_candidate_limit,
                )
                scored_phrases: list[tuple[float, str]] = []
                for doc_id in phrase_docs:
                    score = content_index.phrase_score(phrase_set, source_types, doc_id)
                    if score <= 0.0:
                        continue
                    scored_phrases.append((score, doc_id))
                scored_phrases.sort(key=lambda item: (-item[0], path_index.uuid_index.get(item[1], ""), item[1]))
                for score, doc_id in scored_phrases[: args.phrase_boost_limit]:
                    scores[doc_id] = scores.get(doc_id, 0.0) + score * args.weight_phrase
                    score_sources[doc_id].add("source_phrase")

            if args.neighbor_expansion_limit > 0 and scores:
                seed_ids = sorted(
                    scores,
                    key=lambda doc_id: (
                        -scores[doc_id],
                        path_index.uuid_index.get(doc_id, ""),
                        doc_id,
                    ),
                )[: args.neighbor_seed_limit]
                for score, doc_id in content_index.neighbor_scores(
                    seed_ids,
                    source_types,
                    max_docs=args.neighbor_expansion_limit,
                    max_per_seed=args.neighbor_max_per_seed,
                    max_posting=args.neighbor_max_posting,
                ):
                    scores[doc_id] = scores.get(doc_id, 0.0) + score * args.weight_neighbor
                    score_sources[doc_id].add("neighbor")

        if source_types and args.source_match_boost != 0.0:
            for doc_id in list(scores):
                doc_source = source_type(path_index.uuid_index.get(doc_id, ""))
                if doc_source in source_types:
                    scores[doc_id] += args.source_match_boost
                    score_sources[doc_id].add("source_type_boost")

        reranked = sorted(
            scores,
            key=lambda doc_id: (
                -scores[doc_id],
                -len(score_sources[doc_id]),
                path_index.uuid_index.get(doc_id, ""),
                doc_id,
            ),
        )
        selected = reranked[: args.top_k]
        recall = recall_pct(question, selected)
        if recall is not None:
            recall_values.append(recall)
        for doc_id in selected:
            for source in score_sources.get(doc_id, {"unknown"}):
                source_counts[source] += 1
        output_rows.append(
            {
                "answer": "",
                "document_ids": selected,
                "question": question.get("question", ""),
                "question_id": qid,
                "question_type": question.get("question_type"),
                "route": {
                    "policy": settings["policy"],
                    "top_k": args.top_k,
                    "content_boost_limit": int(settings["content_boost_limit"]),
                    "content_existing_only": bool(settings["content_existing_only"]),
                    "content_weight": float(settings["content_weight"]),
                    "path_weight": float(settings["path_weight"]),
                    "source_types": sorted(source_types),
                },
            }
        )
        if args.diagnostics_top_k > 0:
            diagnostics[qid] = {
                "recall_pct": recall,
                "terms": terms[:24],
                "path_terms": path_terms[:24],
                "route": settings,
                "candidate_sources": [
                    {
                        "doc_id": doc_id,
                        "score": round(scores[doc_id], 4),
                        "sources": sorted(score_sources[doc_id]),
                        "path": path_index.uuid_index.get(doc_id, ""),
                    }
                    for doc_id in selected[: args.diagnostics_top_k]
                ],
            }

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.multi_index_candidates.v1",
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "base_retrieval_file": str(args.base_retrieval_file),
        "extra_retrieval_files": [str(path) for path in args.extra_retrieval_file],
        "uuid_index": str(args.uuid_index),
        "output": str(args.output),
        "top_k": args.top_k,
        "base_limit": args.base_limit,
        "extra_limit": args.extra_limit,
        "path_candidate_limit": args.path_candidate_limit,
        "path_existing_only": args.path_existing_only,
        "path_terms_mode": args.path_terms_mode,
        "enable_path_ngrams": args.enable_path_ngrams,
        "content_candidate_limit": args.content_candidate_limit,
        "content_boost_limit": args.content_boost_limit,
        "content_preview_chars": args.content_preview_chars,
        "content_score_threshold": args.content_score_threshold,
        "phrase_candidate_limit": args.phrase_candidate_limit,
        "phrase_boost_limit": args.phrase_boost_limit,
        "phrase_max_posting": args.phrase_max_posting,
        "weight_phrase": args.weight_phrase,
        "neighbor_expansion_limit": args.neighbor_expansion_limit,
        "neighbor_seed_limit": args.neighbor_seed_limit,
        "neighbor_max_per_seed": args.neighbor_max_per_seed,
        "neighbor_max_posting": args.neighbor_max_posting,
        "weight_neighbor": args.weight_neighbor,
        "enable_source_link_neighbors": args.enable_source_link_neighbors,
        "max_posting": args.max_posting,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "source_counts": dict(sorted(source_counts.items())),
        "source_match_boost": args.source_match_boost,
        "route_counts": dict(sorted(route_counts.items())),
        "content_index": content_index.report() if content_index else None,
        "diagnostics": diagnostics,
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--base-retrieval-file", type=Path, required=True)
    parser.add_argument("--extra-retrieval-file", type=Path, action="append", default=[])
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--top-k", type=int, default=500)
    parser.add_argument("--base-limit", type=int, default=500)
    parser.add_argument("--extra-limit", type=int, default=500)
    parser.add_argument("--path-candidate-limit", type=int, default=800)
    parser.add_argument("--path-existing-only", action="store_true")
    parser.add_argument("--path-terms-mode", choices=["all", "entity"], default="all")
    parser.add_argument("--enable-path-ngrams", action="store_true")
    parser.add_argument("--content-candidate-limit", type=int, default=1200)
    parser.add_argument("--content-boost-limit", type=int, default=40)
    parser.add_argument("--content-preview-chars", type=int, default=1800)
    parser.add_argument("--content-score-threshold", type=float, default=0.0)
    parser.add_argument("--phrase-candidate-limit", type=int, default=0)
    parser.add_argument("--phrase-boost-limit", type=int, default=0)
    parser.add_argument("--phrase-max-posting", type=int, default=50000)
    parser.add_argument("--neighbor-expansion-limit", type=int, default=0)
    parser.add_argument("--neighbor-seed-limit", type=int, default=40)
    parser.add_argument("--neighbor-max-per-seed", type=int, default=6)
    parser.add_argument("--neighbor-max-posting", type=int, default=400)
    parser.add_argument("--enable-source-link-neighbors", action="store_true")
    parser.add_argument(
        "--content-existing-only-question-type",
        action="append",
        default=["constrained"],
        help="For these question types, content preview only boosts docs already found by base/extra retrieval.",
    )
    parser.add_argument("--max-posting", type=int, default=12000)
    parser.add_argument("--rrf-k", type=int, default=60)
    parser.add_argument("--weight-base-rrf", type=float, default=900.0)
    parser.add_argument("--weight-extra-rrf", type=float, default=500.0)
    parser.add_argument("--weight-path", type=float, default=1.0)
    parser.add_argument("--weight-content", type=float, default=0.01)
    parser.add_argument("--weight-phrase", type=float, default=1.0)
    parser.add_argument("--weight-neighbor", type=float, default=1.0)
    parser.add_argument("--source-match-boost", type=float, default=0.0)
    parser.add_argument("--enable-query-type-router", action="store_true")
    parser.add_argument("--diagnostics-top-k", type=int, default=5)
    args = parser.parse_args()
    for name in (
        "top_k",
        "base_limit",
        "extra_limit",
        "path_candidate_limit",
        "content_candidate_limit",
        "content_boost_limit",
        "content_preview_chars",
        "max_posting",
        "rrf_k",
        "phrase_max_posting",
        "neighbor_seed_limit",
        "neighbor_max_per_seed",
        "neighbor_max_posting",
    ):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    for name in ("phrase_candidate_limit", "phrase_boost_limit", "neighbor_expansion_limit"):
        if getattr(args, name) < 0:
            parser.error(f"--{name.replace('_', '-')} must be non-negative")
    if args.diagnostics_top_k < 0:
        parser.error("--diagnostics-top-k must be non-negative")
    return args


def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "questions": report["questions"],
                "average_recall_pct": report["average_recall_pct"],
                "full_recall_questions": report["full_recall_questions"],
                "output": report["output"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
