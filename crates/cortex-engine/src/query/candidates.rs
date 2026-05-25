use std::collections::BTreeMap;

use cortex_core::CellId;

use crate::error::{EngineError, EngineResult};

pub(super) fn reverse_candidate_map(
    candidate_to_cell: &BTreeMap<u32, CellId>,
) -> EngineResult<BTreeMap<CellId, u32>> {
    let mut cell_to_candidate = BTreeMap::new();
    for (candidate, cell_id) in candidate_to_cell {
        validate_candidate(*candidate)?;
        if cell_to_candidate.insert(*cell_id, *candidate).is_some() {
            return Err(EngineError::StorageInvariant(format!(
                "cell {} maps to multiple candidates",
                cell_id.0
            )));
        }
    }
    Ok(cell_to_candidate)
}

pub(super) fn candidate_from_ordinal(index: usize) -> EngineResult<u32> {
    let one_based = index
        .checked_add(1)
        .ok_or(EngineError::CandidateIdOverflow)?;
    let candidate = u32::try_from(one_based).map_err(|_| EngineError::CandidateIdOverflow)?;
    validate_candidate(candidate)?;
    Ok(candidate)
}

pub(super) fn next_candidate_after(candidates: impl IntoIterator<Item = u32>) -> EngineResult<u32> {
    let max = candidates.into_iter().max().unwrap_or(0);
    if max == 0 {
        Ok(1)
    } else {
        increment_candidate(max)
    }
}

pub(super) fn increment_candidate(candidate: u32) -> EngineResult<u32> {
    candidate
        .checked_add(1)
        .filter(|value| *value != 0)
        .ok_or(EngineError::CandidateIdOverflow)
}

pub(super) fn validate_candidate(candidate: u32) -> EngineResult<()> {
    if candidate == 0 {
        Err(EngineError::InvalidCandidateId(candidate))
    } else {
        Ok(())
    }
}
