use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::format::{LEGACY_LEXICAL_INDEX_V3_MAGIC, LEXICAL_INDEX_MAGIC};
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
        field_doc_lengths: BTreeMap::from([
            ("body".to_owned(), BTreeMap::from([(1, 5), (2, 2)])),
            ("title".to_owned(), BTreeMap::from([(1, 2)])),
        ]),
        field_term_frequencies: BTreeMap::from([
            (
                "body".to_owned(),
                BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1), (2, 1)]))]),
            ),
            (
                "title".to_owned(),
                BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1)]))]),
            ),
        ]),
    };
    index.write(&path).unwrap();
    assert_eq!(&std::fs::read(&path).unwrap()[..4], &LEXICAL_INDEX_MAGIC);
    assert_eq!(LexicalIndex::read(&path).unwrap(), index);
    assert!(!dir.path().join("0001.aci.tmp").exists());
}

#[test]
fn aci_lexical_index_terms_only_read_skips_heavy_frequency_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("0001.aci");
    let index = LexicalIndex {
        terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
        doc_lengths: BTreeMap::from([(1, 7), (2, 3)]),
        term_frequencies: BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 2), (2, 1)]))]),
        field_doc_lengths: BTreeMap::from([("title".to_owned(), BTreeMap::from([(1, 2)]))]),
        field_term_frequencies: BTreeMap::from([(
            "title".to_owned(),
            BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1)]))]),
        )]),
    };
    index.write(&path).unwrap();

    let light = LexicalIndex::read_terms_only(&path).unwrap();

    assert_eq!(light.terms, index.terms);
    assert_eq!(light.doc_lengths, index.doc_lengths);
    assert!(light.term_frequencies.is_empty());
    assert!(light.field_doc_lengths.is_empty());
    assert!(light.field_term_frequencies.is_empty());
}

#[test]
fn aci3_lexical_index_remains_readable_with_field_frequencies() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("legacy-v3.aci");
    let current_path = dir.path().join("migrated.aci");
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
        field_doc_lengths: BTreeMap::from([("title".to_owned(), BTreeMap::from([(1, 2)]))]),
        field_term_frequencies: BTreeMap::from([(
            "title".to_owned(),
            BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1)]))]),
        )]),
    };
    std::fs::write(&legacy_path, encode_legacy_aci3(&index)).unwrap();

    let loaded = LexicalIndex::read(&legacy_path).unwrap();
    assert_eq!(loaded, index);

    let light = LexicalIndex::read_terms_only(&legacy_path).unwrap();
    assert_eq!(light.terms, index.terms);
    assert_eq!(light.doc_lengths, index.doc_lengths);
    assert!(light.term_frequencies.is_empty());

    loaded.write(&current_path).unwrap();
    let current_bytes = std::fs::read(&current_path).unwrap();
    assert_eq!(&current_bytes[..4], &LEXICAL_INDEX_MAGIC);
    assert_eq!(LexicalIndex::read(&current_path).unwrap(), index);
}

#[test]
fn aci4_term_dictionary_reduces_repeated_term_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("compact.aci");
    let mut index = LexicalIndex::default();
    let fields = ["title", "body", "path", "heading"];
    for candidate in 1..=300u32 {
        let term = format!("very_long_enterprise_budget_retrieval_term_{candidate:04}");
        index
            .terms
            .insert(term.clone(), BTreeSet::from([candidate]));
        index
            .term_frequencies
            .insert(term.clone(), BTreeMap::from([(candidate, 1)]));
        index.doc_lengths.insert(candidate, 4);
        for field in fields {
            index
                .field_doc_lengths
                .entry(field.to_owned())
                .or_default()
                .insert(candidate, 1);
            index
                .field_term_frequencies
                .entry(field.to_owned())
                .or_default()
                .insert(term.clone(), BTreeMap::from([(candidate, 1)]));
        }
    }

    let legacy = encode_legacy_aci3(&index);
    index.write(&path).unwrap();
    let compact = std::fs::read(&path).unwrap();

    assert!(compact.len() * 3 < legacy.len());
    assert_eq!(LexicalIndex::read(&path).unwrap(), index);
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

fn encode_legacy_aci3(index: &LexicalIndex) -> Vec<u8> {
    let mut bytes = Vec::from(&LEGACY_LEXICAL_INDEX_V3_MAGIC[..]);
    put_u32(&mut bytes, index.terms.len() as u32);
    for (term, values) in &index.terms {
        put_string(&mut bytes, term);
        put_u32(&mut bytes, values.len() as u32);
        for value in values {
            put_u32(&mut bytes, *value);
        }
    }
    put_u32(&mut bytes, index.doc_lengths.len() as u32);
    for (candidate, length) in &index.doc_lengths {
        put_u32(&mut bytes, *candidate);
        put_u32(&mut bytes, *length);
    }
    put_u32(&mut bytes, index.term_frequencies.len() as u32);
    for (term, values) in &index.term_frequencies {
        put_string(&mut bytes, term);
        put_u32(&mut bytes, values.len() as u32);
        for (candidate, frequency) in values {
            put_u32(&mut bytes, *candidate);
            put_u32(&mut bytes, *frequency);
        }
    }
    put_u32(&mut bytes, index.field_doc_lengths.len() as u32);
    for (field, values) in &index.field_doc_lengths {
        put_string(&mut bytes, field);
        put_u32(&mut bytes, values.len() as u32);
        for (candidate, length) in values {
            put_u32(&mut bytes, *candidate);
            put_u32(&mut bytes, *length);
        }
    }
    put_u32(&mut bytes, index.field_term_frequencies.len() as u32);
    for (field, terms) in &index.field_term_frequencies {
        put_string(&mut bytes, field);
        put_u32(&mut bytes, terms.len() as u32);
        for (term, values) in terms {
            put_string(&mut bytes, term);
            put_u32(&mut bytes, values.len() as u32);
            for (candidate, frequency) in values {
                put_u32(&mut bytes, *candidate);
                put_u32(&mut bytes, *frequency);
            }
        }
    }
    append_crc(&mut bytes);
    bytes
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    put_u16(out, value.len() as u16);
    out.extend_from_slice(value.as_bytes());
}
