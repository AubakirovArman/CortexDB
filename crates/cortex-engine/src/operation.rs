use cortex_core::CellId;
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
    pub fn encode(operation: &DbOperation) -> WalRecord {
        wal_record_from_operation(operation)
    }
}

impl OperationDecoder {
    pub fn decode(record: &DecodedWalRecord) -> EngineResult<DbOperation> {
        operation_from_decoded_wal_record(record)
    }
}

pub fn wal_record_from_operation(operation: &DbOperation) -> WalRecord {
    match operation {
        DbOperation::PutCell { cell_id, payload } => {
            record_with_payload(WalRecordType::PutCellBatch, *cell_id, payload.clone())
        }
        DbOperation::PatchCell { cell_id, payload } => {
            record_with_payload(WalRecordType::PatchCellBatch, *cell_id, payload.clone())
        }
        DbOperation::TombstoneCell { cell_id } => WalRecord::new(
            WalRecordType::TombstoneBatch,
            vec![WalSection::new(
                SectionTag::CellCore,
                encode_cell_id(*cell_id),
            )],
        ),
    }
}

pub fn operation_from_decoded_wal_record(record: &DecodedWalRecord) -> EngineResult<DbOperation> {
    let cell_id = decode_cell_id(
        section(record, SectionTag::CellCore).ok_or(EngineError::MissingWalSection("CellCore"))?,
    )?;
    match record.record.record_type {
        WalRecordType::PutCellBatch => Ok(DbOperation::PutCell {
            cell_id,
            payload: payload(record)?,
        }),
        WalRecordType::PatchCellBatch => Ok(DbOperation::PatchCell {
            cell_id,
            payload: payload(record)?,
        }),
        WalRecordType::TombstoneBatch => Ok(DbOperation::TombstoneCell { cell_id }),
        _ => Err(EngineError::InvalidOperation),
    }
}

pub fn encode_cell_id(cell_id: CellId) -> [u8; 8] {
    cell_id.0.to_le_bytes()
}

pub fn decode_cell_id(bytes: &[u8]) -> EngineResult<CellId> {
    let raw: [u8; 8] = bytes
        .try_into()
        .map_err(|_| EngineError::InvalidOperation)?;
    Ok(CellId(u64::from_le_bytes(raw)))
}

fn record_with_payload(record_type: WalRecordType, cell_id: CellId, payload: Vec<u8>) -> WalRecord {
    WalRecord::new(
        record_type,
        vec![
            WalSection::new(SectionTag::CellCore, encode_cell_id(cell_id)),
            WalSection::new(SectionTag::PayloadInline, payload),
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
