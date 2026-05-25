#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecordType {
    PutCellBatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecord {
    pub record_type: WalRecordType,
    pub payload: Vec<u8>,
}
