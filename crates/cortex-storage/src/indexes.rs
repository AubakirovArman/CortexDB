use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::{
    BITMAP_INDEX_MAGIC, LEGACY_LEXICAL_INDEX_MAGIC, LEGACY_LEXICAL_INDEX_V1_MAGIC,
    LEXICAL_INDEX_MAGIC,
};

#[derive(Clone, Copy, Debug)]
enum IndexKind {
    Bitmap,
    Lexical,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BitmapIndex {
    pub bitmaps: BTreeMap<u64, BTreeSet<u32>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexicalIndex {
    pub terms: BTreeMap<String, BTreeSet<u32>>,
    pub doc_lengths: BTreeMap<u32, u32>,
    pub term_frequencies: BTreeMap<String, BTreeMap<u32, u32>>,
}

impl BitmapIndex {
    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        let mut out = Vec::from(&BITMAP_INDEX_MAGIC[..]);
        put_u32(&mut out, self.bitmaps.len() as u32);
        for (handle, values) in &self.bitmaps {
            put_u64(&mut out, *handle);
            put_u32(&mut out, values.len() as u32);
            for value in values {
                put_u32(&mut out, *value);
            }
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
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)?;
        Ok(())
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        let bytes = std::fs::read(path)?;
        decode_lexical(&bytes)
    }
}

fn decode_bitmap(bytes: &[u8]) -> StorageResult<BitmapIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidBitmapIndexFile)?;
    if bytes.len() < 8 || bytes[..4] != BITMAP_INDEX_MAGIC {
        return Err(StorageError::InvalidBitmapIndexFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor, IndexKind::Bitmap)? as usize;
    let mut bitmaps = BTreeMap::new();
    for _ in 0..count {
        let handle = read_u64(bytes, &mut cursor, IndexKind::Bitmap)?;
        bitmaps.insert(handle, read_set(bytes, &mut cursor, IndexKind::Bitmap)?);
    }
    if cursor != bytes.len() {
        return Err(StorageError::InvalidBitmapIndexFile);
    }
    Ok(BitmapIndex { bitmaps })
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
    if version == LEGACY_LEXICAL_INDEX_V1_MAGIC || version == LEXICAL_INDEX_MAGIC {
        let count = read_u32(bytes, &mut cursor, IndexKind::Lexical)? as usize;
        for _ in 0..count {
            let candidate = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            let length = read_u32(bytes, &mut cursor, IndexKind::Lexical)?;
            doc_lengths.insert(candidate, length);
        }
    }
    let mut term_frequencies = BTreeMap::new();
    if version == LEXICAL_INDEX_MAGIC {
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
    if cursor != bytes.len() {
        return Err(StorageError::InvalidLexicalIndexFile);
    }
    Ok(LexicalIndex {
        terms,
        doc_lengths,
        term_frequencies,
    })
}

fn is_lexical_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && (bytes[..4] == LEGACY_LEXICAL_INDEX_MAGIC
            || bytes[..4] == LEGACY_LEXICAL_INDEX_V1_MAGIC
            || bytes[..4] == LEXICAL_INDEX_MAGIC)
}

fn read_set(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<BTreeSet<u32>> {
    let count = read_u32(bytes, cursor, kind)? as usize;
    let mut values = BTreeSet::new();
    for _ in 0..count {
        values.insert(read_u32(bytes, cursor, kind)?);
    }
    Ok(values)
}

fn read_frequencies(bytes: &[u8], cursor: &mut usize) -> StorageResult<BTreeMap<u32, u32>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut values = BTreeMap::new();
    for _ in 0..count {
        let candidate = read_u32(bytes, cursor, IndexKind::Lexical)?;
        let frequency = read_u32(bytes, cursor, IndexKind::Lexical)?;
        values.insert(candidate, frequency);
    }
    Ok(values)
}

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn read_u16(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u16> {
    let raw = read_bytes(bytes, cursor, 2, kind)?;
    Ok(u16::from_le_bytes(raw.try_into().expect("u16 width")))
}

fn read_u32(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u32> {
    let raw = read_bytes(bytes, cursor, 4, kind)?;
    Ok(u32::from_le_bytes(raw.try_into().expect("u32 width")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u64> {
    let raw = read_bytes(bytes, cursor, 8, kind)?;
    Ok(u64::from_le_bytes(raw.try_into().expect("u64 width")))
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    len: usize,
    kind: IndexKind,
) -> StorageResult<&'a [u8]> {
    let end = cursor.checked_add(len).ok_or_else(|| invalid(kind))?;
    if end > bytes.len() {
        return Err(invalid(kind));
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}

fn invalid(kind: IndexKind) -> StorageError {
    match kind {
        IndexKind::Bitmap => StorageError::InvalidBitmapIndexFile,
        IndexKind::Lexical => StorageError::InvalidLexicalIndexFile,
    }
}
