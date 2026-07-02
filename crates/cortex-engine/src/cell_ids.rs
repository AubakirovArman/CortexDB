use cortex_aql::AgentId;
use cortex_core::CellId;

pub(crate) const GENERIC_CELL_ID_FLOOR: u64 = 10_000;
pub(crate) const MEMORY_CELL_NAMESPACE: u64 = 0x8000_0000_0000_0000;
// Agent cell-id layout v1:
// agent-cell-id.v1.top-nibble-28-bit-slot-32-bit-sequence.
// Top-nibble namespaces reserve bits 63..60, leaving 28 agent bits with a 32-bit sequence.
// A true 31-bit session/feedback agent slot requires a new persisted schema
// version with an explicit migration or refuse-to-read gate.
pub(crate) const AGENT_CELL_ID_SLOT_MASK: u64 = 0x0fff_ffff;
pub(crate) const CELL_ID_SEQUENCE_MASK: u64 = 0xffff_ffff;
pub(crate) const MEMORY_AGENT_SLOT_MASK: u64 = AGENT_CELL_ID_SLOT_MASK;
pub(crate) const MEMORY_SEQUENCE_MASK: u64 = CELL_ID_SEQUENCE_MASK;

pub(crate) fn agent_cell_id_slot(agent_id: AgentId) -> Option<u64> {
    (agent_id.0 <= AGENT_CELL_ID_SLOT_MASK).then_some(agent_id.0)
}

pub(crate) fn namespaced_agent_cell_id(
    namespace: u64,
    agent_slot: u64,
    sequence: u64,
) -> Option<CellId> {
    if agent_slot > AGENT_CELL_ID_SLOT_MASK || sequence > CELL_ID_SEQUENCE_MASK {
        return None;
    }
    Some(CellId(namespace | (agent_slot << 32) | sequence))
}

pub(crate) fn memory_agent_slot(agent_id: AgentId) -> Option<u64> {
    agent_cell_id_slot(agent_id)
}

pub(crate) fn memory_cell_id(agent_slot: u64, sequence: u64) -> Option<CellId> {
    namespaced_agent_cell_id(MEMORY_CELL_NAMESPACE, agent_slot, sequence)
}

pub(crate) fn memory_sequence(cell_id: CellId, agent_slot: u64) -> Option<u64> {
    let slot = (cell_id.0 >> 32) & MEMORY_AGENT_SLOT_MASK;
    (cell_id.0 & MEMORY_CELL_NAMESPACE != 0 && slot == agent_slot)
        .then_some(cell_id.0 & MEMORY_SEQUENCE_MASK)
}
