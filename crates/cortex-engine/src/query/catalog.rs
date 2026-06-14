use std::collections::BTreeSet;

use cortex_aql::{
    AqlCatalog, BitmapHandle, BitmapProvider, BrainId, CellTypeId, MemoryType, RoaringBitmap,
    ScopeId, StatusId,
};
use cortex_core::CellId;

use super::metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, scope_id, status_handle,
    status_id,
};
use super::{EngineAqlIndex, DEFAULT_BRAIN};
use crate::database::CandidateResolver;

impl AqlCatalog for EngineAqlIndex {
    fn resolve_brain(&self, _name: &str) -> Option<BrainId> {
        Some(DEFAULT_BRAIN)
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        Some(scope_id(name))
    }

    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId> {
        Some(scope_id(name))
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        Some(status_id(status))
    }

    fn resolve_cell_type(&self, _brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        Some(cell_type_id(cell_type))
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        Some(scope_handle(scope))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        Some(status_handle(status))
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        Some(cell_type_handle(cell_type))
    }

    fn memory_type_bitmap(&self, _brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        Some(memory_type_handle(memory_type))
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(
            field,
            "space" | "scope" | "status" | "type" | "cell_type" | "memory_type"
        )
    }

    fn bitmap_estimated_cardinality(&self, _brain: BrainId, handle: BitmapHandle) -> Option<u64> {
        self.bitmaps.get(&handle).map(|values| values.len() as u64)
    }
}

impl BitmapProvider for EngineAqlIndex {
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        Some(set_to_bitmap(
            &self.bitmaps.get(&handle).cloned().unwrap_or_default(),
        ))
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        set_to_bitmap(&self.universe)
    }

    fn live(&self) -> RoaringBitmap {
        set_to_bitmap(&self.universe)
    }

    fn universe(&self) -> RoaringBitmap {
        set_to_bitmap(&self.universe)
    }
}

impl CandidateResolver for EngineAqlIndex {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId> {
        self.candidate_to_cell.get(&candidate).copied()
    }
}

fn set_to_bitmap(values: &BTreeSet<u32>) -> RoaringBitmap {
    values.iter().copied().collect()
}
