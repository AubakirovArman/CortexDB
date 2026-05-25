#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecordType {
    PutCellBatch,
    PatchCellBatch,
    TombstoneBatch,
    Commit,
    Checkpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionTag {
    CellCore,
    PayloadInline,
    SourceRef,
    RedundancyMeta,
    NumericGuards,
    EdgeHints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalSection {
    pub tag: SectionTag,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecord {
    pub record_type: WalRecordType,
    pub sections: Vec<WalSection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedWalRecord {
    pub lsn: u64,
    pub record: WalRecord,
    pub bytes_consumed: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionEntry {
    pub tag_raw: u16,
    pub offset: u32,
    pub len: u32,
}

impl WalRecord {
    pub fn new(record_type: WalRecordType, sections: Vec<WalSection>) -> Self {
        Self {
            record_type,
            sections,
        }
    }
}

impl WalSection {
    pub fn new(tag: SectionTag, data: impl Into<Vec<u8>>) -> Self {
        Self {
            tag,
            data: data.into(),
        }
    }
}

impl WalRecordType {
    pub fn to_u16(self) -> u16 {
        match self {
            Self::PutCellBatch => 1,
            Self::PatchCellBatch => 2,
            Self::TombstoneBatch => 3,
            Self::Commit => 4,
            Self::Checkpoint => 5,
        }
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::PutCellBatch),
            2 => Some(Self::PatchCellBatch),
            3 => Some(Self::TombstoneBatch),
            4 => Some(Self::Commit),
            5 => Some(Self::Checkpoint),
            _ => None,
        }
    }
}

impl SectionTag {
    pub fn to_u16(self) -> u16 {
        match self {
            Self::CellCore => 1,
            Self::PayloadInline => 2,
            Self::SourceRef => 3,
            Self::RedundancyMeta => 4,
            Self::NumericGuards => 5,
            Self::EdgeHints => 6,
        }
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::CellCore),
            2 => Some(Self::PayloadInline),
            3 => Some(Self::SourceRef),
            4 => Some(Self::RedundancyMeta),
            5 => Some(Self::NumericGuards),
            6 => Some(Self::EdgeHints),
            _ => None,
        }
    }
}
