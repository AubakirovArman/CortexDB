use std::collections::BTreeMap;

use cortex_core::CellId;

use crate::error::{EngineError, EngineResult};

pub(super) struct CandidateAllocator {
    existing: BTreeMap<CellId, u32>,
    next: u32,
}

impl CandidateAllocator {
    pub(super) fn new(existing: &BTreeMap<CellId, u32>) -> EngineResult<Self> {
        for candidate in existing.values() {
            validate_candidate(*candidate)?;
        }
        Ok(Self {
            existing: existing.clone(),
            next: next_candidate_after(existing.values().copied())?,
        })
    }

    pub(super) fn candidate_for(&mut self, cell_id: CellId) -> EngineResult<u32> {
        if let Some(candidate) = self.existing.get(&cell_id) {
            Ok(*candidate)
        } else {
            let candidate = self.next;
            self.next = increment_candidate(self.next)?;
            self.existing.insert(cell_id, candidate);
            Ok(candidate)
        }
    }
}

pub(super) fn candidate_from_ordinal(index: usize) -> EngineResult<u32> {
    let one_based = index
        .checked_add(1)
        .ok_or(EngineError::CandidateIdOverflow)?;
    let candidate = u32::try_from(one_based).map_err(|_| EngineError::CandidateIdOverflow)?;
    validate_candidate(candidate)?;
    Ok(candidate)
}

pub(super) fn segment_cell_count(len: usize) -> EngineResult<u32> {
    u32::try_from(len).map_err(|_| EngineError::CandidateIdOverflow)
}

fn next_candidate_after(candidates: impl IntoIterator<Item = u32>) -> EngineResult<u32> {
    let max = candidates.into_iter().max().unwrap_or(0);
    if max == 0 {
        Ok(1)
    } else {
        increment_candidate(max)
    }
}

fn increment_candidate(candidate: u32) -> EngineResult<u32> {
    candidate
        .checked_add(1)
        .filter(|value| *value != 0)
        .ok_or(EngineError::CandidateIdOverflow)
}

fn validate_candidate(candidate: u32) -> EngineResult<()> {
    if candidate == 0 {
        Err(EngineError::InvalidCandidateId(candidate))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_allocator_errors_on_overflow() {
        let existing = BTreeMap::from([(CellId(1), u32::MAX)]);
        assert!(matches!(
            CandidateAllocator::new(&existing),
            Err(EngineError::CandidateIdOverflow)
        ));
    }
}
