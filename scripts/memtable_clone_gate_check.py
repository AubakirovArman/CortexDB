#!/usr/bin/env python3
"""Guard cold checkpoint paths against reintroducing MemTable payload clones."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CHECKPOINT = ROOT / "crates/cortex-engine/src/checkpoint.rs"
MEMTABLE = ROOT / "crates/cortex-core/src/memtable/mod.rs"
SEGMENT = ROOT / "crates/cortex-storage/src/segment.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    checkpoint = CHECKPOINT.read_text()
    memtable = MEMTABLE.read_text()
    segment = SEGMENT.read_text()

    require(memtable, "pub fn visible_iter", "borrowed visible iterator")
    require(
        memtable,
        "pub fn visible_created_after_iter",
        "borrowed delta iterator",
    )
    require(segment, "pub struct SegmentCellRef", "borrowed segment cell view")
    require(segment, "pub fn write_refs", "borrowed segment writer")

    require(
        checkpoint,
        "SegmentWriter::write_refs",
        "checkpoint borrowed segment writer call",
    )
    require(
        checkpoint,
        "try_from_segment_cell_refs",
        "checkpoint borrowed AQL index builder call",
    )
    require(
        checkpoint,
        "vector_index_for_cell_refs",
        "checkpoint borrowed vector index builder call",
    )
    forbid(
        checkpoint,
        "self.snapshot_versions()",
        "checkpoint snapshot clone path",
    )
    forbid(
        checkpoint,
        "SegmentWriter::write(&segment_path",
        "owned segment writer in checkpoint/compact",
    )

    print("memtable clone gate passed")


if __name__ == "__main__":
    main()
