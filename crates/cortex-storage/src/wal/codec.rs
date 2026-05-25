use crate::error::{StorageError, StorageResult};

use super::checksum::{align8, crc32c};
use super::record::{
    DecodedWalRecord, SectionEntry, SectionTag, WalRecord, WalRecordType, WalSection,
};

const FILE_MAGIC: &[u8; 8] = b"ACLOGv0\0";
const RECORD_MAGIC: u32 = 0x4143_4c52;
const FILE_HEADER_LEN: usize = 16;
const RECORD_HEADER_LEN: usize = 32;
const SECTION_ENTRY_LEN: usize = 16;

#[derive(Clone, Debug)]
pub struct WalCodec;

impl WalCodec {
    pub fn file_header() -> [u8; FILE_HEADER_LEN] {
        let mut out = [0u8; FILE_HEADER_LEN];
        out[..8].copy_from_slice(FILE_MAGIC);
        out[8..10].copy_from_slice(&(FILE_HEADER_LEN as u16).to_le_bytes());
        out[10..12].copy_from_slice(&0u16.to_le_bytes());
        out
    }

    pub fn validate_file_header(bytes: &[u8]) -> StorageResult<()> {
        if bytes.len() != FILE_HEADER_LEN || &bytes[..8] != FILE_MAGIC {
            return Err(StorageError::InvalidWalFileHeader);
        }
        Ok(())
    }

    pub fn file_header_len() -> usize {
        FILE_HEADER_LEN
    }

    pub fn encode_record(record: &WalRecord, lsn: u64) -> StorageResult<Vec<u8>> {
        let (entries, payload) = encode_sections(&record.sections)?;
        let header_len = RECORD_HEADER_LEN + entries.len() * SECTION_ENTRY_LEN;
        let total_len = header_len + payload.len();
        let mut out = vec![0u8; total_len];
        put_u32(&mut out[0..4], RECORD_MAGIC);
        put_u16(&mut out[4..6], header_len as u16);
        put_u16(&mut out[6..8], record.record_type.to_u16());
        put_u64(&mut out[8..16], lsn);
        put_u32(&mut out[16..20], payload.len() as u32);
        put_u16(&mut out[20..22], entries.len() as u16);
        put_u32(&mut out[24..28], crc32c(&payload));
        for (index, entry) in entries.iter().enumerate() {
            encode_entry(
                &mut out[RECORD_HEADER_LEN + index * SECTION_ENTRY_LEN..],
                entry,
            );
        }
        let header_crc = crc32c(&out[..header_len]);
        put_u32(&mut out[28..32], header_crc);
        out[header_len..].copy_from_slice(&payload);
        Ok(out)
    }

    pub fn decode_record(bytes: &[u8]) -> StorageResult<DecodedWalRecord> {
        if bytes.len() < RECORD_HEADER_LEN || get_u32(&bytes[0..4]) != RECORD_MAGIC {
            return Err(StorageError::InvalidWalRecord);
        }
        let header_len = usize::from(get_u16(&bytes[4..6]));
        let record_type = get_u16(&bytes[6..8]);
        let lsn = get_u64(&bytes[8..16]);
        let payload_len = get_u32(&bytes[16..20]) as usize;
        let section_count = usize::from(get_u16(&bytes[20..22]));
        let payload_crc = get_u32(&bytes[24..28]);
        let header_crc = get_u32(&bytes[28..32]);
        validate_lengths(bytes, header_len, payload_len, section_count)?;
        let mut header = bytes[..header_len].to_vec();
        put_u32(&mut header[28..32], 0);
        if crc32c(&header) != header_crc {
            return Err(StorageError::WalChecksumMismatch);
        }
        let payload = &bytes[header_len..header_len + payload_len];
        if crc32c(payload) != payload_crc {
            return Err(StorageError::WalChecksumMismatch);
        }
        let entries = decode_entries(&bytes[RECORD_HEADER_LEN..header_len], section_count)?;
        Ok(DecodedWalRecord {
            lsn,
            record: WalRecord {
                record_type: WalRecordType::from_u16(record_type)
                    .ok_or(StorageError::InvalidWalRecord)?,
                sections: decode_sections(payload, &entries)?,
            },
            bytes_consumed: header_len + payload_len,
        })
    }
}

fn encode_sections(sections: &[WalSection]) -> StorageResult<(Vec<SectionEntry>, Vec<u8>)> {
    let mut entries = Vec::with_capacity(sections.len());
    let mut payload = Vec::new();
    for section in sections {
        payload.resize(align8(payload.len()), 0);
        let offset = payload.len();
        payload.extend_from_slice(&section.data);
        entries.push(SectionEntry {
            tag_raw: section.tag.to_u16(),
            offset: offset as u32,
            len: section.data.len() as u32,
        });
    }
    Ok((entries, payload))
}

fn decode_sections(payload: &[u8], entries: &[SectionEntry]) -> StorageResult<Vec<WalSection>> {
    let mut sections = Vec::new();
    for entry in entries {
        let Some(tag) = SectionTag::from_u16(entry.tag_raw) else {
            continue;
        };
        let start = entry.offset as usize;
        let end = start
            .checked_add(entry.len as usize)
            .ok_or(StorageError::InvalidWalRecord)?;
        if end > payload.len() {
            return Err(StorageError::InvalidWalRecord);
        }
        sections.push(WalSection::new(tag, payload[start..end].to_vec()));
    }
    Ok(sections)
}

fn validate_lengths(
    bytes: &[u8],
    header_len: usize,
    payload_len: usize,
    section_count: usize,
) -> StorageResult<()> {
    if header_len != RECORD_HEADER_LEN + section_count * SECTION_ENTRY_LEN {
        return Err(StorageError::InvalidWalRecord);
    }
    if bytes.len() < header_len + payload_len {
        return Err(StorageError::InvalidWalRecord);
    }
    Ok(())
}

fn encode_entry(out: &mut [u8], entry: &SectionEntry) {
    put_u16(&mut out[0..2], entry.tag_raw);
    put_u32(&mut out[4..8], entry.offset);
    put_u32(&mut out[8..12], entry.len);
}

fn decode_entries(bytes: &[u8], count: usize) -> StorageResult<Vec<SectionEntry>> {
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let start = index * SECTION_ENTRY_LEN;
        let raw = &bytes[start..start + SECTION_ENTRY_LEN];
        entries.push(SectionEntry {
            tag_raw: get_u16(&raw[0..2]),
            offset: get_u32(&raw[4..8]),
            len: get_u32(&raw[8..12]),
        });
    }
    Ok(entries)
}

fn put_u16(out: &mut [u8], value: u16) {
    out.copy_from_slice(&value.to_le_bytes());
}

fn put_u32(out: &mut [u8], value: u32) {
    out.copy_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut [u8], value: u64) {
    out.copy_from_slice(&value.to_le_bytes());
}

fn get_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("u16 slice length"))
}

fn get_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("u32 slice length"))
}

fn get_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("u64 slice length"))
}
