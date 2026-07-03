//! A4.2: deterministic temporal supersession at retrieval time.
//!
//! When a retrieval result contains two cells that state the same temporal fact
//! (same `TemporalFactKey` — subject + metric), only the newest should be
//! returned: an agent asking "what is Apollo's budget?" must not receive both
//! the stale and the current value. This reuses the *exact* key and date
//! semantics of the maintained temporal fact index (`temporal_fact_key`,
//! `temporal_date_for_record`), so retrieval-time supersession agrees with the
//! index by construction rather than re-deriving lineage.
//!
//! It is conservative: a stale cell is dropped only when a newer cell for the
//! same key is *also present in the result set*, so a query that surfaces only
//! the older cell still returns it. Ordering is the index's own —
//! `(as_of, created_unix_seconds, cell_id)` — and every input is data-derived, so
//! this is wall-clock-free and a rebuild is byte-identical (INV-3). Off by
//! default; a no-op for cells that carry no temporal fact key.
//!
//! Like `suppress_duplicate_content` (the DedupOp), this runs before parent
//! context expansion, so — only when enabled — a dropped stale fact that also
//! happens to be another retrieved cell's parent-expansion source takes its
//! parent context with it. This is a shared property of the suppress-before-
//! expand pipeline stage rather than specific to supersession.

use std::collections::BTreeMap;

use cortex_core::CellId;

use crate::database::RetrievedCell;
use crate::verification::{
    temporal_date_for_record, temporal_fact_key, TemporalDate, TemporalFactKey,
};

/// The index's "latest wins" ordering: greater is newer.
type SupersedeSortKey = (Option<TemporalDate>, Option<u64>, CellId);

/// Drops retrieved cells that a newer cell in the same result set supersedes on
/// the same temporal fact key. Cells without a temporal fact key are always kept.
pub(crate) fn suppress_superseded_cells(cells: Vec<RetrievedCell>) -> Vec<RetrievedCell> {
    if cells.len() <= 1 {
        return cells;
    }

    // Per-cell temporal fact key + sort key (None for cells with no fact key).
    let keyed: Vec<Option<(TemporalFactKey, SupersedeSortKey)>> = cells
        .iter()
        .map(|cell| {
            let metadata = cell.metadata();
            temporal_fact_key(&metadata).map(|key| {
                let sort = (
                    temporal_date_for_record(&metadata),
                    metadata.created_unix_seconds,
                    cell.cell_id,
                );
                (key, sort)
            })
        })
        .collect();

    // Winner (max sort key) per fact key. The sort key's third element is the
    // winning cell id, so a cell survives iff it is its key's winner.
    let mut winner: BTreeMap<TemporalFactKey, SupersedeSortKey> = BTreeMap::new();
    for (key, sort) in keyed.iter().flatten() {
        winner
            .entry(key.clone())
            .and_modify(|best| {
                if *sort > *best {
                    *best = *sort;
                }
            })
            .or_insert(*sort);
    }

    cells
        .into_iter()
        .zip(keyed)
        .filter(|(cell, keyed)| match keyed {
            Some((key, _)) => winner.get(key).is_none_or(|best| best.2 == cell.cell_id),
            None => true,
        })
        .map(|(cell, _)| cell)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(id: u64, as_of: &str, value: &str) -> RetrievedCell {
        // Mirrors the temporal fact index test fixtures: subject via `project=`,
        // `metric=`, and the temporal date via the `as_of=` header line.
        RetrievedCell::from_payload(
            cortex_core::CellId(id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nas_of={as_of}\n\nproject=Apollo\nmetric=budget\nvalue={value}"
            )
            .into_bytes(),
        )
    }

    fn plain(id: u64, body: &str) -> RetrievedCell {
        RetrievedCell::from_payload(
            cortex_core::CellId(id),
            format!("scope=x\nstatus=ready\n\n{body}").into_bytes(),
        )
    }

    #[test]
    fn newer_fact_supersedes_older_same_key() {
        let out = suppress_superseded_cells(vec![
            fact(1, "2025-01-01", "12000"),
            fact(2, "2025-06-01", "14000"),
        ]);
        assert_eq!(out.len(), 1, "stale duplicate dropped");
        assert_eq!(out[0].cell_id.0, 2, "the latest fact survives");
    }

    #[test]
    fn preserves_input_order_of_survivors() {
        // Newer fact appears first; an unrelated cell keeps its place.
        let out = suppress_superseded_cells(vec![
            fact(2, "2025-06-01", "14000"),
            plain(9, "unrelated body"),
            fact(1, "2025-01-01", "12000"),
        ]);
        let ids: Vec<u64> = out.iter().map(|c| c.cell_id.0).collect();
        assert_eq!(
            ids,
            vec![2, 9],
            "winner + unrelated kept, in original order"
        );
    }

    #[test]
    fn distinct_keys_are_not_superseded() {
        let a = RetrievedCell::from_payload(
            cortex_core::CellId(1),
            b"scope=s\nstatus=ready\ntype=fact\nas_of=2025-01-01\n\nproject=Apollo\nmetric=budget\nvalue=1"
                .to_vec(),
        );
        let b = RetrievedCell::from_payload(
            cortex_core::CellId(2),
            b"scope=s\nstatus=ready\ntype=fact\nas_of=2025-06-01\n\nproject=Apollo\nmetric=headcount\nvalue=2"
                .to_vec(),
        );
        let out = suppress_superseded_cells(vec![a, b]);
        assert_eq!(out.len(), 2, "different metrics are independent facts");
    }

    #[test]
    fn cells_without_a_fact_key_are_kept() {
        let out = suppress_superseded_cells(vec![plain(1, "just prose"), plain(2, "more prose")]);
        assert_eq!(out.len(), 2, "no temporal key -> no supersession");
    }

    #[test]
    fn lone_stale_cell_is_kept() {
        // Only the older fact is retrieved; nothing supersedes it here.
        let out = suppress_superseded_cells(vec![fact(1, "2025-01-01", "12000")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].cell_id.0, 1);
    }

    #[test]
    fn deterministic() {
        let build = || {
            vec![
                fact(1, "2025-01-01", "12000"),
                fact(2, "2025-06-01", "14000"),
                fact(3, "2025-03-01", "13000"),
            ]
        };
        let a: Vec<u64> = suppress_superseded_cells(build())
            .iter()
            .map(|c| c.cell_id.0)
            .collect();
        let b: Vec<u64> = suppress_superseded_cells(build())
            .iter()
            .map(|c| c.cell_id.0)
            .collect();
        assert_eq!(a, b);
        assert_eq!(
            a,
            vec![2],
            "only the newest of three same-key facts survives"
        );
    }
}
