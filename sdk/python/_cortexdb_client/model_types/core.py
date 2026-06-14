from __future__ import annotations

from dataclasses import dataclass
from typing import Any


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
class AqlQueryCacheStatsResponse:
    entries: int
    max_entries: int
    hits: int
    misses: int
    evictions: int
    catalog_invalidations: int
    hit_rate_q16: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "AqlQueryCacheStatsResponse":
        return cls(
            entries=int(value.get("entries", 0)),
            max_entries=int(value.get("max_entries", 0)),
            hits=int(value.get("hits", 0)),
            misses=int(value.get("misses", 0)),
            evictions=int(value.get("evictions", 0)),
            catalog_invalidations=int(value.get("catalog_invalidations", 0)),
            hit_rate_q16=int(value.get("hit_rate_q16", 0)),
        )


@dataclass(frozen=True)
class StatsResponse:
    current_seq: int
    checkpoint_seq: int
    live_segments: int
    retired_segments: int
    memtable_cells: int
    memtable_versions: int
    memtable_payload_bytes: int
    estimated_memtable_bytes: int
    estimated_index_bytes: int
    estimated_context_pack_bytes: int
    estimated_total_memory_bytes: int
    live_segment_bytes: int
    retired_segment_bytes: int
    total_segment_bytes: int
    durable_storage_bytes: int
    live_segment_payload_bytes: int
    logical_payload_bytes: int
    space_amplification_q16: int
    write_amplification_q16: int
    compaction_pressure_q16: int
    wal_size_bytes: int
    wal_writer_records: int
    wal_writer_bytes: int
    wal_writer_fsyncs: int
    wal_writer_batches: int
    aql_query_cache: AqlQueryCacheStatsResponse

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "StatsResponse":
        return cls(
            current_seq=int(value["current_seq"]),
            checkpoint_seq=int(value["checkpoint_seq"]),
            live_segments=int(value["live_segments"]),
            retired_segments=int(value["retired_segments"]),
            memtable_cells=int(value["memtable_cells"]),
            memtable_versions=int(value["memtable_versions"]),
            memtable_payload_bytes=int(value.get("memtable_payload_bytes", 0)),
            estimated_memtable_bytes=int(value.get("estimated_memtable_bytes", 0)),
            estimated_index_bytes=int(value.get("estimated_index_bytes", 0)),
            estimated_context_pack_bytes=int(value.get("estimated_context_pack_bytes", 0)),
            estimated_total_memory_bytes=int(value.get("estimated_total_memory_bytes", 0)),
            live_segment_bytes=int(value.get("live_segment_bytes", 0)),
            retired_segment_bytes=int(value.get("retired_segment_bytes", 0)),
            total_segment_bytes=int(value.get("total_segment_bytes", 0)),
            durable_storage_bytes=int(value.get("durable_storage_bytes", 0)),
            live_segment_payload_bytes=int(value.get("live_segment_payload_bytes", 0)),
            logical_payload_bytes=int(value.get("logical_payload_bytes", 0)),
            space_amplification_q16=int(value.get("space_amplification_q16", 0)),
            write_amplification_q16=int(value.get("write_amplification_q16", 0)),
            compaction_pressure_q16=int(value.get("compaction_pressure_q16", 0)),
            wal_size_bytes=int(value["wal_size_bytes"]),
            wal_writer_records=int(value["wal_writer_records"]),
            wal_writer_bytes=int(value["wal_writer_bytes"]),
            wal_writer_fsyncs=int(value["wal_writer_fsyncs"]),
            wal_writer_batches=int(value["wal_writer_batches"]),
            aql_query_cache=AqlQueryCacheStatsResponse.from_json(
                value.get("aql_query_cache", {})
            ),
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
