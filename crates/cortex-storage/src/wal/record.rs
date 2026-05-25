pub const ACLOG_MAGIC: [u8; 8] = *b"ACLOGv0\0";
pub const WAL_RECORD_MAGIC: u32 = 0x4143_4c52;
pub const WAL_FORMAT_VERSION: u16 = 0;
pub const WAL_FILE_HEADER_LEN: usize = 16;
pub const WAL_RECORD_HEADER_LEN: usize = 32;
pub const WAL_SECTION_ENTRY_LEN: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalCodecVersion(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AclogMagic(pub [u8; 8]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalFlags(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalFileHeader {
    pub magic: AclogMagic,
    pub header_len: u16,
    pub version: WalCodecVersion,
    pub flags: WalFlags,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalRecordHeader {
    pub magic: u32,
    pub header_len: u16,
    pub record_type: WalRecordType,
    pub flags: WalFlags,
    pub lsn: u64,
    pub payload_len: u32,
    pub section_count: u16,
    pub payload_crc32c: u32,
    pub header_crc32c: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalRecordType {
    PutCellBatch,
    PatchCellBatch,
    TombstoneBatch,
    PutEdgeBatch,
    Checkpoint,
    ManifestSwitch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionTag {
    CellCore,
    PayloadInline,
    SourceRef,
    RedundancyMeta,
    NumericGuards,
    VectorRef,
    EdgeHints,
    CellMetadata,
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
    pub header: WalRecordHeader,
    pub record: WalRecord,
    pub sections: Vec<DecodedSection>,
    pub bytes_consumed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedSection {
    pub tag_raw: u16,
    pub tag: Option<SectionTag>,
    pub data: Vec<u8>,
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
            Self::PutEdgeBatch => 4,
            Self::Checkpoint => 5,
            Self::ManifestSwitch => 6,
        }
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::PutCellBatch),
            2 => Some(Self::PatchCellBatch),
            3 => Some(Self::TombstoneBatch),
            4 => Some(Self::PutEdgeBatch),
            5 => Some(Self::Checkpoint),
            6 => Some(Self::ManifestSwitch),
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
            Self::VectorRef => 6,
            Self::EdgeHints => 7,
            Self::CellMetadata => 8,
        }
    }

    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::CellCore),
            2 => Some(Self::PayloadInline),
            3 => Some(Self::SourceRef),
            4 => Some(Self::RedundancyMeta),
            5 => Some(Self::NumericGuards),
            6 => Some(Self::VectorRef),
            7 => Some(Self::EdgeHints),
            8 => Some(Self::CellMetadata),
            _ => None,
        }
    }
}
