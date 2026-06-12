use crate::wal::{ACLOG_MAGIC, WAL_FORMAT_VERSION};

pub const SEGMENT_MAGIC: [u8; 4] = *b"ACS1";
pub const BITMAP_INDEX_MAGIC: [u8; 4] = *b"ACB0";
pub const LEXICAL_INDEX_MAGIC: [u8; 4] = *b"ACI3";
pub const LEGACY_LEXICAL_INDEX_MAGIC: [u8; 4] = *b"ACI0";
pub const LEGACY_LEXICAL_INDEX_V1_MAGIC: [u8; 4] = *b"ACI1";
pub const LEGACY_LEXICAL_INDEX_V2_MAGIC: [u8; 4] = *b"ACI2";
pub const VECTOR_INDEX_MAGIC: [u8; 4] = *b"ACV0";
pub const HNSW_GRAPH_MAGIC: [u8; 4] = *b"ACH0";
pub const MANIFEST_MAGIC: [u8; 4] = *b"ACM0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageFormatKind {
    AclogWal,
    Segment,
    BitmapIndex,
    LexicalIndex,
    VectorIndex,
    HnswGraph,
    Manifest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageFormatSpec {
    pub kind: StorageFormatKind,
    pub name: &'static str,
    pub extension: &'static str,
    pub current_magic: &'static [u8],
    pub current_version: u16,
    pub legacy_magics: &'static [&'static [u8]],
    pub compatibility_rule: &'static str,
}

pub fn storage_format_specs() -> [StorageFormatSpec; 7] {
    [
        StorageFormatSpec {
            kind: StorageFormatKind::AclogWal,
            name: "ACLOG WAL",
            extension: "aclog",
            current_magic: &ACLOG_MAGIC,
            current_version: WAL_FORMAT_VERSION,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require WAL_FORMAT_VERSION bump",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::Segment,
            name: "Segment",
            extension: "acs",
            current_magic: &SEGMENT_MAGIC,
            current_version: 1,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require a new segment magic",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::BitmapIndex,
            name: "Bitmap index",
            extension: "acb",
            current_magic: &BITMAP_INDEX_MAGIC,
            current_version: 0,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require a new bitmap magic",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::LexicalIndex,
            name: "Lexical index",
            extension: "aci",
            current_magic: &LEXICAL_INDEX_MAGIC,
            current_version: 3,
            legacy_magics: &[
                &LEGACY_LEXICAL_INDEX_MAGIC,
                &LEGACY_LEXICAL_INDEX_V1_MAGIC,
                &LEGACY_LEXICAL_INDEX_V2_MAGIC,
            ],
            compatibility_rule: "ACI0, ACI1 and ACI2 remain read-only compatible",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::VectorIndex,
            name: "Vector index",
            extension: "acv",
            current_magic: &VECTOR_INDEX_MAGIC,
            current_version: 0,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require a new vector magic",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::HnswGraph,
            name: "HNSW graph",
            extension: "ach",
            current_magic: &HNSW_GRAPH_MAGIC,
            current_version: 0,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require a new HNSW graph magic",
        },
        StorageFormatSpec {
            kind: StorageFormatKind::Manifest,
            name: "Manifest",
            extension: "acm",
            current_magic: &MANIFEST_MAGIC,
            current_version: 0,
            legacy_magics: &[],
            compatibility_rule: "breaking changes require a new manifest magic",
        },
    ]
}
