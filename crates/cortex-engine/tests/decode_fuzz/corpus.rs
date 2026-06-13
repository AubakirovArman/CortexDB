use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentReader, SegmentWriter};
use cortex_storage::vectors::VectorIndex;
use cortex_storage::wal::{SectionTag, WalCodec, WalReader, WalRecord, WalRecordType, WalSection};

#[derive(Clone, Copy, Debug)]
pub enum SeedKind {
    WalRecord,
    WalFile,
    Segment,
    Bitmap,
    Lexical,
    Vector,
    Hnsw,
    Manifest,
}

#[derive(Clone, Debug)]
pub struct DecodeSeed {
    pub name: &'static str,
    pub kind: SeedKind,
    pub path: PathBuf,
}

pub fn build_seed_corpus(root: &Path) -> Vec<DecodeSeed> {
    vec![
        write_wal_record_seed(root),
        write_wal_file_seed(root),
        write_segment_seed(root),
        write_bitmap_seed(root),
        write_lexical_seed(root),
        write_vector_seed(root),
        write_hnsw_seed(root),
        write_manifest_seed(root),
    ]
}

pub fn assert_seed_decodes(seed: &DecodeSeed) {
    match seed.kind {
        SeedKind::WalRecord => {
            WalCodec::decode_record(&fs::read(&seed.path).unwrap()).unwrap();
        }
        SeedKind::WalFile => {
            assert_eq!(WalReader::scan_path(&seed.path).unwrap().records.len(), 2);
        }
        SeedKind::Segment => {
            assert_eq!(SegmentReader::read(&seed.path).unwrap().len(), 2);
            assert_eq!(SegmentReader::read_records(&seed.path).unwrap().len(), 2);
            assert_eq!(
                SegmentReader::read_candidate_entries(&seed.path)
                    .unwrap()
                    .len(),
                2
            );
            assert_eq!(
                SegmentReader::read_descriptors(&seed.path).unwrap().len(),
                2
            );
            assert!(SegmentReader::read_payload_at(&seed.path, 1)
                .unwrap()
                .is_some());
        }
        SeedKind::Bitmap => {
            assert_eq!(BitmapIndex::read(&seed.path).unwrap().bitmaps.len(), 1);
        }
        SeedKind::Lexical => {
            assert_eq!(LexicalIndex::read(&seed.path).unwrap().terms.len(), 1);
            assert_eq!(
                LexicalIndex::read_terms_only(&seed.path)
                    .unwrap()
                    .terms
                    .len(),
                1
            );
        }
        SeedKind::Vector => {
            assert_eq!(VectorIndex::read(&seed.path).unwrap().vectors.len(), 2);
        }
        SeedKind::Hnsw => {
            assert_eq!(HnswGraphIndex::read(&seed.path).unwrap().links.len(), 2);
        }
        SeedKind::Manifest => {
            assert_eq!(
                StorageManifest::load(&seed.path)
                    .unwrap()
                    .live_segments
                    .len(),
                1
            );
        }
    }
}

pub fn assert_decode_is_panic_free(seed: &DecodeSeed, case: &str, bytes: &[u8]) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(seed.path.file_name().unwrap());
    fs::write(&path, bytes).unwrap();
    let result = std::panic::catch_unwind(|| match seed.kind {
        SeedKind::WalRecord => {
            let _ = WalCodec::decode_record(bytes);
        }
        SeedKind::WalFile => {
            let _ = WalReader::scan_path(&path);
            let _ = WalReader::scan_best_effort_path(&path);
        }
        SeedKind::Segment => {
            let _ = SegmentReader::read(&path);
            let _ = SegmentReader::read_records(&path);
            let _ = SegmentReader::read_candidate_entries(&path);
            let _ = SegmentReader::read_descriptors(&path);
            let _ = SegmentReader::read_payload_at(&path, 1);
        }
        SeedKind::Bitmap => {
            let _ = BitmapIndex::read(&path);
        }
        SeedKind::Lexical => {
            let _ = LexicalIndex::read(&path);
            let _ = LexicalIndex::read_terms_only(&path);
        }
        SeedKind::Vector => {
            let _ = VectorIndex::read(&path);
        }
        SeedKind::Hnsw => {
            let _ = HnswGraphIndex::read(&path);
        }
        SeedKind::Manifest => {
            let _ = StorageManifest::load(&path);
        }
    });
    assert!(
        result.is_ok(),
        "{} decoder panicked on mutation case {case}",
        seed.name
    );
}

