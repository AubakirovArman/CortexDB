use crate::types::{BrainId, CellTypeId, MemoryType, ScopeId, StatusId};

use super::{AqlCatalog, BitmapHandle};

pub trait BrainCatalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId>;
}

pub trait ScopeCatalog {
    fn resolve_scope(&self, brain: BrainId, name: &str) -> Option<ScopeId>;
    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId>;
    fn scope_bitmap(&self, brain: BrainId, scope: ScopeId) -> Option<BitmapHandle>;
}

pub trait StatusCatalog {
    fn resolve_status(&self, brain: BrainId, status: &str) -> Option<StatusId>;
    fn status_bitmap(&self, brain: BrainId, status: StatusId) -> Option<BitmapHandle>;
}

pub trait CellTypeCatalog {
    fn resolve_cell_type(&self, brain: BrainId, cell_type: &str) -> Option<CellTypeId>;
    fn cell_type_bitmap(&self, brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle>;
}

impl<T: AqlCatalog + ?Sized> BrainCatalog for T {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        AqlCatalog::resolve_brain(self, name)
    }
}

impl<T: AqlCatalog + ?Sized> ScopeCatalog for T {
    fn resolve_scope(&self, brain: BrainId, name: &str) -> Option<ScopeId> {
        AqlCatalog::resolve_scope(self, brain, name)
    }

    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId> {
        AqlCatalog::resolve_write_scope(self, name)
    }

    fn scope_bitmap(&self, brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        AqlCatalog::scope_bitmap(self, brain, scope)
    }
}

impl<T: AqlCatalog + ?Sized> StatusCatalog for T {
    fn resolve_status(&self, brain: BrainId, status: &str) -> Option<StatusId> {
        AqlCatalog::resolve_status(self, brain, status)
    }

    fn status_bitmap(&self, brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        AqlCatalog::status_bitmap(self, brain, status)
    }
}

impl<T: AqlCatalog + ?Sized> CellTypeCatalog for T {
    fn resolve_cell_type(&self, brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        AqlCatalog::resolve_cell_type(self, brain, cell_type)
    }

    fn cell_type_bitmap(&self, brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        AqlCatalog::cell_type_bitmap(self, brain, cell_type)
    }
}

pub trait MemoryTypeCatalog {
    fn memory_type_bitmap(&self, brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle>;
}

impl<T: AqlCatalog + ?Sized> MemoryTypeCatalog for T {
    fn memory_type_bitmap(&self, brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        AqlCatalog::memory_type_bitmap(self, brain, memory_type)
    }
}
