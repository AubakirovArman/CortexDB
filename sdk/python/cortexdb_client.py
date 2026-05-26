from __future__ import annotations

import json
import urllib.parse
import urllib.request
from typing import Any
from dataclasses import dataclass


@dataclass(frozen=True)
class AnnSearchReport:
    path: str
    fallback_reason: str | None
    requested_limit: int
    allowed_candidates: int
    graph_nodes: int
    returned_candidates: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AnnSearchReport":
        reason = value.get("fallback_reason")
        return cls(
            path=str(value["path"]),
            fallback_reason=str(reason) if reason is not None else None,
            requested_limit=int(value["requested_limit"]),
            allowed_candidates=int(value["allowed_candidates"]),
            graph_nodes=int(value["graph_nodes"]),
            returned_candidates=int(value["returned_candidates"]),
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
class CortexDBClient:
    base_url: str = "http://127.0.0.1:8181"
    token: str | None = None

    def health(self) -> dict[str, Any]:
        return self._request("GET", "/v1/health", b"")

    def put_cell(self, cell_id: int, payload: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/cell", cell_id=cell_id), payload.encode())

    def get_cell(self, cell_id: int) -> dict[str, Any]:
        return self._request("GET", self._path("/v1/cell", cell_id=cell_id), b"")

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

    def context(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/context", scope=scope), statement.encode())

    def verify(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/verify", scope=scope), statement.encode())

    def remember(self, scope: str, statement: str) -> dict[str, Any]:
        return self._request("POST", self._path("/v1/remember", scope=scope), statement.encode())

    def ingest_text(
        self,
        scope: str,
        text: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/text", scope=scope, source=source)
        return self._request("POST", path, text.encode())

    def ingest_json(
        self,
        scope: str,
        document: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/json", scope=scope, source=source)
        return self._request("POST", path, document.encode())

    def ingest_csv(
        self,
        scope: str,
        document: str,
        source: str = "python_sdk",
    ) -> dict[str, Any]:
        path = self._path("/v1/ingest/csv", scope=scope, source=source)
        return self._request("POST", path, document.encode())

    def ingestion_job(self, job_id: int) -> dict[str, Any]:
        return self._request("GET", f"/v1/ingest/jobs/{job_id}", b"")

    def validate(self) -> dict[str, Any]:
        return self._request("GET", "/v1/validate", b"")

    def stats(self) -> dict[str, Any]:
        return self._request("GET", "/v1/stats", b"")

    def _request(self, method: str, path: str, body: bytes) -> dict[str, Any]:
        headers = {"content-type": "application/json"}
        if self.token:
            headers["authorization"] = f"Bearer {self.token}"
        request = urllib.request.Request(
            f"{self.base_url}{path}", data=body or None, headers=headers, method=method
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.loads(response.read().decode())

    @staticmethod
    def _path(path: str, **query: object) -> str:
        encoded = urllib.parse.urlencode({key: str(value) for key, value in query.items()})
        return f"{path}?{encoded}" if encoded else path
