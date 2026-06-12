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
    recall_q16: int | None
    min_recall_q16: int | None
    hnsw_ef_construction: int
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
            recall_q16=int(recall) if recall is not None else None,
            min_recall_q16=int(minimum) if minimum is not None else None,
            hnsw_ef_construction=int(value.get("hnsw_ef_construction", 0)),
            require_slo=bool(value.get("require_slo", False)),
            production_safe=bool(value.get("production_safe", True)),
            slo_violations=tuple(str(item) for item in value.get("slo_violations", [])),
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
    ann_report: AnnSearchReport | None
    results: tuple[SearchResult, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "SearchResponse":
        report = value.get("ann_report")
        return cls(
            search_mode=str(value["search_mode"]),
            ann_report=AnnSearchReport.from_json(report) if report else None,
            results=tuple(SearchResult.from_json(row) for row in value["results"]),
        )


@dataclass(frozen=True)
class AnnEvaluationResponse:
    available: bool
    reason: str | None
    ann_report: AnnSearchReport | None
    exact_top_k: tuple[int, ...]
    ann_top_k: tuple[int, ...]
    overlap_count: int
    recall_q16: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnnEvaluationResponse":
        report = value.get("ann_report")
        reason = value.get("reason")
        return cls(
            available=bool(value["available"]),
            reason=str(reason) if reason is not None else None,
            ann_report=AnnSearchReport.from_json(report) if report else None,
            exact_top_k=tuple(int(row) for row in value["exact_top_k"]),
            ann_top_k=tuple(int(row) for row in value["ann_top_k"]),
            overlap_count=int(value["overlap_count"]),
            recall_q16=int(value["recall_q16"]),
        )


@dataclass(frozen=True)
class HealthResponse:
    status: str
    version: str
    server_version: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "HealthResponse":
        return cls(
            status=str(value["status"]),
            version=str(value["version"]),
            server_version=str(value["server_version"]),
        )


@dataclass(frozen=True)
class StatsResponse:
    current_seq: int
    checkpoint_seq: int
    live_segments: int
    retired_segments: int
    memtable_cells: int
    memtable_versions: int
    wal_size_bytes: int
    wal_writer_records: int
    wal_writer_bytes: int
    wal_writer_fsyncs: int
    wal_writer_batches: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "StatsResponse":
        return cls(
            current_seq=int(value["current_seq"]),
            checkpoint_seq=int(value["checkpoint_seq"]),
            live_segments=int(value["live_segments"]),
            retired_segments=int(value["retired_segments"]),
            memtable_cells=int(value["memtable_cells"]),
            memtable_versions=int(value["memtable_versions"]),
            wal_size_bytes=int(value["wal_size_bytes"]),
            wal_writer_records=int(value["wal_writer_records"]),
            wal_writer_bytes=int(value["wal_writer_bytes"]),
            wal_writer_fsyncs=int(value["wal_writer_fsyncs"]),
            wal_writer_batches=int(value["wal_writer_batches"]),
        )


@dataclass(frozen=True)
class ValidationResponse:
    ok: bool
    manifest_ok: bool
    wal_ok: bool
    live_segments_checked: int
    bitmap_indexes_checked: int
    lexical_indexes_checked: int
    vector_indexes_checked: int
    hnsw_graphs_checked: int
    cells_checked: int
    wal_records_checked: int
    wal_safe_truncate_offset: int
    errors: tuple[str, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ValidationResponse":
        return cls(
            ok=bool(value["ok"]),
            manifest_ok=bool(value["manifest_ok"]),
            wal_ok=bool(value["wal_ok"]),
            live_segments_checked=int(value["live_segments_checked"]),
            bitmap_indexes_checked=int(value["bitmap_indexes_checked"]),
            lexical_indexes_checked=int(value["lexical_indexes_checked"]),
            vector_indexes_checked=int(value["vector_indexes_checked"]),
            hnsw_graphs_checked=int(value["hnsw_graphs_checked"]),
            cells_checked=int(value["cells_checked"]),
            wal_records_checked=int(value["wal_records_checked"]),
            wal_safe_truncate_offset=int(value["wal_safe_truncate_offset"]),
            errors=tuple(str(row) for row in value.get("errors", [])),
        )


@dataclass(frozen=True)
class PutCellResponse:
    seq: int
    cell_id: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "PutCellResponse":
        return cls(seq=int(value["seq"]), cell_id=int(value["cell_id"]))


@dataclass(frozen=True)
class CellResponse:
    cell_id: int
    payload: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "CellResponse":
        return cls(cell_id=int(value["cell_id"]), payload=str(value["payload"]))


@dataclass(frozen=True)
class CellLookupResponse:
    cell: CellResponse | None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "CellLookupResponse":
        cell = value.get("cell")
        return cls(cell=CellResponse.from_json(cell) if cell else None)


@dataclass(frozen=True)
class AqlCellResponse:
    cell_id: int
    payload: str

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AqlCellResponse":
        return cls(cell_id=int(value["cell_id"]), payload=str(value["payload"]))


@dataclass(frozen=True)
class AqlResponse:
    cells: tuple[AqlCellResponse, ...]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AqlResponse":
        return cls(cells=tuple(AqlCellResponse.from_json(row) for row in value["cells"]))


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

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ContextPackResponse":
        return cls(
            schema_version=str(value["schema_version"]),
            token_budget_tokens=int(value["token_budget_tokens"]),
            estimated_tokens=int(value["estimated_tokens"]),
            truncated=bool(value["truncated"]),
            citations_required=bool(value["citations_required"]),
            cells=tuple(ContextPackCellResponse.from_json(row) for row in value["cells"]),
            anomalies=tuple(ContextPackAnomalyResponse.from_json(row) for row in value.get("anomalies", [])),
        )

    def ground_answer(
        self,
        answer: str,
        *,
        min_span_support_q16: int = 65535,
        require_citations: bool = False,
        reject_unsupported: bool = False,
    ) -> "AnswerGroundingReportResponse":
        from .grounding import ground_answer

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


@dataclass(frozen=True)
class AnswerGroundingReportResponse:
    answer_supported: bool
    rejected: bool
    support_q16: int
    supported_span_count: int
    unsupported_span_count: int
    spans: tuple[AnswerGroundingSpanResponse, ...]


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


@dataclass(frozen=True)
class IngestResponse:
    rows_ingested: int
    chunks_ingested: int
    facts_ingested: int
    first_cell_id: int | None
    job_id: int | None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "IngestResponse":
        return cls(
            rows_ingested=int(value["rows_ingested"]),
            chunks_ingested=int(value["chunks_ingested"]),
            facts_ingested=int(value["facts_ingested"]),
            first_cell_id=int(value["first_cell_id"]) if value.get("first_cell_id") is not None else None,
            job_id=int(value["job_id"]) if value.get("job_id") is not None else None,
        )


@dataclass(frozen=True)
class IngestionJobResponse:
    job_id: int
    label: str
    status: str
    total_items: int | None
    completed_items: int
    failed_items: int
    last_cell_id: int | None
    message: str | None
    retry_count: int = 0
    max_retries: int = 3

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "IngestionJobResponse":
        return cls(
            job_id=int(value["job_id"]),
            label=str(value["label"]),
            status=str(value["status"]),
            total_items=int(value["total_items"]) if value.get("total_items") is not None else None,
            completed_items=int(value["completed_items"]),
            failed_items=int(value["failed_items"]),
            last_cell_id=int(value["last_cell_id"]) if value.get("last_cell_id") is not None else None,
            message=str(value["message"]) if value.get("message") is not None else None,
            retry_count=int(value.get("retry_count", 0)),
            max_retries=int(value.get("max_retries", 3)),
        )


@dataclass(frozen=True)
class DeleteJobResponse:
    deleted: bool

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "DeleteJobResponse":
        return cls(deleted=bool(value["deleted"]))


@dataclass(frozen=True)
class RememberResponse:
    seq: int
    cell_id: int
    ttl_seconds: int | None

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "RememberResponse":
        return cls(
            seq=int(value["seq"]),
            cell_id=int(value["cell_id"]),
            ttl_seconds=int(value["ttl_seconds"]) if value.get("ttl_seconds") is not None else None,
        )


