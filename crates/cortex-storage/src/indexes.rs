use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::path::Path;

use roaring::RoaringBitmap;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::{
    BITMAP_INDEX_MAGIC, LEGACY_BITMAP_INDEX_MAGIC, LEGACY_LEXICAL_INDEX_MAGIC,
    LEGACY_LEXICAL_INDEX_V1_MAGIC, LEGACY_LEXICAL_INDEX_V2_MAGIC, LEGACY_LEXICAL_INDEX_V3_MAGIC,
    LEXICAL_INDEX_MAGIC,
};

mod codec;

use codec::{
    put_u16, put_u32, put_u64, put_var_u32, read_bytes, read_field_term_frequencies,
    read_frequencies, read_set, read_u16, read_u32, read_u64, read_var_u32, IndexKind,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BitmapIndex {
    pub bitmaps: BTreeMap<u64, RoaringBitmap>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexicalIndex {
    pub terms: BTreeMap<String, BTreeSet<u32>>,
    pub doc_lengths: BTreeMap<u32, u32>,
    pub term_frequencies: BTreeMap<String, BTreeMap<u32, u32>>,
    pub field_doc_lengths: BTreeMap<String, BTreeMap<u32, u32>>,
    pub field_term_frequencies: BTreeMap<String, BTreeMap<String, BTreeMap<u32, u32>>>,
}

impl BitmapIndex {
    pub fn from_sets(bitmaps: BTreeMap<u64, BTreeSet<u32>>) -> Self {
        Self {
            bitmaps: bitmaps
                .into_iter()
                .map(|(handle, values)| (handle, values.into_iter().collect()))
                .collect(),
        }
    }

    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        let mut out = Vec::from(&BITMAP_INDEX_MAGIC[..]);
        put_u32(&mut out, self.bitmaps.len() as u32);
        for (handle, values) in &self.bitmaps {
            put_u64(&mut out, *handle);
            let mut serialized = Vec::with_capacity(values.serialized_size());
            values
                .serialize_into(&mut serialized)
                .map_err(|_| StorageError::InvalidBitmapIndexFile)?;
            put_u32(&mut out, serialized.len() as u32);
            out.extend_from_slice(&serialized);
        }
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        let bytes = std::fs::read(path)?;
        decode_bitmap(&bytes)
    }
}

impl LexicalIndex {
    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        let dictionary = build_term_dictionary(self);
        let term_ids = dictionary
            .iter()
            .enumerate()
            .map(|(term_id, term)| (term.as_str(), term_id as u32))
            .collect::<BTreeMap<_, _>>();
        let mut out = Vec::from(&LEXICAL_INDEX_MAGIC[..]);
        put_u32(&mut out, dictionary.len() as u32);
        for term in &dictionary {
            put_string(&mut out, term)?;
        }

        put_u32(&mut out, self.terms.len() as u32);
        for (term, values) in &self.terms {
            put_u32(&mut out, lookup_term_id(&term_ids, term)?);
            put_compact_set(&mut out, values);
        }
        put_compact_u32_map(&mut out, &self.doc_lengths);

        put_u32(&mut out, self.term_frequencies.len() as u32);
        for (term, values) in &self.term_frequencies {
            put_u32(&mut out, lookup_term_id(&term_ids, term)?);
            put_compact_u32_map(&mut out, values);
        }

        put_u32(&mut out, self.field_doc_lengths.len() as u32);
        for (field, values) in &self.field_doc_lengths {
            put_string(&mut out, field)?;
            put_compact_u32_map(&mut out, values);
        }

        put_u32(&mut out, self.field_term_frequencies.len() as u32);
        for (field, terms) in &self.field_term_frequencies {
            put_string(&mut out, field)?;
            put_u32(&mut out, terms.len() as u32);
            for (term, values) in terms {
                put_u32(&mut out, lookup_term_id(&term_ids, term)?);
                put_compact_u32_map(&mut out, values);
            }
        }
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        let bytes = std::fs::read(path)?;
        decode_lexical(&bytes)
    }

    pub fn read_terms_only(path: impl AsRef<Path>) -> StorageResult<Self> {
        let bytes = std::fs::read(path)?;
        decode_lexical_terms_only(&bytes)
    }
}

fn decode_bitmap(bytes: &[u8]) -> StorageResult<BitmapIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidBitmapIndexFile)?;
    if bytes.len() < 8
        || (bytes[..4] != BITMAP_INDEX_MAGIC && bytes[..4] != LEGACY_BITMAP_INDEX_MAGIC)
    {
        return Err(StorageError::InvalidBitmapIndexFile);
    }
    let version = &bytes[..4];
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor, IndexKind::Bitmap)? as usize;
    let mut bitmaps = BTreeMap::new();
    for _ in 0..count {
        let handle = read_u64(bytes, &mut cursor, IndexKind::Bitmap)?;
        let bitmap = if version == LEGACY_BITMAP_INDEX_MAGIC {
            read_set(bytes, &mut cursor, IndexKind::Bitmap)?
                .into_iter()
                .collect()
        } else {
            read_roaring_bitmap(bytes, &mut cursor)?
        };
        bitmaps.insert(handle, bitmap);
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidBitmapIndexFile);
    }
    Ok(BitmapIndex { bitmaps })
}

