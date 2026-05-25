use crate::wal::{ACLOG_MAGIC, WAL_FORMAT_VERSION};

pub const SEGMENT_MAGIC: [u8; 4] = *b"ACS1";
pub const BITMAP_INDEX_MAGIC: [u8; 4] = *b"ACB0";
pub const LEXICAL_INDEX_MAGIC: [u8; 4] = *b"ACI1";
pub const LEGACY_LEXICAL_INDEX_MAGIC: [u8; 4] = *b"ACI0";
pub const MANIFEST_MAGIC: [u8; 4] = *b"ACM0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageFormatKind {
    AclogWal,
    Segment,
    BitmapIndex,
    LexicalIndex,
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

pub fn storage_format_specs() -> [StorageFormatSpec; 5] {
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
            current_version: 1,
            legacy_magics: &[&LEGACY_LEXICAL_INDEX_MAGIC],
            compatibility_rule: "ACI0 remains read-only compatible",
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
