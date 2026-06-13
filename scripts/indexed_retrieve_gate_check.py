#!/usr/bin/env python3
"""Guard AQL retrieval against rebuilding indexes from cloned snapshots."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
QUERY = ROOT / "crates/cortex-engine/src/query.rs"
INDEX = ROOT / "crates/cortex-engine/src/query/index.rs"
INDEX_MERGE = ROOT / "crates/cortex-engine/src/query/index_merge.rs"
RETRIEVAL_RANK = ROOT / "crates/cortex-engine/src/retrieval_rank.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def require_order(text: str, labels: list[tuple[str, str]]) -> None:
    positions: list[tuple[int, str, str]] = []
    for needle, label in labels:
        position = text.find(needle)
        if position < 0:
            raise SystemExit(f"missing {label}: {needle}")
        positions.append((position, needle, label))
    for previous, current in zip(positions, positions[1:]):
        if previous[0] >= current[0]:
            raise SystemExit(
                f"expected {previous[2]} before {current[2]}: "
                f"{previous[1]} before {current[1]}"
            )


def main() -> None:
    query = QUERY.read_text()
    index = INDEX.read_text()
    index_merge = INDEX_MERGE.read_text()
    retrieval_rank = RETRIEVAL_RANK.read_text()

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
    require_order(
        retrieval_rank,
        [
            ("let metadata = cells", "rank metadata precompute"),
            ("let lexical_scores = lexical_bm25_scores_from_metadata", "rank lexical precompute"),
            ("let recency_scores = recency_scores_q16_from_metadata", "rank recency precompute"),
            ("let rank_keys = cells", "rank key precompute"),
            ("indexed.sort_by_key(|((score, index), _)|", "rank sort by precomputed key"),
        ],
    )
    sort_site = retrieval_rank.find("indexed.sort_by_key(|((score, index), _)|")
    sort_window = retrieval_rank[sort_site : sort_site + 240]
    forbid(sort_window, ".metadata()", "metadata parsing in retrieval sort key")
    forbid(sort_window, "semantic_dot_score", "semantic scoring in retrieval sort key")
    forbid(sort_window, "lexical_bm25_scores", "lexical scoring in retrieval sort key")

    print("indexed retrieve gate passed")


if __name__ == "__main__":
    main()
