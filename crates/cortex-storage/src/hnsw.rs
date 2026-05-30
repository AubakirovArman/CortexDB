use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::atomic::{append_crc32c, verify_crc32c, write_atomic};
use crate::error::{StorageError, StorageResult};
use crate::format::HNSW_GRAPH_MAGIC;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HnswGraphIndex {
    pub links: BTreeMap<u32, BTreeSet<u32>>,
    pub dimension: u32,
    pub metric: u8,
    pub upper_layers: BTreeMap<u32, BTreeMap<u32, BTreeSet<u32>>>,
}

impl HnswGraphIndex {
    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        let mut out = Vec::from(&HNSW_GRAPH_MAGIC[..]);
        put_len_u32(&mut out, self.links.len())?;
        for (candidate, neighbors) in &self.links {
            put_u32(&mut out, *candidate);
            put_len_u32(&mut out, neighbors.len())?;
            for neighbor in neighbors {
                put_u32(&mut out, *neighbor);
            }
        }
        put_u32(&mut out, self.dimension);
        put_u32(&mut out, u32::from(self.metric));
        put_len_u32(&mut out, self.upper_layers.len())?;
        for (layer, links) in &self.upper_layers {
            put_u32(&mut out, *layer);
            put_len_u32(&mut out, links.len())?;
            for (candidate, neighbors) in links {
                put_u32(&mut out, *candidate);
                put_len_u32(&mut out, neighbors.len())?;
                for neighbor in neighbors {
                    put_u32(&mut out, *neighbor);
                }
            }
        }
        append_crc32c(&mut out);
        write_atomic(path.as_ref(), &out)
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        decode(&std::fs::read(path)?)
    }
}

fn decode(bytes: &[u8]) -> StorageResult<HnswGraphIndex> {
    let bytes = verify_crc32c(bytes).ok_or(StorageError::InvalidHnswGraphFile)?;
    if bytes.len() < 8 || bytes[..4] != HNSW_GRAPH_MAGIC {
        return Err(StorageError::InvalidHnswGraphFile);
    }
    let mut cursor = 4;
    let count = read_u32(bytes, &mut cursor)? as usize;
    let mut links = BTreeMap::new();
    for _ in 0..count {
        let candidate = read_u32(bytes, &mut cursor)?;
        let neighbors = read_set(bytes, &mut cursor)?;
        if candidate == 0 || links.insert(candidate, neighbors).is_some() {
            return Err(StorageError::InvalidHnswGraphFile);
        }
    }
    let (dimension, metric) = if bytes.len() >= cursor + 8 {
        (
            read_u32(bytes, &mut cursor)?,
            read_u32(bytes, &mut cursor)? as u8,
        )
    } else {
        (0, 0)
    };
    let upper_layers = if bytes.len() > cursor {
        read_layers(bytes, &mut cursor)?
    } else {
        BTreeMap::new()
    };
    if cursor != bytes.len() {
        return Err(StorageError::InvalidHnswGraphFile);
    }
    Ok(HnswGraphIndex {
        links,
        dimension,
        metric,
        upper_layers,
    })
}

fn read_layers(
    bytes: &[u8],
    cursor: &mut usize,
) -> StorageResult<BTreeMap<u32, BTreeMap<u32, BTreeSet<u32>>>> {
    let count = read_u32(bytes, cursor)? as usize;
    let mut layers = BTreeMap::new();
    for _ in 0..count {
        let layer = read_u32(bytes, cursor)?;
        if layer == 0 {
            return Err(StorageError::InvalidHnswGraphFile);
        }
        let node_count = read_u32(bytes, cursor)? as usize;
        let mut links = BTreeMap::new();
        for _ in 0..node_count {
            let candidate = read_u32(bytes, cursor)?;
            let neighbors = read_set(bytes, cursor)?;
            if candidate == 0 || links.insert(candidate, neighbors).is_some() {
                return Err(StorageError::InvalidHnswGraphFile);
            }
        }
        if layers.insert(layer, links).is_some() {
            return Err(StorageError::InvalidHnswGraphFile);
        }
    }
    Ok(layers)
}

fn read_set(bytes: &[u8], cursor: &mut usize) -> StorageResult<BTreeSet<u32>> {
    let count = read_u32(bytes, cursor)? as usize;
    let mut values = BTreeSet::new();
    for _ in 0..count {
        let value = read_u32(bytes, cursor)?;
        if value == 0 || !values.insert(value) {
            return Err(StorageError::InvalidHnswGraphFile);
        }
    }
    Ok(values)
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_len_u32(out: &mut Vec<u8>, value: usize) -> StorageResult<()> {
    let value = u32::try_from(value).map_err(|_| StorageError::InvalidHnswGraphFile)?;
    put_u32(out, value);
    Ok(())
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> StorageResult<u32> {
    let raw = read_bytes(bytes, cursor, 4)?;
    let raw = raw
        .try_into()
        .map_err(|_| StorageError::InvalidHnswGraphFile)?;
    Ok(u32::from_le_bytes(raw))
}

fn read_bytes<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> StorageResult<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(StorageError::InvalidHnswGraphFile)?;
    if end > bytes.len() {
        return Err(StorageError::InvalidHnswGraphFile);
    }
    let value = &bytes[*cursor..end];
    *cursor = end;
    Ok(value)
}
