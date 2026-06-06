use crate::error::{StorageError, StorageResult};

use super::checksum::{align8, crc32c};
use super::record::{
    AclogMagic, DecodedSection, DecodedWalRecord, SectionEntry, SectionTag, WalCodecVersion,
    WalFileHeader, WalFlags, WalRecord, WalRecordHeader, WalRecordType, WalSection, ACLOG_MAGIC,
    WAL_FILE_HEADER_LEN, WAL_FORMAT_VERSION, WAL_RECORD_HEADER_LEN, WAL_RECORD_MAGIC,
    WAL_SECTION_ENTRY_LEN,
};

#[derive(Clone, Debug)]
pub struct WalCodec;

impl WalCodec {
    pub fn file_header() -> [u8; WAL_FILE_HEADER_LEN] {
        encode_file_header(WalFileHeader {
            magic: AclogMagic(ACLOG_MAGIC),
            header_len: WAL_FILE_HEADER_LEN as u16,
            version: WalCodecVersion(WAL_FORMAT_VERSION),
            flags: WalFlags::default(),
        })
    }

    pub fn validate_file_header(bytes: &[u8]) -> StorageResult<WalFileHeader> {
        if bytes.len() != WAL_FILE_HEADER_LEN || bytes[..8] != ACLOG_MAGIC {
            return Err(StorageError::InvalidWalFileHeader);
        }
        let header = WalFileHeader {
            magic: AclogMagic(ACLOG_MAGIC),
            header_len: get_file_u16(&bytes[8..10])?,
            version: WalCodecVersion(get_file_u16(&bytes[10..12])?),
            flags: WalFlags(get_file_u16(&bytes[12..14])?),
        };
        if usize::from(header.header_len) != WAL_FILE_HEADER_LEN
            || header.version.0 != WAL_FORMAT_VERSION
        {
            return Err(StorageError::InvalidWalFileHeader);
        }
        Ok(header)
    }

    pub fn file_header_len() -> usize {
        WAL_FILE_HEADER_LEN
    }

    pub fn encode_record(record: &WalRecord) -> StorageResult<Vec<u8>> {
        Self::encode_record_at(record, 0)
    }

    pub fn encode_record_at(record: &WalRecord, lsn: u64) -> StorageResult<Vec<u8>> {
        let (entries, payload) = encode_sections(&record.sections)?;
        let header_len = WAL_RECORD_HEADER_LEN + entries.len() * WAL_SECTION_ENTRY_LEN;
        let total_len = header_len + payload.len();
        let mut out = vec![0u8; total_len];
        put_u32(&mut out[0..4], WAL_RECORD_MAGIC);
        put_u16(&mut out[4..6], header_len as u16);
        put_u16(&mut out[6..8], record.record_type.to_u16());
        put_u64(&mut out[8..16], lsn);
        put_u32(&mut out[16..20], payload.len() as u32);
        put_u16(&mut out[20..22], entries.len() as u16);
        put_u16(&mut out[22..24], WalFlags::default().0);
        put_u32(&mut out[24..28], crc32c(&payload));
        for (index, entry) in entries.iter().enumerate() {
            encode_entry(
                &mut out[WAL_RECORD_HEADER_LEN + index * WAL_SECTION_ENTRY_LEN..],
                entry,
            );
        }
        let header_crc = crc32c(&out[..header_len]);
        put_u32(&mut out[28..32], header_crc);
        out[header_len..].copy_from_slice(&payload);
        Ok(out)
    }

    pub fn scan_next(bytes: &[u8]) -> StorageResult<Option<DecodedWalRecord>> {
        if bytes.is_empty() {
            return Ok(None);
        }
        Self::decode_record(bytes).map(Some)
    }

