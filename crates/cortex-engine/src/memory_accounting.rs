use std::collections::{BTreeMap, BTreeSet};
use std::mem::{size_of, size_of_val};

use cortex_core::memtable::{CellVersion, MemTableStats};
use cortex_core::CellId;

use crate::context::{ContextExplain, ContextPackCell, ContextScoreComponent};
use crate::database::{Database, RetrievedCell};
use crate::error::EngineResult;
use crate::query::EngineAqlIndex;

const BTREE_ENTRY_OVERHEAD_BYTES: usize = 48;
const STRING_HEAP_OVERHEAD_BYTES: usize = 24;
const CONTEXT_EXPLAIN_STRING_BUDGET_BYTES: usize = 192;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EngineMemoryEstimate {
    pub memtable_payload_bytes: usize,
    pub estimated_memtable_bytes: usize,
    pub estimated_index_bytes: usize,
    pub estimated_context_pack_bytes: usize,
    pub estimated_total_memory_bytes: usize,
}

pub(crate) fn estimate_database_memory(db: &Database) -> EngineResult<EngineMemoryEstimate> {
    let memtable = db.memtable.stats();
    let index = db.try_aql_index()?;
    let memtable_payload_bytes = memtable.payload_bytes;
    let estimated_memtable_bytes = estimate_memtable_bytes(memtable);
    let estimated_index_bytes = estimate_aql_index_bytes(&index);
    let estimated_context_pack_bytes =
        estimate_context_pack_working_set_bytes(db.memtable.visible_iter(db.read_txn()));
    Ok(EngineMemoryEstimate {
        memtable_payload_bytes,
        estimated_memtable_bytes,
        estimated_index_bytes,
        estimated_context_pack_bytes,
        estimated_total_memory_bytes: estimated_memtable_bytes
            .saturating_add(estimated_index_bytes)
            .saturating_add(estimated_context_pack_bytes),
    })
}

fn estimate_memtable_bytes(stats: MemTableStats) -> usize {
    stats
        .payload_bytes
        .saturating_add(size_of::<BTreeMap<CellId, Vec<CellVersion>>>())
        .saturating_add(
            stats.cell_count.saturating_mul(
                size_of::<CellId>()
                    .saturating_add(size_of::<Vec<CellVersion>>())
                    .saturating_add(BTREE_ENTRY_OVERHEAD_BYTES),
            ),
        )
        .saturating_add(stats.version_count.saturating_mul(
            size_of::<CellVersion>().saturating_add(BTREE_ENTRY_OVERHEAD_BYTES / 2),
        ))
}

fn estimate_aql_index_bytes(index: &EngineAqlIndex) -> usize {
    size_of::<EngineAqlIndex>()
        .saturating_add(estimate_bitmap_maps(index))
        .saturating_add(estimate_lexical_maps(index))
        .saturating_add(estimate_set(&index.universe))
        .saturating_add(estimate_map_len(
            index.candidate_to_cell.len(),
            size_of::<u32>().saturating_add(size_of::<CellId>()),
        ))
        .saturating_add(estimate_map_len(
            index.cell_to_candidate.len(),
            size_of::<CellId>().saturating_add(size_of::<u32>()),
        ))
}

fn estimate_bitmap_maps(index: &EngineAqlIndex) -> usize {
    index
        .bitmaps
        .iter()
        .fold(size_of_val(&index.bitmaps), |total, (handle, values)| {
            total
                .saturating_add(size_of_val(handle))
                .saturating_add(BTREE_ENTRY_OVERHEAD_BYTES)
                .saturating_add(estimate_set(values))
        })
}

fn estimate_lexical_maps(index: &EngineAqlIndex) -> usize {
    let lexical_terms =
        index
            .lexical
            .iter()
            .fold(size_of_val(&index.lexical), |total, (term, values)| {
                total
                    .saturating_add(estimate_string(term))
                    .saturating_add(BTREE_ENTRY_OVERHEAD_BYTES)
                    .saturating_add(estimate_set(values))
            });
    let doc_lengths = estimate_map_len(
        index.lexical_doc_lengths.len(),
        size_of::<u32>().saturating_add(size_of::<u32>()),
    );
    let term_frequencies = index.lexical_term_frequencies.iter().fold(
        size_of_val(&index.lexical_term_frequencies),
        |total, (term, values)| {
            total
                .saturating_add(estimate_string(term))
                .saturating_add(BTREE_ENTRY_OVERHEAD_BYTES)
                .saturating_add(estimate_map_len(
                    values.len(),
                    size_of::<u32>().saturating_add(size_of::<u32>()),
                ))
        },
    );
    lexical_terms
        .saturating_add(doc_lengths)
        .saturating_add(term_frequencies)
}

fn estimate_context_pack_working_set_bytes<'a>(
    versions: impl Iterator<Item = &'a CellVersion>,
) -> usize {
    let mut count = 0usize;
    let mut payload_bytes = 0usize;
    for version in versions {
        count += 1;
        payload_bytes = payload_bytes.saturating_add(version.payload.len());
    }
    let per_cell = size_of::<RetrievedCell>()
        .saturating_add(size_of::<ContextPackCell>())
        .saturating_add(size_of::<ContextExplain>())
        .saturating_add(3usize.saturating_mul(size_of::<ContextScoreComponent>()))
        .saturating_add(CONTEXT_EXPLAIN_STRING_BUDGET_BYTES);
    payload_bytes
        .saturating_add(size_of::<Vec<RetrievedCell>>())
        .saturating_add(size_of::<Vec<ContextPackCell>>())
        .saturating_add(count.saturating_mul(per_cell))
}

fn estimate_set(values: &BTreeSet<u32>) -> usize {
    size_of_val(values).saturating_add(
        values
            .len()
            .saturating_mul(size_of::<u32>().saturating_add(BTREE_ENTRY_OVERHEAD_BYTES)),
    )
}

fn estimate_map_len(len: usize, entry_bytes: usize) -> usize {
    size_of::<BTreeMap<u32, u32>>()
        .saturating_add(len.saturating_mul(entry_bytes.saturating_add(BTREE_ENTRY_OVERHEAD_BYTES)))
}

fn estimate_string(value: &str) -> usize {
    size_of::<String>()
        .saturating_add(STRING_HEAP_OVERHEAD_BYTES)
        .saturating_add(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memtable_estimate_includes_payload_bytes() {
        let estimate = estimate_memtable_bytes(MemTableStats {
            cell_count: 2,
            live_cell_count: 2,
            version_count: 3,
            payload_bytes: 128,
            ..MemTableStats::default()
        });

        assert!(estimate > 128);
    }
}
