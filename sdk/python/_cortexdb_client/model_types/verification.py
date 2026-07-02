from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class EvidenceResponse:
    cell_id: int
    matched_terms: int
    match_score_q16: int
    match_kind: str
    source_trust_q16: int
    source_trust_category: str
    citation: str | None
    payload_text: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "EvidenceResponse":
        return cls(
            cell_id=int(value["cell_id"]),
            matched_terms=int(value["matched_terms"]),
            match_score_q16=int(value.get("match_score_q16", 0)),
            match_kind=str(value.get("match_kind", "")),
            source_trust_q16=int(value["source_trust_q16"]),
            source_trust_category=str(value.get("source_trust_category", "")),
            citation=str(value["citation"]) if value.get("citation") is not None else None,
            payload_text=str(value["payload_text"]),
        )


@dataclass(frozen=True)
class GuardResponse:
    cell_id: int | None
    code: str
    message: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "GuardResponse":
        return cls(
            cell_id=int(value["cell_id"]) if value.get("cell_id") is not None else None,
            code=str(value["code"]),
            message=str(value["message"]),
        )


@dataclass(frozen=True)
class NumericConflictResponse:
    kind: str
    metric: str
    left: str
    right: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "NumericConflictResponse":
        return cls(
            kind=str(value.get("kind", "numeric")),
            metric=str(value["metric"]),
            left=str(value["left"]),
            right=str(value["right"]),
        )


@dataclass(frozen=True)
class VerificationReportResponse:
    fact: str
    status: str
    verdict: str
    confidence_q16: int
    evidence: tuple[EvidenceResponse, ...]
    contradicting_evidence: tuple[EvidenceResponse, ...]
    guards: tuple[GuardResponse, ...]
    supporting: tuple[EvidenceResponse, ...]
    contradicting: tuple[EvidenceResponse, ...]
    numeric_conflicts: tuple[NumericConflictResponse, ...]
    accountability_receipt: dict[str, Any] | None = None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "VerificationReportResponse":
        return cls(
            fact=str(value["fact"]),
            status=str(value["status"]),
            verdict=str(value["verdict"]),
            confidence_q16=int(value.get("confidence_q16", 0)),
            evidence=tuple(EvidenceResponse.from_json(row) for row in value["evidence"]),
            contradicting_evidence=tuple(EvidenceResponse.from_json(row) for row in value["contradicting_evidence"]),
            guards=tuple(GuardResponse.from_json(row) for row in value["guards"]),
            supporting=tuple(EvidenceResponse.from_json(row) for row in value["supporting"]),
            contradicting=tuple(EvidenceResponse.from_json(row) for row in value["contradicting"]),
            numeric_conflicts=tuple(NumericConflictResponse.from_json(row) for row in value["numeric_conflicts"]),
            accountability_receipt=(
                dict(value["accountability_receipt"])
                if value.get("accountability_receipt") is not None
                else None
            ),
        )
