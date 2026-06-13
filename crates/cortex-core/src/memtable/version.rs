use crate::cell::CellDescriptor;
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
    pub descriptor: CellDescriptor,
    pub payload: Vec<u8>,
    pub payload_ref: PayloadRef,
    pub delta_depth: u32,
    pub index_debt: IndexDebt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadRef {
    Inline,
    Segment {
        segment_id: u64,
        candidate_id: u32,
        offset: u64,
        len: u64,
        crc32c: u32,
    },
}

impl CellVersion {
    pub fn new(
        cell_id: CellId,
        created_seq: CommitSeq,
        payload: Vec<u8>,
        delta_depth: u32,
    ) -> Self {
        let descriptor = CellDescriptor::from_payload_lossy(&payload);
        Self::new_with_descriptor(cell_id, created_seq, payload, delta_depth, descriptor)
    }

    pub fn new_with_descriptor(
        cell_id: CellId,
        created_seq: CommitSeq,
        payload: Vec<u8>,
        delta_depth: u32,
        descriptor: CellDescriptor,
    ) -> Self {
        Self {
            cell_id,
            created_seq,
            deleted_seq: None,
            descriptor,
            payload,
            payload_ref: PayloadRef::Inline,
            delta_depth,
            index_debt: IndexDebt::default(),
        }
    }

    pub fn new_with_payload_ref(
        cell_id: CellId,
        created_seq: CommitSeq,
        descriptor: CellDescriptor,
        payload_ref: PayloadRef,
    ) -> Self {
        Self {
            cell_id,
            created_seq,
            deleted_seq: None,
            descriptor,
            payload: Vec::new(),
            payload_ref,
            delta_depth: 0,
            index_debt: IndexDebt::default(),
        }
    }

    pub fn resident_payload_len(&self) -> usize {
        self.payload.len()
    }

    pub fn is_payload_resident(&self) -> bool {
        matches!(self.payload_ref, PayloadRef::Inline)
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

#[cfg(test)]
mod tests {
    use crate::cell::KnowledgeCellType;
    use crate::memtable::CellVersion;
    use crate::types::{CellId, CommitSeq};

    #[test]
    fn cell_version_materializes_descriptor_from_payload_headers() {
        let version = CellVersion::new(
            CellId(7),
            CommitSeq(11),
            b"scope=project:alpha\nstatus=ready\ntype=document_block\nsource_trust_q16=50000\n\nbody".to_vec(),
            0,
        );

        assert_eq!(version.descriptor.scope, "project:alpha");
        assert_eq!(version.descriptor.status, "ready");
        assert_eq!(
            version.descriptor.cell_type,
            KnowledgeCellType::DocumentBlock
        );
        assert_eq!(version.descriptor.source_trust_q16, Some(50_000));
    }
}
