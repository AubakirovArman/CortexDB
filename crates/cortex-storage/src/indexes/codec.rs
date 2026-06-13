use std::collections::{BTreeMap, BTreeSet};

use crate::error::{StorageError, StorageResult};

#[derive(Clone, Copy, Debug)]
pub(super) enum IndexKind {
    Bitmap,
    Lexical,
}

pub(super) fn read_set(
    bytes: &[u8],
    cursor: &mut usize,
    kind: IndexKind,
) -> StorageResult<BTreeSet<u32>> {
    let count = read_u32(bytes, cursor, kind)? as usize;
    let mut values = BTreeSet::new();
    for _ in 0..count {
        values.insert(read_u32(bytes, cursor, kind)?);
    }
    Ok(values)
}

pub(super) fn read_frequencies(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<BTreeMap<u32, u32>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut values = BTreeMap::new();
    for _ in 0..count {
        let candidate = read_u32(bytes, cursor, IndexKind::Lexical)?;
        let frequency = read_u32(bytes, cursor, IndexKind::Lexical)?;
        values.insert(candidate, frequency);
    }
    Ok(values)
}

pub(super) fn read_field_term_frequencies(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<BTreeMap<String, BTreeMap<u32, u32>>> {
    let count = read_u32(bytes, cursor, IndexKind::Lexical)? as usize;
    let mut terms = BTreeMap::new();
    for _ in 0..count {
        let len = read_u16(bytes, cursor, IndexKind::Lexical)? as usize;
        let term = std::str::from_utf8(read_bytes(bytes, cursor, len, IndexKind::Lexical)?)
            .map_err(|_| StorageError::InvalidLexicalIndexFile)?
            .to_owned();
        terms.insert(term, read_frequencies(bytes, cursor)?);
    }
    Ok(terms)
}

pub(super) fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn read_u16(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u16> {
    Ok(u16::from_le_bytes(read_array(bytes, cursor, kind)?))
}

pub(super) fn read_u32(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u32> {
    Ok(u32::from_le_bytes(read_array(bytes, cursor, kind)?))
}

pub(super) fn read_u64(bytes: &[u8], cursor: &mut usize, kind: IndexKind) -> StorageResult<u64> {
    Ok(u64::from_le_bytes(read_array(bytes, cursor, kind)?))
}

fn read_array<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    kind: IndexKind,
) -> StorageResult<[u8; N]> {
    read_bytes(bytes, cursor, N, kind)?
        .try_into()
        .map_err(|_| invalid(kind))
}

pub(super) fn read_bytes<'a>(
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