fn read_roaring_bitmap(bytes: &[u8], cursor: &mut usize) -> StorageResult<RoaringBitmap> {
    let len = read_u32(bytes, cursor, IndexKind::Bitmap)? as usize;
    let payload = read_bytes(bytes, cursor, len, IndexKind::Bitmap)?;
    RoaringBitmap::deserialize_from(Cursor::new(payload))
        .map_err(|_| StorageError::InvalidBitmapIndexFile)
}

fn build_term_dictionary(index: &LexicalIndex) -> Vec<String> {
    let mut terms = BTreeSet::new();
    terms.extend(index.terms.keys().cloned());
    terms.extend(index.term_frequencies.keys().cloned());
    for field_terms in index.field_term_frequencies.values() {
        terms.extend(field_terms.keys().cloned());
    }
    terms.into_iter().collect()
}

fn lookup_term_id(term_ids: &BTreeMap<&str, u32>, term: &str) -> StorageResult<u32> {
    term_ids
        .get(term)
        .copied()
        .ok_or(StorageError::InvalidLexicalIndexFile)
}

fn put_string(out: &mut Vec<u8>, value: &str) -> StorageResult<()> {
    let len = u16::try_from(value.len()).map_err(|_| StorageError::InvalidLexicalIndexFile)?;
    put_u16(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> StorageResult<String> {
    let len = read_u16(bytes, cursor, IndexKind::Lexical)? as usize;
    std::str::from_utf8(read_bytes(bytes, cursor, len, IndexKind::Lexical)?)
        .map_err(|_| StorageError::InvalidLexicalIndexFile)
        .map(str::to_owned)
}

fn put_compact_set(out: &mut Vec<u8>, values: &BTreeSet<u32>) {
    put_u32(out, values.len() as u32);
    let mut previous = 0u32;
    for value in values {
        put_var_u32(out, value.saturating_sub(previous));
        previous = *value;
    }
}

fn read_compact_set(bytes: &[u8], cursor: &mut usize) -> StorageResult<BTreeSet<u32>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut values = BTreeSet::new();
    let mut previous = 0u32;
    for index in 0..count {
        let delta = read_var_u32(bytes, cursor, IndexKind::Lexical)?;
        if index > 0 && delta == 0 {
            return Err(StorageError::InvalidLexicalIndexFile);
        }
        let value = previous
            .checked_add(delta)
            .ok_or(StorageError::InvalidLexicalIndexFile)?;
        values.insert(value);
        previous = value;
    }
    Ok(values)
}

fn put_compact_u32_map(out: &mut Vec<u8>, values: &BTreeMap<u32, u32>) {
    put_u32(out, values.len() as u32);
    let mut previous = 0u32;
    for (candidate, value) in values {
        put_var_u32(out, candidate.saturating_sub(previous));
        put_var_u32(out, *value);
        previous = *candidate;
    }
}

fn read_compact_u32_map(bytes: &[u8], cursor: &mut usize) -> StorageResult<BTreeMap<u32, u32>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut values = BTreeMap::new();
    let mut previous = 0u32;
    for index in 0..count {
        let delta = read_var_u32(bytes, cursor, IndexKind::Lexical)?;
        if index > 0 && delta == 0 {
            return Err(StorageError::InvalidLexicalIndexFile);
        }
        let candidate = previous
            .checked_add(delta)
            .ok_or(StorageError::InvalidLexicalIndexFile)?;
        let value = read_var_u32(bytes, cursor, IndexKind::Lexical)?;
        values.insert(candidate, value);
        previous = candidate;
    }
    Ok(values)
}

fn read_term_dictionary(bytes: &[u8], cursor: &mut usize) -> StorageResult<Vec<String>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut dictionary = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    for _ in 0..count {
        let term = read_string(bytes, cursor)?;
        if !seen.insert(term.clone()) {
            return Err(StorageError::InvalidLexicalIndexFile);
        }
        dictionary.push(term);
    }
    Ok(dictionary)
}

fn dictionary_term(dictionary: &[String], term_id: u32) -> StorageResult<String> {
    dictionary
        .get(term_id as usize)
        .cloned()
        .ok_or(StorageError::InvalidLexicalIndexFile)
}

fn decode_lexical(bytes: &[u8]) -> StorageResult<LexicalIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidLexicalIndexFile)?;
    if !is_lexical_magic(bytes) {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    let version = &bytes[..4];
    let mut cursor = 4;
    if version == LEXICAL_INDEX_MAGIC {
        return decode_compact_lexical(bytes, &mut cursor);
    }
    decode_legacy_lexical(bytes, version, &mut cursor, true)
}

