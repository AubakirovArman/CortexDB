#!/usr/bin/env python3
"""Guard AQL retrieval against rebuilding indexes from cloned snapshots."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
QUERY = ROOT / "crates/cortex-engine/src/query.rs"
INDEX = ROOT / "crates/cortex-engine/src/query/index.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    query = QUERY.read_text()
    index = INDEX.read_text()

    require(
        index,
        "pub(crate) fn try_from_version_refs",
        "borrowed AQL index builder",
    )
    require(
        index,
        "pub(crate) fn from_persisted_refs",
        "borrowed persisted/delta AQL index merge",
    )
    require(
        query,
        "EngineAqlIndex::try_from_version_refs(self.memtable.visible_iter(txn))",
        "empty-persisted AQL index from borrowed MemTable iterator",
    )
    require(
        query,
        "EngineAqlIndex::from_persisted_refs(",
        "persisted AQL index merge from borrowed MemTable iterator",
    )
    forbid(
        query,
        "snapshot_versions()",
        "query-time full snapshot clone in AQL index path",
    )

    print("indexed retrieve gate passed")


if __name__ == "__main__":
    main()
