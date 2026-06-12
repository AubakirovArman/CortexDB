from __future__ import annotations

from dataclasses import dataclass
from typing import Any


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

