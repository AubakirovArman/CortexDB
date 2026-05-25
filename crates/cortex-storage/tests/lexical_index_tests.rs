use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::indexes::LexicalIndex;
use cortex_storage::wal::checksum::crc32c;

#[test]
fn aci_lexical_index_roundtrips_terms() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.aci");
    let index = LexicalIndex {
        terms: BTreeMap::from([
            ("budget".to_owned(), BTreeSet::from([1, 2])),
            ("ready".to_owned(), BTreeSet::from([2])),
        ]),
        doc_lengths: BTreeMap::from([(1, 7), (2, 3)]),
        term_frequencies: BTreeMap::from([
            ("budget".to_owned(), BTreeMap::from([(1, 2), (2, 1)])),
            ("ready".to_owned(), BTreeMap::from([(2, 1)])),
        ]),
    };
    index.write(&path).unwrap();
    assert_eq!(LexicalIndex::read(&path).unwrap(), index);
    assert!(!dir.path().join("0001.aci.tmp").exists());
}

#[test]
fn aci0_lexical_index_remains_readable_without_doc_lengths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.aci");
    let mut bytes = Vec::from(&b"ACI0"[..]);
    put_u32(&mut bytes, 1);
    put_u16(&mut bytes, 6);
    bytes.extend_from_slice(b"budget");
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 7);
    append_crc(&mut bytes);
    std::fs::write(&path, bytes).unwrap();

    let index = LexicalIndex::read(&path).unwrap();
    assert_eq!(index.terms["budget"], BTreeSet::from([7]));
    assert!(index.doc_lengths.is_empty());
    assert!(index.term_frequencies.is_empty());
}

#[test]
fn aci1_lexical_index_remains_readable_without_term_frequencies() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old-v1.aci");
    let mut bytes = Vec::from(&b"ACI1"[..]);
    put_u32(&mut bytes, 1);
    put_u16(&mut bytes, 6);
    bytes.extend_from_slice(b"budget");
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 7);
    put_u32(&mut bytes, 1);
    put_u32(&mut bytes, 7);
    put_u32(&mut bytes, 3);
    append_crc(&mut bytes);
    std::fs::write(&path, bytes).unwrap();

    let index = LexicalIndex::read(&path).unwrap();
    assert_eq!(index.terms["budget"], BTreeSet::from([7]));
    assert_eq!(index.doc_lengths.get(&7), Some(&3));
    assert!(index.term_frequencies.is_empty());
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn append_crc(out: &mut Vec<u8>) {
    let checksum = crc32c(out);
    out.extend_from_slice(&checksum.to_le_bytes());
}
