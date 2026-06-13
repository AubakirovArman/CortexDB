#!/usr/bin/env python3
"""Guard AQL retrieval against rebuilding indexes from cloned snapshots."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
QUERY = ROOT / "crates/cortex-engine/src/query.rs"
INDEX = ROOT / "crates/cortex-engine/src/query/index.rs"
INDEX_MERGE = ROOT / "crates/cortex-engine/src/query/index_merge.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    query = QUERY.read_text()
    index = INDEX.read_text()
    index_merge = INDEX_MERGE.read_text()

    require(
        index,
        "pub(crate) fn try_from_version_refs",
        "borrowed AQL index builder",
    )
    require(
        index_merge,
        "pub(crate) fn from_persisted_refs",
        "borrowed persisted/delta AQL index merge",
    )
    require(
        index_merge,
        "pub(crate) fn from_persisted_delta",
        "maintained delta AQL index merge",
    )
    require(
        query,
        "EngineAqlIndex::try_from_delta(&self.aql_delta_index)",
        "empty-persisted AQL index from maintained delta index",
    )
    require(
        query,
        "EngineAqlIndex::from_persisted_delta(",
        "persisted AQL index merge from maintained delta index",
    )
    forbid(
        query,
        "snapshot_versions()",
        "query-time full snapshot clone in AQL index path",
    )
    forbid(
        query,
        "memtable.visible_iter",
        "query-time MemTable scan in AQL index path",
    )
    forbid(
        query,
        "changed_cell_ids_after",
        "query-time changed-cell scan in AQL index path",
    )

    print("indexed retrieve gate passed")


if __name__ == "__main__":
    main()
