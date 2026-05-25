use super::reader::WalScan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalScanSummary {
    pub records: usize,
    pub safe_truncate_offset: u64,
    pub total_payload_bytes: u64,
    pub known_sections: usize,
    pub unknown_sections: usize,
    pub last_lsn: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct WalDiagnostics;

impl WalDiagnostics {
    pub fn summarize(scan: &WalScan) -> WalScanSummary {
        let mut total_payload_bytes = 0;
        let mut known_sections = 0;
        let mut unknown_sections = 0;
        let mut last_lsn = None;

        for record in &scan.records {
            total_payload_bytes += u64::from(record.header.payload_len);
            last_lsn = Some(record.lsn);
            for section in &record.sections {
                if section.tag.is_some() {
                    known_sections += 1;
                } else {
                    unknown_sections += 1;
                }
            }
        }

        WalScanSummary {
            records: scan.records.len(),
            safe_truncate_offset: scan.safe_truncate_offset,
            total_payload_bytes,
            known_sections,
            unknown_sections,
            last_lsn,
        }
    }
}