fn write_wal_record_seed(root: &Path) -> DecodeSeed {
    let path = root.join("seed-record.aclog-record");
    fs::write(
        &path,
        WalCodec::encode_record_at(&wal_record(), 16).unwrap(),
    )
    .unwrap();
    DecodeSeed {
        name: "wal_record",
        kind: SeedKind::WalRecord,
        path,
    }
}

fn write_wal_file_seed(root: &Path) -> DecodeSeed {
    let path = root.join("seed.aclog");
    let first = WalCodec::encode_record_at(&wal_record(), 16).unwrap();
    let second = WalCodec::encode_record_at(&wal_record(), 16 + first.len() as u64).unwrap();
    let mut bytes = Vec::from(WalCodec::file_header());
    bytes.extend_from_slice(&first);
    bytes.extend_from_slice(&second);
    fs::write(&path, bytes).unwrap();
    DecodeSeed {
        name: "wal_file",
        kind: SeedKind::WalFile,
        path,
    }
}

fn write_segment_seed(root: &Path) -> DecodeSeed {
    let path = root.join("segment-1.acs");
    SegmentWriter::write(
        &path,
        &[
            segment_cell(1, 10, b"scope=project:decode\nstatus=ready\none"),
            segment_cell(2, 20, b"scope=project:decode\nstatus=ready\ntwo"),
        ],
    )
    .unwrap();
    DecodeSeed {
        name: "segment",
        kind: SeedKind::Segment,
        path,
    }
}

fn write_bitmap_seed(root: &Path) -> DecodeSeed {
    let path = root.join("segment-1.acb");
    BitmapIndex {
        bitmaps: BTreeMap::from([(42, BTreeSet::from([1, 2]))]),
    }
    .write(&path)
    .unwrap();
    DecodeSeed {
        name: "bitmap_index",
        kind: SeedKind::Bitmap,
        path,
    }
}

fn write_lexical_seed(root: &Path) -> DecodeSeed {
    let path = root.join("segment-1.aci");
    LexicalIndex {
        terms: BTreeMap::from([("decode".to_owned(), BTreeSet::from([1, 2]))]),
        doc_lengths: BTreeMap::from([(1, 4), (2, 3)]),
        term_frequencies: BTreeMap::from([("decode".to_owned(), BTreeMap::from([(1, 2), (2, 1)]))]),
        ..LexicalIndex::default()
    }
    .write(&path)
    .unwrap();
    DecodeSeed {
        name: "lexical_index",
        kind: SeedKind::Lexical,
        path,
    }
}

fn write_vector_seed(root: &Path) -> DecodeSeed {
    let path = root.join("segment-1.acv");
    VectorIndex {
        vectors: BTreeMap::from([(1, vec![10, -3, 5]), (2, vec![4, 2, -8])]),
    }
    .write(&path)
    .unwrap();
    DecodeSeed {
        name: "vector_index",
        kind: SeedKind::Vector,
        path,
    }
}

fn write_hnsw_seed(root: &Path) -> DecodeSeed {
    let path = root.join("segment-1.ach");
    HnswGraphIndex {
        links: BTreeMap::from([(1, BTreeSet::from([2])), (2, BTreeSet::from([1]))]),
        dimension: 3,
        metric: 0,
        upper_layers: BTreeMap::from([(1, BTreeMap::from([(1, BTreeSet::from([2]))]))]),
        max_neighbors: 16,
        ef_search: 128,
        layer_count: 3,
        ef_construction: 256,
    }
    .write(&path)
    .unwrap();
    DecodeSeed {
        name: "hnsw_graph",
        kind: SeedKind::Hnsw,
        path,
    }
}

fn write_manifest_seed(root: &Path) -> DecodeSeed {
    let path = root.join("manifest.acm");
    let mut manifest = StorageManifest::default();
    manifest.checkpoint_segment(ManifestSegment {
        id: 1,
        generation: 1,
        checkpoint_seq: 2,
        cell_count: 2,
    });
    manifest.store(&path).unwrap();
    DecodeSeed {
        name: "manifest",
        kind: SeedKind::Manifest,
        path,
    }
}

fn wal_record() -> WalRecord {
    WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, b"cell-core".to_vec()),
            WalSection::new(SectionTag::PayloadInline, b"payload".to_vec()),
            WalSection::new(SectionTag::CellMetadata, b"scope=project:decode".to_vec()),
        ],
    )
}

fn segment_cell(candidate_id: u32, cell_id: u64, payload: &[u8]) -> SegmentCell {
    SegmentCell {
        candidate_id,
        cell_id,
        created_seq: candidate_id as u64,
        deleted_seq: None,
        payload: payload.to_vec(),
    }
}
