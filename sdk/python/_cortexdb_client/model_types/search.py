from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class AnnSearchReport:
    path: str
    fallback_reason: str | None
    fallback_performed: bool
    requested_limit: int
    allowed_candidates: int
    graph_nodes: int
    returned_candidates: int
    visited_candidates: int
    max_visited_candidates: int | None
    recall_q16: int | None
    min_recall_q16: int | None
    hnsw_max_neighbors: int
    hnsw_ef_search: int
    hnsw_ef_construction: int
    hnsw_layer_count: int
    upper_graph_edges: int
    require_slo: bool
    production_safe: bool
    slo_violations: tuple[str, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnnSearchReport":
        reason = value.get("fallback_reason")
        recall = value.get("recall_q16")
        minimum = value.get("min_recall_q16")
        return cls(
            path=str(value["path"]),
            fallback_reason=str(reason) if reason is not None else None,
            fallback_performed=bool(value.get("fallback_performed", False)),
            requested_limit=int(value["requested_limit"]),
            allowed_candidates=int(value["allowed_candidates"]),
            graph_nodes=int(value["graph_nodes"]),
            returned_candidates=int(value["returned_candidates"]),
            visited_candidates=int(value.get("visited_candidates", 0)),
            max_visited_candidates=(
                int(value["max_visited_candidates"])
                if value.get("max_visited_candidates") is not None
                else None
            ),
            recall_q16=int(recall) if recall is not None else None,
            min_recall_q16=int(minimum) if minimum is not None else None,
            hnsw_max_neighbors=int(value.get("hnsw_max_neighbors", 0)),
            hnsw_ef_search=int(value.get("hnsw_ef_search", 0)),
            hnsw_ef_construction=int(value.get("hnsw_ef_construction", 0)),
            hnsw_layer_count=int(value.get("hnsw_layer_count", 0)),
            upper_graph_edges=int(value.get("upper_graph_edges", 0)),
            require_slo=bool(value.get("require_slo", False)),
            production_safe=bool(value.get("production_safe", True)),
            slo_violations=tuple(str(item) for item in value.get("slo_violations", [])),
        )


@dataclass(frozen=True)
class AnnNoFallbackDecision:
    allowed: bool
    reasons: tuple[str, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnnNoFallbackDecision":
        return cls(
            allowed=bool(value["allowed"]),
            reasons=tuple(str(item) for item in value.get("reasons", [])),
        )


@dataclass(frozen=True)
class SearchRoutingDecision:
    requested_mode: str
    selected_strategy: str
    reason: str
    text_available: bool
    vector_available: bool

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "SearchRoutingDecision":
        return cls(
            requested_mode=str(value["requested_mode"]),
            selected_strategy=str(value["selected_strategy"]),
            reason=str(value["reason"]),
            text_available=bool(value["text_available"]),
            vector_available=bool(value["vector_available"]),
        )


@dataclass(frozen=True)
class SearchResult:
    cell_id: int
    score: int
    lexical_score: int
    vector_score: int
    payload: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "SearchResult":
        return cls(
            cell_id=int(value["cell_id"]),
            score=int(value["score"]),
            lexical_score=int(value["lexical_score"]),
            vector_score=int(value["vector_score"]),
            payload=str(value["payload"]),
        )


@dataclass(frozen=True)
class SearchResponse:
    search_mode: str
    routing: SearchRoutingDecision | None
    rerank: str | None
    ann_report: AnnSearchReport | None
    no_fallback_decision: AnnNoFallbackDecision | None
    results: tuple[SearchResult, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "SearchResponse":
        report = value.get("ann_report")
        routing = value.get("routing")
        decision = value.get("no_fallback_decision")
        return cls(
            search_mode=str(value["search_mode"]),
            routing=SearchRoutingDecision.from_json(routing) if routing else None,
            rerank=str(value["rerank"]) if value.get("rerank") is not None else None,
            ann_report=AnnSearchReport.from_json(report) if report else None,
            no_fallback_decision=(
                AnnNoFallbackDecision.from_json(decision) if decision else None
            ),
            results=tuple(SearchResult.from_json(row) for row in value["results"]),
        )


@dataclass(frozen=True)
class AnnEvaluationResponse:
    available: bool
    reason: str | None
    ann_report: AnnSearchReport | None
    no_fallback_decision: AnnNoFallbackDecision | None
    exact_top_k: tuple[int, ...]
    ann_top_k: tuple[int, ...]
    overlap_count: int
    recall_q16: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnnEvaluationResponse":
        report = value.get("ann_report")
        decision = value.get("no_fallback_decision")
        reason = value.get("reason")
        return cls(
            available=bool(value["available"]),
            reason=str(reason) if reason is not None else None,
            ann_report=AnnSearchReport.from_json(report) if report else None,
            no_fallback_decision=(
                AnnNoFallbackDecision.from_json(decision) if decision else None
            ),
            exact_top_k=tuple(int(row) for row in value["exact_top_k"]),
            ann_top_k=tuple(int(row) for row in value["ann_top_k"]),
            overlap_count=int(value["overlap_count"]),
            recall_q16=int(value["recall_q16"]),
        )
