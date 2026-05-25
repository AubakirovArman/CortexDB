from __future__ import annotations

import json
import urllib.request
from dataclasses import dataclass


@dataclass(frozen=True)
class CortexDBClient:
    base_url: str = "http://127.0.0.1:8181"
    token: str | None = None

    def put_cell(self, cell_id: int, payload: str) -> dict:
        return self._request("POST", f"/v1/cell?cell_id={cell_id}", payload.encode())

    def get_cell(self, cell_id: int) -> dict:
        return self._request("GET", f"/v1/cell?cell_id={cell_id}", b"")

    def search(self, scope: str, query: str, limit: int = 20) -> dict:
        body = json.dumps({"scope": scope, "query": query, "limit": limit}).encode()
        return self._request("POST", "/v1/search", body)

    def validate(self) -> dict:
        return self._request("GET", "/v1/validate", b"")

    def stats(self) -> dict:
        return self._request("GET", "/v1/stats", b"")

    def _request(self, method: str, path: str, body: bytes) -> dict:
        headers = {"content-type": "application/json"}
        if self.token:
            headers["authorization"] = f"Bearer {self.token}"
        request = urllib.request.Request(
            f"{self.base_url}{path}", data=body or None, headers=headers, method=method
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.loads(response.read().decode())
