use cortex_aql::{BitmapHandle, CellTypeId, MemoryType, ScopeId, StatusId};

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
        Self {
            scope,
            status,
            cell_type,
            memory_type,
            terms: tokenize(&text),
        }
    }
}

pub fn scope_id(name: &str) -> ScopeId {
    ScopeId(stable_hash(name))
}

pub(crate) fn status_id(name: &str) -> StatusId {
    StatusId(stable_hash(name))
}

pub(crate) fn cell_type_id(name: &str) -> CellTypeId {
    CellTypeId(stable_hash(name))
}

pub(crate) fn scope_handle(scope: ScopeId) -> BitmapHandle {
    BitmapHandle(SCOPE_NS | scope.0)
}

pub(crate) fn status_handle(status: StatusId) -> BitmapHandle {
    BitmapHandle(STATUS_NS | status.0)
}

pub(crate) fn cell_type_handle(cell_type: CellTypeId) -> BitmapHandle {
    BitmapHandle(TYPE_NS | cell_type.0)
}

pub(crate) fn memory_type_handle(memory_type: MemoryType) -> BitmapHandle {
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
