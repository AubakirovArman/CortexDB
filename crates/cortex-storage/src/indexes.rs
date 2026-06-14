use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::path::Path;

use roaring::RoaringBitmap;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::{
    BITMAP_INDEX_MAGIC, LEGACY_BITMAP_INDEX_MAGIC, LEGACY_LEXICAL_INDEX_MAGIC,
    LEGACY_LEXICAL_INDEX_V1_MAGIC, LEGACY_LEXICAL_INDEX_V2_MAGIC, LEXICAL_INDEX_MAGIC,
};

mod codec;

use codec::{
    put_u16, put_u32, put_u64, read_bytes, read_field_term_frequencies, read_frequencies, read_set,
    read_u16, read_u32, read_u64, IndexKind,
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
        let mut out = Vec::from(&LEXICAL_INDEX_MAGIC[..]);
        put_u32(&mut out, self.terms.len() as u32);
        for (term, values) in &self.terms {
            put_u16(&mut out, term.len() as u16);
            out.extend_from_slice(term.as_bytes());
            put_u32(&mut out, values.len() as u32);
            for value in values {
                put_u32(&mut out, *value);
            }
        }
        put_u32(&mut out, self.doc_lengths.len() as u32);
        for (candidate, length) in &self.doc_lengths {
            put_u32(&mut out, *candidate);
            put_u32(&mut out, *length);
        }
        put_u32(&mut out, self.term_frequencies.len() as u32);
        for (term, values) in &self.term_frequencies {
            put_u16(&mut out, term.len() as u16);
            out.extend_from_slice(term.as_bytes());
            put_u32(&mut out, values.len() as u32);
            for (candidate, frequency) in values {
                put_u32(&mut out, *candidate);
                put_u32(&mut out, *frequency);
            }
        }
        put_u32(&mut out, self.field_doc_lengths.len() as u32);
        for (field, values) in &self.field_doc_lengths {
            put_u16(&mut out, field.len() as u16);
            out.extend_from_slice(field.as_bytes());
            put_u32(&mut out, values.len() as u32);
            for (candidate, length) in values {
                put_u32(&mut out, *candidate);
                put_u32(&mut out, *length);
            }
        }
        put_u32(&mut out, self.field_term_frequencies.len() as u32);
        for (field, terms) in &self.field_term_frequencies {
            put_u16(&mut out, field.len() as u16);
            out.extend_from_slice(field.as_bytes());
            put_u32(&mut out, terms.len() as u32);
            for (term, values) in terms {
                put_u16(&mut out, term.len() as u16);
                out.extend_from_slice(term.as_bytes());
                put_u32(&mut out, values.len() as u32);
                for (candidate, frequency) in values {
                    put_u32(&mut out, *candidate);
                    put_u32(&mut out, *frequency);
                }
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

fn decode_lexical(bytes: &[u8]) -> StorageResult<LexicalIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidLexicalIndexFile)?;
    if !is_lexical_magic(bytes) {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    let version = &bytes[..4];
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let len = read_u16(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        let term = std::str::from_utf8(read_bytes(bytes, &mut cursor, len, IndexKind::Lexical)?)
            .map_err(|_| StorageError::InvalidLexicalIndexFile)?
            .to_owned();
        terms.insert(term, read_set(bytes, &mut cursor, IndexKind::Lexical)?);
    }
    let mut doc_lengths = BTreeMap::new();
    if version == LEGACY_LEXICAL_INDEX_V1_MAGIC
        || version == LEGACY_LEXICAL_INDEX_V2_MAGIC
        || version == LEXICAL_INDEX_MAGIC
    {
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let candidate = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            let length = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            doc_lengths.insert(candidate, length);
        }
    }
    let mut term_frequencies = BTreeMap::new();
    if version == LEGACY_LEXICAL_INDEX_V2_MAGIC || version == LEXICAL_INDEX_MAGIC {
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let len = read_u16(bytes, &mut cursor, IndexKind::Lexical)? as usize;
            let term =
                std::str::from_utf8(read_bytes(bytes, &mut cursor, len, IndexKind::Lexical)?)
                    .map_err(|_| StorageError::InvalidLexicalIndexFile)?
                    .to_owned();
            term_frequencies.insert(term, read_frequencies(bytes, &mut cursor)?);
        }
    }
    let mut field_doc_lengths = BTreeMap::new();
    let mut field_term_frequencies = BTreeMap::new();
    if version == LEXICAL_INDEX_MAGIC {
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let len = read_u16(bytes, &mut cursor, IndexKind::Lexical)? as usize;
            let field =
                std::str::from_utf8(read_bytes(bytes, &mut cursor, len, IndexKind::Lexical)?)
                    .map_err(|_| StorageError::InvalidLexicalIndexFile)?
                    .to_owned();
            field_doc_lengths.insert(field, read_frequencies(bytes, &mut cursor)?);
        }
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let len = read_u16(bytes, &mut cursor, IndexKind::Lexical)? as usize;
            let field =
                std::str::from_utf8(read_bytes(bytes, &mut cursor, len, IndexKind::Lexical)?)
                    .map_err(|_| StorageError::InvalidLexicalIndexFile)?
                    .to_owned();
            field_term_frequencies.insert(field, read_field_term_frequencies(bytes, &mut cursor)?);
        }
    }
    if cursor != bytes.len() {
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
    let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let len = read_u16(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        let term = std::str::from_utf8(read_bytes(bytes, &mut cursor, len, IndexKind::Lexical)?)
            .map_err(|_| StorageError::InvalidLexicalIndexFile)?
            .to_owned();
        terms.insert(term, read_set(bytes, &mut cursor, IndexKind::Lexical)?);
    }
    let mut doc_lengths = BTreeMap::new();
    if version == LEGACY_LEXICAL_INDEX_V1_MAGIC
        || version == LEGACY_LEXICAL_INDEX_V2_MAGIC
        || version == LEXICAL_INDEX_MAGIC
    {
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let candidate = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            let length = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            doc_lengths.insert(candidate, length);
        }
    }
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
            || bytes[..4] == LEXICAL_INDEX_MAGIC)
}
