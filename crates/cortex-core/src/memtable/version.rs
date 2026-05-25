use crate::types::{CellId, CommitSeq};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndexDebt {
    pub bitmap: u32,
    pub lexical: u32,
    pub vector: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellVersion {
    pub cell_id: CellId,
    pub created_seq: CommitSeq,
    pub deleted_seq: Option<CommitSeq>,
    pub payload: Vec<u8>,
    pub delta_depth: u32,
    pub index_debt: IndexDebt,
}

impl CellVersion {
    pub fn new(
        cell_id: CellId,
        created_seq: CommitSeq,
        payload: Vec<u8>,
        delta_depth: u32,
    ) -> Self {
        Self {
            cell_id,
            created_seq,
            deleted_seq: None,
            payload,
            delta_depth,
            index_debt: IndexDebt::default(),
        }
    }

    pub fn visible_at(&self, read_seq: CommitSeq) -> bool {
        self.created_seq <= read_seq && self.deleted_seq.is_none_or(|deleted| deleted > read_seq)
    }
}

impl IndexDebt {
    pub fn total(self) -> u32 {
        self.bitmap + self.lexical + self.vector
    }
}
