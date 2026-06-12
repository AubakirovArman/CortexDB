from __future__ import annotations

import json
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass, replace
from typing import Any, Callable

from .aql import build_remember_aql, build_retrieve_context_aql, build_verify_fact_aql
from .errors import CortexDBError
from .grounding import _grounded_answer_response
from .models import (
    AnnEvaluationResponse,
    AqlResponse,
    CellLookupResponse,
    ContextPackResponse,
    DeleteJobResponse,
    GroundedAnswerResponse,
    HealthResponse,
    IngestResponse,
    IngestionJobResponse,
    PutCellResponse,
    RememberResponse,
    SearchResponse,
    StatsResponse,
    ValidationResponse,
    VerificationReportResponse,
)


@dataclass(frozen=True)
class CortexDBClient:
    base_url: str = "http://127.0.0.1:8181"
    token: str | None = None
    tenant: str | None = None
    max_retries: int = 0
    retry_delay_seconds: float = 0.5

    def with_tenant(self, tenant: str) -> "CortexDBClient":
        return replace(self, tenant=tenant)

    def with_retries(self, max_retries: int, retry_delay_seconds: float = 0.5) -> "CortexDBClient":
        return replace(self, max_retries=max_retries, retry_delay_seconds=retry_delay_seconds)

    build_retrieve_context_aql = staticmethod(build_retrieve_context_aql)
    build_verify_fact_aql = staticmethod(build_verify_fact_aql)
    build_remember_aql = staticmethod(build_remember_aql)

    def health(self) -> dict[str, Any]:
        return self._request("GET", "/v1/health", b"")

    def health_response(self) -> HealthResponse:
        return HealthResponse.from_json(self.health())

    def put_cell(self, cell_id: int, payload: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/cell", cell_id=cell_id), payload.encode())

    def put_cell_response(self, cell_id: int, payload: str) -> PutCellResponse:
        return PutCellResponse.from_json(self.put_cell(cell_id, payload))

    def get_cell(self, cell_id: int) -> dict[str, Any]:
        return self._request("GET", self._path("/v1/cell", cell_id=cell_id), b"")

    def get_cell_response(self, cell_id: int) -> CellLookupResponse:
        return CellLookupResponse.from_json(self.get_cell(cell_id))

    def tombstone_cell(self, cell_id: int) -> dict[str, Any]:
        return self._request("DELETE", self._path("/v1/cell", cell_id=cell_id), b"")

    def flush(self) -> dict[str, Any]:
        return self._request("POST", "/v1/flush", b"")

    def compact(self) -> dict[str, Any]:
        return self._request("POST", "/v1/compact", b"")

    def search(self, scope: str, query: str, limit: int = 20) -> dict[str, Any]:
        path = self._path("/v1/search", scope=scope, mode="keyword", q=query, limit=limit)
        return self._request("POST", path, b"")

    def search_response(self, scope: str, query: str, limit: int = 20) -> SearchResponse:
        return SearchResponse.from_json(self.search(scope, query, limit))

    def search_vector(
        self,
        scope: str,
        vector: list[int] | tuple[int, ...],
        limit: int = 20,
        algorithm: str = "ann",
    ) -> dict[str, Any]:
        literal = ",".join(str(value) for value in vector)
        path = self._path(
            "/v1/search",
            scope=scope,
            mode="vector",
            algorithm=algorithm,
            vector=literal,
            limit=limit,
        )
        return self._request("POST", path, b"")

    def search_vector_response(
        self,
        scope: str,
        vector: list[int] | tuple[int, ...],
        limit: int = 20,
        algorithm: str = "ann",
    ) -> SearchResponse:
        return SearchResponse.from_json(self.search_vector(scope, vector, limit, algorithm))

    def evaluate_ann(
        self,
        scope: str,
        vector: list[int] | tuple[int, ...],
        limit: int = 20,
    ) -> dict[str, Any]:
        literal = ",".join(str(value) for value in vector)
        path = self._path(
            "/v1/search/ann-evaluate",
            scope=scope,
            vector=literal,
            limit=limit,
        )
        return self._request("POST", path, b"")

    def evaluate_ann_response(
        self,
        scope: str,
        vector: list[int] | tuple[int, ...],
        limit: int = 20,
    ) -> AnnEvaluationResponse:
        return AnnEvaluationResponse.from_json(self.evaluate_ann(scope, vector, limit))

    def aql(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/aql", scope=scope), statement.encode())

    def aql_response(self, scope: str, statement: str) -> AqlResponse:
        return AqlResponse.from_json(self.aql(scope, statement))

    def context(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/context", scope=scope), statement.encode())

    def context_response(self, scope: str, statement: str) -> ContextPackResponse:
        return ContextPackResponse.from_json(self.context(scope, statement))

    def answer_with_grounded_context(
        self,
        scope: str,
        brain: str,
        question: str,
        answerer: Callable[[ContextPackResponse], str],
        *,
        mode: str | None = "balanced",
        budget_tokens: int | None = None,
        limit_candidates: int | None = None,
        where_clause: str | None = None,
        require_citations: bool = True,
        reject_unsupported: bool = False,
        verify_answer: bool = True,
    ) -> GroundedAnswerResponse:
        retrieve_statement = build_retrieve_context_aql(
            question,
            brain,
            mode=mode,
            budget_tokens=budget_tokens,
            limit_candidates=limit_candidates,
            where_clause=where_clause,
            require_citations=require_citations,
        )
        context = self.context_response(scope, retrieve_statement)
        answer = answerer(context)
        verify_statement = (
            build_verify_fact_aql(answer, brain)
            if verify_answer and answer.strip()
            else None
        )
        verification = (
            self.verify_response(scope, verify_statement)
            if verify_statement is not None
            else None
        )
        return _grounded_answer_response(
            question=question,
            answer=answer,
            retrieve_statement=retrieve_statement,
            verify_statement=verify_statement,
            context=context,
            verification=verification,
            require_citations=require_citations,
            reject_unsupported=reject_unsupported,
        )

    def verify(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/verify", scope=scope), statement.encode())

    def verify_response(self, scope: str, statement: str) -> VerificationReportResponse:
        return VerificationReportResponse.from_json(self.verify(scope, statement))

    def remember(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/remember", scope=scope), statement.encode())

    def remember_response(self, scope: str, statement: str) -> RememberResponse:
        return RememberResponse.from_json(self.remember(scope, statement))

    def ingest_text(
        self,
        scope: str,
        text: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/text", scope=scope, source=source)
        return self._request("POST", path, text.encode())

    def ingest_text_response(self, scope: str, text: str, source: str = "python_sdk") -> IngestResponse:
        return IngestResponse.from_json(self.ingest_text(scope, text, source))

    def ingest_json(
        self,
        scope: str,
        document: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/json", scope=scope, source=source)
        return self._request("POST", path, document.encode())

    def ingest_json_response(self, scope: str, document: str, source: str = "python_sdk") -> IngestResponse:
        return IngestResponse.from_json(self.ingest_json(scope, document, source))

    def ingest_csv(
        self,
        scope: str,
        document: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/csv", scope=scope, source=source)
        return self._request("POST", path, document.encode())

    def ingest_csv_response(self, scope: str, document: str, source: str = "python_sdk") -> IngestResponse:
        return IngestResponse.from_json(self.ingest_csv(scope, document, source))

    def ingestion_job(self, job_id: int) -> dict[str, Any]:
        return self._request("GET", f"/v1/ingest/jobs/{job_id}", b"")

    def ingestion_job_response(self, job_id: int) -> IngestionJobResponse:
        return IngestionJobResponse.from_json(self.ingestion_job(job_id))

    def delete_ingestion_job(self, job_id: int) -> DeleteJobResponse:
        return DeleteJobResponse.from_json(
            self._request("DELETE", f"/v1/ingest/jobs/{job_id}", b"")
        )

    def retry_ingestion_job(self, job_id: int) -> IngestionJobResponse:
        return IngestionJobResponse.from_json(
            self._request("POST", f"/v1/ingest/jobs/{job_id}/retry", b"")
        )

    def validate(self) -> dict[str, Any]:
        return self._request("GET", "/v1/validate", b"")

    def validate_response(self) -> ValidationResponse:
        return ValidationResponse.from_json(self.validate())

    def stats(self) -> dict[str, Any]:
        return self._request("GET", "/v1/stats", b"")

    def stats_response(self) -> StatsResponse:
        return StatsResponse.from_json(self.stats())

    def _request(self, method: str, path: str, body: bytes) -> dict[str, Any]:
        headers = {"content-type": "application/json"}
        if self.token:
            headers["authorization"] = f"Bearer {self.token}"
        url = f"{self.base_url}{self._scoped(path)}"
        attempt = 0
        while True:
            request = urllib.request.Request(url, data=body or None, headers=headers, method=method)
            try:
                with urllib.request.urlopen(request, timeout=10) as response:
                    return json.loads(response.read().decode())
            except urllib.error.HTTPError as e:
                body_text = e.read().decode()
                if attempt < self.max_retries and self._is_retryable(e.code):
                    attempt += 1
                    time.sleep(self.retry_delay_seconds * attempt)
                    continue
                raise CortexDBError.from_response(e.code, body_text) from None
            except urllib.error.URLError as e:
                if attempt < self.max_retries:
                    attempt += 1
                    time.sleep(self.retry_delay_seconds * attempt)
                    continue
                raise CortexDBError(str(e.reason), code=None, status=None, body=str(e.reason)) from None

    @staticmethod
    def _is_retryable(status: int) -> bool:
        return status in (500, 502, 503, 504)

    def _scoped(self, path: str) -> str:
        if not self.tenant or self.tenant == "default":
            return path
        separator = "&" if "?" in path else "?"
        encoded = urllib.parse.urlencode({"tenant": self.tenant})
        return f"{path}{separator}{encoded}"

    @staticmethod
    def _path(path: str, **query: object) -> str:
        encoded = urllib.parse.urlencode({key: str(value) for key, value in query.items()})
        return f"{path}?{encoded}" if encoded else path
