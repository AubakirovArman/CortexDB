use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    parse_aql, AgentView, AqlCatalog, Binder, BitmapHandle, BitmapProvider, BoundPlan, BrainId,
    CellTypeId, MemoryType, ScopeId, StatusId,
};
use cortex_core::memtable::CellVersion;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentCell;

use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};

const DEFAULT_BRAIN: BrainId = BrainId(1);
const SCOPE_NS: u64 = 0x1000_0000_0000_0000;
const STATUS_NS: u64 = 0x2000_0000_0000_0000;
const TYPE_NS: u64 = 0x3000_0000_0000_0000;
const MEMORY_NS: u64 = 0x4000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellMetadata {
    pub scope: String,
    pub status: String,
    pub cell_type: String,
    pub memory_type: Option<MemoryType>,
    pub terms: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct EngineAqlIndex {
    pub bitmaps: BTreeMap<BitmapHandle, BTreeSet<u32>>,
    pub lexical: BTreeMap<String, BTreeSet<u32>>,
    pub universe: BTreeSet<u32>,
}

impl Database {
    pub fn aql_index(&self) -> EngineAqlIndex {
        EngineAqlIndex::from_versions(&self.snapshot_versions())
    }

    pub fn retrieve_aql(&self, aql: &str, view: &AgentView) -> EngineResult<Vec<RetrievedCell>> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let index = self.aql_index();
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        match bound {
            BoundPlan::Retrieve(plan) => self.retrieve_cells(&plan, &index),
            _ => Err(EngineError::InvalidOperation),
        }
    }
}

impl EngineAqlIndex {
    pub fn from_versions(versions: &[CellVersion]) -> Self {
        Self::from_cells(versions.iter().map(|version| {
            (
                version.cell_id.0 as u32,
                version.payload.as_slice(),
                version.cell_id.0,
            )
        }))
    }

    pub fn from_segment_cells(cells: &[SegmentCell]) -> Self {
        Self::from_cells(
            cells
                .iter()
                .map(|cell| (cell.cell_id as u32, cell.payload.as_slice(), cell.cell_id)),
        )
    }

    pub fn bitmap_index(&self) -> BitmapIndex {
        BitmapIndex {
            bitmaps: self
                .bitmaps
                .iter()
                .map(|(handle, values)| (handle.0, values.clone()))
                .collect(),
        }
    }

    pub fn lexical_index(&self) -> LexicalIndex {
        LexicalIndex {
            terms: self.lexical.clone(),
        }
    }

    fn from_cells<'a>(cells: impl IntoIterator<Item = (u32, &'a [u8], u64)>) -> Self {
        let mut index = Self::default();
        for (candidate, payload, cell_id) in cells {
            let metadata = CellMetadata::from_payload(payload);
            index.universe.insert(candidate);
            index.push(scope_handle(scope_id(&metadata.scope)), candidate);
            index.push(status_handle(status_id(&metadata.status)), candidate);
            index.push(
                cell_type_handle(cell_type_id(&metadata.cell_type)),
                candidate,
            );
            if let Some(memory_type) = metadata.memory_type {
                index.push(memory_type_handle(memory_type), candidate);
            }
            index.push(BitmapHandle(cell_id), candidate);
            for term in metadata.terms {
                index.lexical.entry(term).or_default().insert(candidate);
            }
        }
        index
    }

    fn push(&mut self, handle: BitmapHandle, candidate: u32) {
        self.bitmaps.entry(handle).or_default().insert(candidate);
    }
}

impl CellMetadata {
    pub fn from_payload(payload: &[u8]) -> Self {
        let text = String::from_utf8_lossy(payload);
        let mut scope = "default".to_owned();
        let mut status = "ready".to_owned();
        let mut cell_type = "cell".to_owned();
        let mut memory_type = None;
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("scope=") {
                scope = value.trim().to_owned();
            } else if let Some(value) = line.strip_prefix("status=") {
                status = value.trim().to_owned();
            } else if let Some(value) = line.strip_prefix("type=") {
                cell_type = value.trim().to_owned();
            } else if let Some(value) = line.strip_prefix("memory_type=") {
                memory_type = value.trim().parse().ok();
            }
        }
        let terms = tokenize(&text);
        Self {
            scope,
            status,
            cell_type,
            memory_type,
            terms,
        }
    }
}

impl AqlCatalog for EngineAqlIndex {
    fn resolve_brain(&self, _name: &str) -> Option<BrainId> {
        Some(DEFAULT_BRAIN)
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
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
    fn bitmap(&self, handle: BitmapHandle) -> Option<BTreeSet<u32>> {
        self.bitmaps.get(&handle).cloned()
    }

    fn agent_allowed(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }

    fn live(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }

    fn universe(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }
}

pub fn scope_id(name: &str) -> ScopeId {
    ScopeId(stable_hash(name))
}

fn status_id(name: &str) -> StatusId {
    StatusId(stable_hash(name))
}

fn cell_type_id(name: &str) -> CellTypeId {
    CellTypeId(stable_hash(name))
}

fn scope_handle(scope: ScopeId) -> BitmapHandle {
    BitmapHandle(SCOPE_NS | scope.0)
}

fn status_handle(status: StatusId) -> BitmapHandle {
    BitmapHandle(STATUS_NS | status.0)
}

fn cell_type_handle(cell_type: CellTypeId) -> BitmapHandle {
    BitmapHandle(TYPE_NS | cell_type.0)
}

fn memory_type_handle(memory_type: MemoryType) -> BitmapHandle {
    BitmapHandle(MEMORY_NS | memory_type as u64)
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash & 0x0fff_ffff_ffff_ffff
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(|term| term.to_ascii_lowercase())
        .collect()
}
