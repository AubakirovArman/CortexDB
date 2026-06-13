from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class IngestResponse:
    rows_ingested: int
    chunks_ingested: int
    facts_ingested: int
    first_cell_id: int | None
    job_id: int | None
    validation_report: dict[str, Any]

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "IngestResponse":
        return cls(
            rows_ingested=int(value["rows_ingested"]),
            chunks_ingested=int(value["chunks_ingested"]),
            facts_ingested=int(value["facts_ingested"]),
            first_cell_id=int(value["first_cell_id"]) if value.get("first_cell_id") is not None else None,
            job_id=int(value["job_id"]) if value.get("job_id") is not None else None,
            validation_report=dict(value.get("validation_report", {})),
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
