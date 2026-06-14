use cortex_aql::AgentId;
use cortex_core::CellId;

pub(crate) const GENERIC_CELL_ID_FLOOR: u64 = 10_000;
pub(crate) const MEMORY_CELL_NAMESPACE: u64 = 0x8000_0000_0000_0000;
pub(crate) const MEMORY_AGENT_SLOT_MASK: u64 = 0x7fff_ffff;
pub(crate) const MEMORY_SEQUENCE_MASK: u64 = 0xffff_ffff;

pub(crate) fn memory_agent_slot(agent_id: AgentId) -> u64 {
    agent_id.0 & MEMORY_AGENT_SLOT_MASK
}

pub(crate) fn memory_cell_id(agent_slot: u64, sequence: u64) -> Option<CellId> {
    if agent_slot > MEMORY_AGENT_SLOT_MASK || sequence > MEMORY_SEQUENCE_MASK {
        return None;
    }
    Some(CellId(
        MEMORY_CELL_NAMESPACE | (agent_slot << 32) | sequence,
    ))
}

pub(crate) fn memory_sequence(cell_id: CellId, agent_slot: u64) -> Option<u64> {
    let slot = (cell_id.0 >> 32) & MEMORY_AGENT_SLOT_MASK;
    (cell_id.0 & MEMORY_CELL_NAMESPACE != 0 && slot == agent_slot)
        .then_some(cell_id.0 & MEMORY_SEQUENCE_MASK)
}
