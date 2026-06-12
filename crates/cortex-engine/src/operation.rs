use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::wal::{DecodedWalRecord, SectionTag, WalRecord, WalRecordType, WalSection};

use crate::error::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbOperationKind {
    PutCell,
    PatchCell,
    TombstoneCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbOperation {
    PutCell { cell_id: CellId, payload: Vec<u8> },
    PatchCell { cell_id: CellId, payload: Vec<u8> },
    TombstoneCell { cell_id: CellId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedDbOperation {
    pub seq: CommitSeq,
    pub operation: DbOperation,
    pub descriptor: Option<CellDescriptor>,
}

#[derive(Clone, Debug)]
pub struct OperationEncoder;

#[derive(Clone, Debug)]
pub struct OperationDecoder;

impl DbOperation {
    pub fn kind(&self) -> DbOperationKind {
        match self {
            Self::PutCell { .. } => DbOperationKind::PutCell,
            Self::PatchCell { .. } => DbOperationKind::PatchCell,
            Self::TombstoneCell { .. } => DbOperationKind::TombstoneCell,
        }
    }
}

impl OperationEncoder {
    pub fn encode(operation: &DbOperation) -> EngineResult<WalRecord> {
        wal_record_from_operation(operation)
    }

    pub fn encode_with_seq(seq: CommitSeq, operation: &DbOperation) -> WalRecord {
        wal_record_from_operation_with_seq(seq, operation)
    }
}

impl OperationDecoder {
    pub fn decode(record: &DecodedWalRecord) -> EngineResult<DbOperation> {
        operation_from_decoded_wal_record(record)
    }

    pub fn decode_with_seq(record: &DecodedWalRecord) -> EngineResult<DecodedDbOperation> {
        decoded_operation_from_wal_record(record)
    }
}

pub fn wal_record_from_operation(_operation: &DbOperation) -> EngineResult<WalRecord> {
    Err(EngineError::MissingCommitSeq)
}

pub fn wal_record_from_operation_with_seq(seq: CommitSeq, operation: &DbOperation) -> WalRecord {
    wal_record_from_operation_inner(seq, operation)
}

pub fn wal_record_from_operation_with_metadata(
    seq: CommitSeq,
    operation: &DbOperation,
    metadata: Vec<u8>,
) -> WalRecord {
    let mut record = wal_record_from_operation_inner(seq, operation);
    if !metadata.is_empty() {
        let descriptor = CellDescriptor::from_metadata_section_lossy(&metadata);
        if let Some(section) = record
            .sections
            .iter_mut()
            .find(|section| section.tag == SectionTag::CellDescriptor)
        {
            section.data = descriptor.encode_section_v1();
        } else {
            record.sections.push(WalSection::new(
                SectionTag::CellDescriptor,
                descriptor.encode_section_v1(),
            ));
        }
        record
            .sections
            .push(WalSection::new(SectionTag::CellMetadata, metadata));
    }
    record
}

fn wal_record_from_operation_inner(seq: CommitSeq, operation: &DbOperation) -> WalRecord {
    match operation {
        DbOperation::PutCell { cell_id, payload } => {
            record_with_payload(WalRecordType::PutCellBatch, *cell_id, seq, payload.clone())
        }
        DbOperation::PatchCell { cell_id, payload } => record_with_payload(
            WalRecordType::PatchCellBatch,
            *cell_id,
            seq,
            payload.clone(),
        ),
        DbOperation::TombstoneCell { cell_id } => WalRecord::new(
            WalRecordType::TombstoneBatch,
            vec![WalSection::new(
                SectionTag::CellCore,
                encode_cell_core(*cell_id, seq),
            )],
        ),
    }
}

pub fn operation_from_decoded_wal_record(record: &DecodedWalRecord) -> EngineResult<DbOperation> {
    Ok(decoded_operation_from_wal_record(record)?.operation)
}

pub fn decoded_operation_from_wal_record(
    record: &DecodedWalRecord,
) -> EngineResult<DecodedDbOperation> {
    let cell_core = decode_cell_core(
        section(record, SectionTag::CellCore).ok_or(EngineError::MissingWalSection("CellCore"))?,
    )?;
    let seq = cell_core.seq.ok_or(EngineError::MissingCommitSeq)?;
    let operation = match record.record.record_type {
        WalRecordType::PutCellBatch => Ok(DbOperation::PutCell {
            cell_id: cell_core.cell_id,
            payload: payload(record)?,
        }),
        WalRecordType::PatchCellBatch => Ok(DbOperation::PatchCell {
            cell_id: cell_core.cell_id,
            payload: payload(record)?,
        }),
        WalRecordType::TombstoneBatch => Ok(DbOperation::TombstoneCell {
            cell_id: cell_core.cell_id,
        }),
        _ => Err(EngineError::InvalidOperation),
    }?;
    let descriptor = descriptor_from_decoded_wal_record(record)?;
    Ok(DecodedDbOperation {
        seq,
        operation,
        descriptor,
    })
}

pub fn encode_cell_id(cell_id: CellId) -> [u8; 8] {
    cell_id.0.to_le_bytes()
}

pub fn encode_cell_core(cell_id: CellId, seq: CommitSeq) -> Vec<u8> {
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&cell_id.0.to_le_bytes());
    out.extend_from_slice(&seq.0.to_le_bytes());
    out
}

pub fn decode_cell_id(bytes: &[u8]) -> EngineResult<CellId> {
    if bytes.len() < 8 {
        return Err(EngineError::InvalidOperation);
    }
    let raw: [u8; 8] = bytes[..8]
        .try_into()
        .map_err(|_| EngineError::InvalidOperation)?;
    Ok(CellId(u64::from_le_bytes(raw)))
}

pub fn metadata_from_decoded_wal_record(record: &DecodedWalRecord) -> Option<&[u8]> {
    section(record, SectionTag::CellMetadata)
}

pub fn descriptor_from_decoded_wal_record(
    record: &DecodedWalRecord,
) -> EngineResult<Option<CellDescriptor>> {
    let Some(bytes) = section(record, SectionTag::CellDescriptor) else {
        return Ok(None);
    };
    CellDescriptor::decode_section_v1(bytes)
        .map(Some)
        .ok_or(EngineError::InvalidOperation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedCellCore {
    pub cell_id: CellId,
    pub seq: Option<CommitSeq>,
}

pub fn decode_cell_core(bytes: &[u8]) -> EngineResult<DecodedCellCore> {
    let cell_id = decode_cell_id(bytes)?;
    let seq = if bytes.len() >= 16 {
        let raw: [u8; 8] = bytes[8..16]
            .try_into()
            .map_err(|_| EngineError::InvalidOperation)?;
        Some(CommitSeq(u64::from_le_bytes(raw)))
    } else {
        None
    };
    Ok(DecodedCellCore { cell_id, seq })
}

fn record_with_payload(
    record_type: WalRecordType,
    cell_id: CellId,
    seq: CommitSeq,
    payload: Vec<u8>,
) -> WalRecord {
    let descriptor = CellDescriptor::from_payload_lossy(&payload);
    WalRecord::new(
        record_type,
        vec![
            WalSection::new(SectionTag::CellCore, encode_cell_core(cell_id, seq)),
            WalSection::new(SectionTag::PayloadInline, payload),
            WalSection::new(SectionTag::CellDescriptor, descriptor.encode_section_v1()),
        ],
    )
}

fn payload(record: &DecodedWalRecord) -> EngineResult<Vec<u8>> {
    section(record, SectionTag::PayloadInline)
        .map(ToOwned::to_owned)
        .ok_or(EngineError::MissingWalSection("PayloadInline"))
}

fn section(record: &DecodedWalRecord, tag: SectionTag) -> Option<&[u8]> {
    record
        .sections
        .iter()
        .find(|section| section.tag == Some(tag))
        .map(|section| section.data.as_slice())
}
