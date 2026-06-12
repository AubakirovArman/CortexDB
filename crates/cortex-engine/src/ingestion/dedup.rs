use cortex_core::CellId;

use crate::database::Database;
use crate::query::CellMetadata;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateContentGroup {
    pub content_hash: String,
    pub canonical_cell_id: CellId,
    pub duplicate_cell_ids: Vec<CellId>,
    pub source_hashes: Vec<String>,
}

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
            let metadata =
                CellMetadata::from_payload_with_descriptor(&version.payload, &version.descriptor);
            let same_source = metadata.source_hash.as_deref() == Some(source_hash);
            let same_content = metadata.content_hash.as_deref() == Some(content_hash);
            (same_source && same_content).then_some(version.cell_id)
        })
    }

    pub fn duplicate_content_groups(&self) -> Vec<DuplicateContentGroup> {
        let mut groups =
            std::collections::BTreeMap::<String, (CellId, Vec<CellId>, Vec<String>)>::new();
        for version in self.snapshot_versions() {
            let metadata =
                CellMetadata::from_payload_with_descriptor(&version.payload, &version.descriptor);
            let Some(content_hash) = metadata.content_hash else {
                continue;
            };
            let entry = groups.entry(content_hash).or_insert_with(|| {
                (
                    version.cell_id,
                    Vec::new(),
                    metadata.source_hash.iter().cloned().collect(),
                )
            });
            if version.cell_id < entry.0 {
                entry.1.push(entry.0);
                entry.0 = version.cell_id;
            } else if version.cell_id != entry.0 {
                entry.1.push(version.cell_id);
            }
            if let Some(source_hash) = metadata.source_hash {
                if !entry.2.contains(&source_hash) {
                    entry.2.push(source_hash);
                }
            }
        }
        groups
            .into_iter()
            .filter_map(
                |(content_hash, (canonical_cell_id, mut duplicate_cell_ids, mut source_hashes))| {
                    if duplicate_cell_ids.is_empty() {
                        return None;
                    }
                    duplicate_cell_ids.sort();
                    duplicate_cell_ids.dedup();
                    source_hashes.sort();
                    source_hashes.dedup();
                    Some(DuplicateContentGroup {
                        content_hash,
                        canonical_cell_id,
                        duplicate_cell_ids,
                        source_hashes,
                    })
                },
            )
            .collect()
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
