from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from enterprise_neighbors import enterprise_neighbor_keys

from .io import extract_document_content, read_json, source_type
from .query import phrase_ngrams, path_tokens, tokens

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

