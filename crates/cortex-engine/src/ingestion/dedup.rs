use cortex_core::CellId;

use crate::database::Database;
use crate::query::CellMetadata;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IngestionUpdatePolicy {
    #[default]
    AlwaysInsert,
    SkipExisting,
}

impl Database {
    pub fn find_duplicate_ingestion_chunk(
        &self,
        source_hash: &str,
        content_hash: &str,
    ) -> Option<CellId> {
        self.snapshot_versions().into_iter().find_map(|version| {
            let metadata = CellMetadata::from_payload(&version.payload);
            let same_source = metadata.source_hash.as_deref() == Some(source_hash);
            let same_content = metadata.content_hash.as_deref() == Some(content_hash);
            (same_source && same_content).then_some(version.cell_id)
        })
    }
}

pub(crate) fn content_hash_hex(text: &str) -> String {
    stable_ingestion_hash_hex(text.as_bytes())
}

pub(crate) fn source_hash_hex(source: &str) -> String {
    stable_ingestion_hash_hex(source.as_bytes())
}

pub fn stable_ingestion_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}
