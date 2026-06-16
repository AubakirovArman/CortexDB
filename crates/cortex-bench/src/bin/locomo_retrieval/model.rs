use std::collections::BTreeMap;

use cortex_core::CellId;

#[derive(Default)]
pub struct TurnIndex {
    pub by_cell: BTreeMap<CellId, TurnMeta>,
}

pub struct TurnMeta {
    pub sample_id: String,
    pub session: String,
    pub date: String,
    pub dia_id: String,
    pub speaker: String,
    pub text: String,
}
