from __future__ import annotations

import json


class CortexDBError(Exception):
    """Typed exception raised for CortexDB HTTP errors."""

    def __init__(
        self,
        message: str,
        code: str | None = None,
        status: int | None = None,
        body: str | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.status = status
        self.body = body

    @classmethod
    def from_response(cls, status: int, body: str) -> "CortexDBError":
        try:
            data = json.loads(body)
            return cls(
                message=str(data.get("message", body)),
                code=str(data.get("code", "unknown")),
                status=status,
                body=body,
            )
        except (json.JSONDecodeError, KeyError):
            return cls(message=body, code=None, status=status, body=body)


