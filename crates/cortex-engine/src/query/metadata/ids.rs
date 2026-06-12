use cortex_aql::{BitmapHandle, CellTypeId, MemoryType, ScopeId, StatusId};

const SCOPE_NS: u64 = 0x1000_0000_0000_0000;
const STATUS_NS: u64 = 0x2000_0000_0000_0000;
const TYPE_NS: u64 = 0x3000_0000_0000_0000;
const MEMORY_NS: u64 = 0x4000_0000_0000_0000;

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
