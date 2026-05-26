from __future__ import annotations

import json
import urllib.parse
import urllib.request
from typing import Any
from dataclasses import dataclass


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
