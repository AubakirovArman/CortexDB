from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class EvidenceResponse:
    cell_id: int
    matched_terms: int
    source_trust_q16: int
    citation: str | None
    payload_text: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "EvidenceResponse":
        return cls(
            cell_id=int(value["cell_id"]),
            matched_terms=int(value["matched_terms"]),
            source_trust_q16=int(value["source_trust_q16"]),
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
    metric: str
    left: str
    right: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "NumericConflictResponse":
        return cls(
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
        )

