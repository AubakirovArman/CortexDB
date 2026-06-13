#!/usr/bin/env python3
"""Guard cold checkpoint paths against reintroducing MemTable payload clones."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKPOINT_DATABASE = ROOT / "crates/cortex-engine/src/checkpoint/database.rs"
CHECKPOINT_COMPACTOR = ROOT / "crates/cortex-engine/src/checkpoint/compactor.rs"
MEMTABLE = ROOT / "crates/cortex-core/src/memtable/mod.rs"
SEGMENT = ROOT / "crates/cortex-storage/src/segment.rs"
VERIFY = ROOT / "crates/cortex-engine/src/verification.rs"
VERIFY_GRAPH = ROOT / "crates/cortex-engine/src/verification/graph.rs"
CONFLICT_INDEX = ROOT / "crates/cortex-engine/src/verification/conflict_index.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    checkpoint_database = CHECKPOINT_DATABASE.read_text()
    checkpoint_compactor = CHECKPOINT_COMPACTOR.read_text()
    checkpoint_hot_paths = checkpoint_database + "\n" + checkpoint_compactor
    memtable = MEMTABLE.read_text()
    segment = SEGMENT.read_text()
    verify = VERIFY.read_text()
    verify_graph = VERIFY_GRAPH.read_text()
    conflict_index = CONFLICT_INDEX.read_text()

    require(memtable, "pub fn visible_iter", "borrowed visible iterator")
    require(
        memtable,
        "pub fn visible_created_after_iter",
        "borrowed delta iterator",
    )
    require(segment, "pub struct SegmentCellRef", "borrowed segment cell view")
    require(segment, "pub fn write_refs", "borrowed segment writer")

    require(
        checkpoint_database,
        "SegmentWriter::write_refs",
        "checkpoint borrowed segment writer call",
    )
    require(
        checkpoint_compactor,
        "SegmentWriter::write_refs",
        "incremental compaction borrowed segment writer call",
    )
    require(
        checkpoint_hot_paths,
        "try_from_segment_cell_refs",
        "checkpoint/compaction borrowed AQL index builder call",
    )
    require(
        checkpoint_hot_paths,
        "vector_index_for_cell_refs",
        "checkpoint/compaction borrowed vector index builder call",
    )
    forbid(
        checkpoint_hot_paths,
        "self.snapshot_versions()",
        "checkpoint/compaction snapshot clone path",
    )
    forbid(
        checkpoint_hot_paths,
        "SegmentWriter::write(&segment_path",
        "owned segment writer in checkpoint/compact",
    )
    forbid(verify, "self.snapshot_versions()", "VERIFY FACT full clone scan")
    forbid(verify, "bind_aql_cached", "VERIFY FACT retrieval-index bind path")
    forbid(
        verify_graph,
        "conflicts_for_fact",
        "VERIFY graph enrichment full conflict-index scan",
    )
    forbid(
        conflict_index,
        "self.snapshot_versions()",
        "verification conflict index full clone scan",
    )
    forbid(
        conflict_index,
        "visible_iter",
        "verification conflict index query-time visible scan",
    )
    forbid(
        conflict_index,
        "payload_for_version",
        "verification conflict index query-time payload materialization scan",
    )
    require(
        conflict_index,
        "self.conflict_index_store.records(view)",
        "maintained conflict-index lookup",
    )

    print("memtable clone gate passed")


if __name__ == "__main__":
    main()