    pub fn decode_record(bytes: &[u8]) -> StorageResult<DecodedWalRecord> {
        if bytes.len() < WAL_RECORD_HEADER_LEN {
            return Err(StorageError::IncompleteTail);
        }
        if get_u32(&bytes[0..4])? != WAL_RECORD_MAGIC {
            return Err(StorageError::InvalidWalRecord);
        }
        let raw_record_type = get_u16(&bytes[6..8])?;
        let header = WalRecordHeader {
            magic: WAL_RECORD_MAGIC,
            header_len: get_u16(&bytes[4..6])?,
            record_type: WalRecordType::from_u16(raw_record_type)
                .ok_or(StorageError::InvalidWalRecord)?,
            flags: WalFlags(get_u16(&bytes[22..24])?),
            lsn: get_u64(&bytes[8..16])?,
            payload_len: get_u32(&bytes[16..20])?,
            section_count: get_u16(&bytes[20..22])?,
            payload_crc32c: get_u32(&bytes[24..28])?,
            header_crc32c: get_u32(&bytes[28..32])?,
        };
        let header_len = usize::from(header.header_len);
        let payload_len = header.payload_len as usize;
        let section_count = usize::from(header.section_count);
        validate_lengths(bytes, header_len, payload_len, section_count)?;
        validate_header_crc(bytes, header_len, header.header_crc32c)?;
        let payload = &bytes[header_len..header_len + payload_len];
        if crc32c(payload) != header.payload_crc32c {
            return Err(StorageError::WalChecksumMismatch);
        }
        let entries = decode_entries(&bytes[WAL_RECORD_HEADER_LEN..header_len], section_count)?;
        let sections = decode_sections(payload, &entries)?;
        Ok(DecodedWalRecord {
            lsn: header.lsn,
            header,
            record: WalRecord {
                record_type: header.record_type,
                sections: known_sections(&sections),
            },
            sections,
            bytes_consumed: header_len + payload_len,
        })
    }
}

fn encode_file_header(header: WalFileHeader) -> [u8; WAL_FILE_HEADER_LEN] {
    let mut out = [0u8; WAL_FILE_HEADER_LEN];
    out[..8].copy_from_slice(&header.magic.0);
    put_u16(&mut out[8..10], header.header_len);
    put_u16(&mut out[10..12], header.version.0);
    put_u16(&mut out[12..14], header.flags.0);
    out
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

fn decode_sections(payload: &[u8], entries: &[SectionEntry]) -> StorageResult<Vec<DecodedSection>> {
    let mut sections = Vec::new();
    for entry in entries {
        if entry.offset % 8 != 0 {
            return Err(StorageError::InvalidWalRecord);
        }
        let start = entry.offset as usize;
        let end = start
            .checked_add(entry.len as usize)
            .ok_or(StorageError::InvalidWalRecord)?;
        if end > payload.len() {
            return Err(StorageError::InvalidWalRecord);
        }
        sections.push(DecodedSection {
            tag_raw: entry.tag_raw,
            tag: SectionTag::from_u16(entry.tag_raw),
            data: payload[start..end].to_vec(),
        });
    }
    Ok(sections)
}

fn known_sections(sections: &[DecodedSection]) -> Vec<WalSection> {
    sections
        .iter()
        .filter_map(|section| {
            section
                .tag
                .map(|tag| WalSection::new(tag, section.data.clone()))
        })
        .collect()
}

fn validate_lengths(
    bytes: &[u8],
    header_len: usize,
    payload_len: usize,
    section_count: usize,
) -> StorageResult<()> {
    if header_len != WAL_RECORD_HEADER_LEN + section_count * WAL_SECTION_ENTRY_LEN {
        return Err(StorageError::InvalidWalRecord);
    }
    if bytes.len() < header_len + payload_len {
        return Err(StorageError::IncompleteTail);
    }
    Ok(())
}

fn validate_header_crc(bytes: &[u8], header_len: usize, expected: u32) -> StorageResult<()> {
    let mut header = bytes[..header_len].to_vec();
    put_u32(&mut header[28..32], 0);
    if crc32c(&header) != expected {
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
        let start = index * WAL_SECTION_ENTRY_LEN;
        let raw = &bytes[start..start + WAL_SECTION_ENTRY_LEN];
        entries.push(SectionEntry {
            tag_raw: get_u16(&raw[0..2])?,
            offset: get_u32(&raw[4..8])?,
            len: get_u32(&raw[8..12])?,
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

fn get_file_u16(bytes: &[u8]) -> StorageResult<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| StorageError::InvalidWalFileHeader)?,
    ))
}

fn get_u16(bytes: &[u8]) -> StorageResult<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| StorageError::InvalidWalRecord)?,
    ))
}

fn get_u32(bytes: &[u8]) -> StorageResult<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| StorageError::InvalidWalRecord)?,
    ))
}

fn get_u64(bytes: &[u8]) -> StorageResult<u64> {
    Ok(u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| StorageError::InvalidWalRecord)?,
    ))
}
