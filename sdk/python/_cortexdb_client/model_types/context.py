from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class ExplainResponse:
    score: int
    matched_terms: tuple[str, ...]
    why_selected: str
    base_bm25: int
    source_trust_bonus: int
    redundancy_penalty: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ExplainResponse":
        return cls(
            score=int(value["score"]),
            matched_terms=tuple(str(row) for row in value["matched_terms"]),
            why_selected=str(value["why_selected"]),
            base_bm25=int(value["base_bm25"]),
            source_trust_bonus=int(value["source_trust_bonus"]),
            redundancy_penalty=int(value["redundancy_penalty"]),
        )


@dataclass(frozen=True)
class SourceRefResponse:
    source_id: str
    document_id: str | None
    page: int | None
    cell_range: str | None
    json_path: str | None
    confidence_q16: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "SourceRefResponse":
        return cls(
            source_id=str(value["source_id"]),
            document_id=str(value["document_id"]) if value.get("document_id") is not None else None,
            page=int(value["page"]) if value.get("page") is not None else None,
            cell_range=str(value["cell_range"]) if value.get("cell_range") is not None else None,
            json_path=str(value["json_path"]) if value.get("json_path") is not None else None,
            confidence_q16=int(value["confidence_q16"]),
        )


@dataclass(frozen=True)
class ContextPackCellResponse:
    cell_id: int
    estimated_tokens: int
    citation: str | None
    payload_text: str
    explain: ExplainResponse | None
    source_ref: SourceRefResponse | None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ContextPackCellResponse":
        explain = value.get("explain")
        source_ref = value.get("source_ref")
        return cls(
            cell_id=int(value["cell_id"]),
            estimated_tokens=int(value["estimated_tokens"]),
            citation=str(value["citation"]) if value.get("citation") is not None else None,
            payload_text=str(value["payload_text"]),
            explain=ExplainResponse.from_json(explain) if explain else None,
            source_ref=SourceRefResponse.from_json(source_ref) if source_ref else None,
        )


@dataclass(frozen=True)
class ContextPackAnomalyResponse:
    cell_id: int | None
    code: str
    message: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ContextPackAnomalyResponse":
        return cls(
            cell_id=int(value["cell_id"]) if value.get("cell_id") is not None else None,
            code=str(value["code"]),
            message=str(value["message"]),
        )


@dataclass(frozen=True)
class ContextPackResponse:
    schema_version: str
    token_budget_tokens: int
    estimated_tokens: int
    truncated: bool
    citations_required: bool
    cells: tuple[ContextPackCellResponse, ...]
    anomalies: tuple[ContextPackAnomalyResponse, ...]
    grounding_report: "AnswerGroundingReportResponse | None" = None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ContextPackResponse":
        grounding_value = value.get("grounding_report")
        return cls(
            schema_version=str(value["schema_version"]),
            token_budget_tokens=int(value["token_budget_tokens"]),
            estimated_tokens=int(value["estimated_tokens"]),
            truncated=bool(value["truncated"]),
            citations_required=bool(value["citations_required"]),
            cells=tuple(ContextPackCellResponse.from_json(row) for row in value["cells"]),
            anomalies=tuple(ContextPackAnomalyResponse.from_json(row) for row in value.get("anomalies", [])),
            grounding_report=AnswerGroundingReportResponse.from_json(grounding_value)
            if isinstance(grounding_value, dict)
            else None,
        )

    def ground_answer(
        self,
        answer: str,
        *,
        min_span_support_q16: int = 65535,
        require_citations: bool = False,
        reject_unsupported: bool = False,
    ) -> "AnswerGroundingReportResponse":
        from ..grounding import ground_answer

        return ground_answer(
            self,
            answer,
            min_span_support_q16=min_span_support_q16,
            require_citations=require_citations,
            reject_unsupported=reject_unsupported,
        )


@dataclass(frozen=True)
class AnswerGroundingSpanResponse:
    text: str
    start_byte: int
    end_byte: int
    support_q16: int
    supported: bool
    covered_terms: tuple[str, ...]
    missing_terms: tuple[str, ...]
    supported_by_cell_ids: tuple[int, ...]
    citations: tuple[str, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnswerGroundingSpanResponse":
        return cls(
            text=str(value["text"]),
            start_byte=int(value["start_byte"]),
            end_byte=int(value["end_byte"]),
            support_q16=int(value["support_q16"]),
            supported=bool(value["supported"]),
            covered_terms=tuple(str(item) for item in value.get("covered_terms", [])),
            missing_terms=tuple(str(item) for item in value.get("missing_terms", [])),
            supported_by_cell_ids=tuple(
                int(item) for item in value.get("supported_by_cell_ids", [])
            ),
            citations=tuple(str(item) for item in value.get("citations", [])),
        )


@dataclass(frozen=True)
class AnswerGroundingReportResponse:
    answer_supported: bool
    rejected: bool
    support_q16: int
    supported_span_count: int
    unsupported_span_count: int
    spans: tuple[AnswerGroundingSpanResponse, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnswerGroundingReportResponse":
        return cls(
            answer_supported=bool(value["answer_supported"]),
            rejected=bool(value["rejected"]),
            support_q16=int(value["support_q16"]),
            supported_span_count=int(value["supported_span_count"]),
            unsupported_span_count=int(value["unsupported_span_count"]),
            spans=tuple(
                AnswerGroundingSpanResponse.from_json(row)
                for row in value.get("spans", [])
            ),
        )


@dataclass(frozen=True)
class GroundedAnswerResponse:
    question: str
    answer: str
    retrieve_statement: str
    verify_statement: str | None
    context: ContextPackResponse
    grounding: AnswerGroundingReportResponse
    verification: "VerificationReportResponse | None"
    citations: tuple[str, ...]
    used_context_cell_ids: tuple[int, ...]
    rejected: bool