fn decode_compact_lexical(bytes: &[u8], cursor: &mut usize) -> StorageResult<LexicalIndex> {
    let dictionary = read_term_dictionary(bytes, cursor)?;

    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let term_id = read_u32(bytes, cursor, IndexKind::Lexical)?;
        let term = dictionary_term(&dictionary, term_id)?;
        terms.insert(term, read_compact_set(bytes, cursor)?);
    }

    let doc_lengths = read_compact_u32_map(bytes, cursor)?;

    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut term_frequencies = BTreeMap::new();
    for _ in 0..count {
        let term_id = read_u32(bytes, cursor, IndexKind::Lexical)?;
        let term = dictionary_term(&dictionary, term_id)?;
        term_frequencies.insert(term, read_compact_u32_map(bytes, cursor)?);
    }

    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut field_doc_lengths = BTreeMap::new();
    for _ in 0..count {
        let field = read_string(bytes, cursor)?;
        field_doc_lengths.insert(field, read_compact_u32_map(bytes, cursor)?);
    }

    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut field_term_frequencies = BTreeMap::new();
    for _ in 0..count {
        let field = read_string(bytes, cursor)?;
        let term_count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
        let mut field_terms = BTreeMap::new();
        for _ in 0..term_count {
            let term_id = read_u32(bytes, cursor, IndexKind::Lexical)?;
            let term = dictionary_term(&dictionary, term_id)?;
            field_terms.insert(term, read_compact_u32_map(bytes, cursor)?);
        }
        field_term_frequencies.insert(field, field_terms);
    }

    if *cursor != bytes.len() {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    Ok(LexicalIndex {
        terms,
        doc_lengths,
        term_frequencies,
        field_doc_lengths,
        field_term_frequencies,
    })
}

fn decode_legacy_lexical(
    bytes: &[u8],
    version: &[u8],
    cursor: &mut usize,
    require_full_read: bool,
) -> StorageResult<LexicalIndex> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let term = read_string(bytes, cursor)?;
        terms.insert(term, read_set(bytes, cursor, IndexKind::Lexical)?);
    }
    let mut doc_lengths = BTreeMap::new();
    if version == LEGACY_LEXICAL_INDEX_V1_MAGIC
        || version == LEGACY_LEXICAL_INDEX_V2_MAGIC
        || version == LEGACY_LEXICAL_INDEX_V3_MAGIC
    {
        let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let candidate = read_u32(bytes, cursor, IndexKind::Lexical)?;
            let length = read_u32(bytes, cursor, IndexKind::Lexical)?;
            doc_lengths.insert(candidate, length);
        }
    }
    let mut term_frequencies = BTreeMap::new();
    if require_full_read
        && (version == LEGACY_LEXICAL_INDEX_V2_MAGIC || version == LEGACY_LEXICAL_INDEX_V3_MAGIC)
    {
        let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let term = read_string(bytes, cursor)?;
            term_frequencies.insert(term, read_frequencies(bytes, cursor)?);
        }
    }
    let mut field_doc_lengths = BTreeMap::new();
    let mut field_term_frequencies = BTreeMap::new();
    if require_full_read && version == LEGACY_LEXICAL_INDEX_V3_MAGIC {
        let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let field = read_string(bytes, cursor)?;
            field_doc_lengths.insert(field, read_frequencies(bytes, cursor)?);
        }
        let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let field = read_string(bytes, cursor)?;
            field_term_frequencies.insert(field, read_field_term_frequencies(bytes, cursor)?);
        }
    }
    if require_full_read && *cursor != bytes.len() {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    Ok(LexicalIndex {
        terms,
        doc_lengths,
        term_frequencies,
        field_doc_lengths,
        field_term_frequencies,
    })
}

fn decode_lexical_terms_only(bytes: &[u8]) -> StorageResult<LexicalIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidLexicalIndexFile)?;
    if !is_lexical_magic(bytes) {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    let version = &bytes[..4];
    let mut cursor = 4;
    if version != LEXICAL_INDEX_MAGIC {
        return decode_legacy_lexical(bytes, version, &mut cursor, false);
    }

    let dictionary = read_term_dictionary(bytes, &mut cursor)?;
    let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let term_id = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
        let term = dictionary_term(&dictionary, term_id)?;
        terms.insert(term, read_compact_set(bytes, &mut cursor)?);
    }
    let doc_lengths = read_compact_u32_map(bytes, &mut cursor)?;
    Ok(LexicalIndex {
        terms,
        doc_lengths,
        term_frequencies: BTreeMap::new(),
        field_doc_lengths: BTreeMap::new(),
        field_term_frequencies: BTreeMap::new(),
    })
}

fn is_lexical_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && (bytes[..4] == LEGACY_LEXICAL_INDEX_MAGIC
            || bytes[..4] == LEGACY_LEXICAL_INDEX_V1_MAGIC
            || bytes[..4] == LEGACY_LEXICAL_INDEX_V2_MAGIC
            || bytes[..4] == LEGACY_LEXICAL_INDEX_V3_MAGIC
            || bytes[..4] == LEXICAL_INDEX_MAGIC)
}
