use std::path::Path;

use cortex_engine::Database;
use cortex_storage::wal::{WalDiagnostics, WalReader};

pub fn validate(root: &str) -> Result<String, String> {
    let path = wal_path(root);
    let scan = WalReader::scan_path(&path).map_err(|error| error.to_string())?;
    let summary = WalDiagnostics::summarize(&scan);
    Ok(format!(
        "ok records={} safe_truncate_offset={} total_payload_bytes={} known_sections={} unknown_sections={}",
        summary.records,
        summary.safe_truncate_offset,
        summary.total_payload_bytes,
        summary.known_sections,
        summary.unknown_sections
    ))
}

pub fn dump(root: &str) -> Result<String, String> {
    let path = wal_path(root);
    let scan = WalReader::scan_path(&path).map_err(|error| error.to_string())?;
    let mut lines = Vec::new();
    for record in scan.records {
        lines.push(format!(
            "lsn={} type={:?} payload_len={} sections={}",
            record.lsn,
            record.header.record_type,
            record.header.payload_len,
            record.sections.len()
        ));
    }
    if lines.is_empty() {
        Ok("empty".to_owned())
    } else {
        Ok(lines.join("\n"))
    }
}

pub fn truncate(root: &str) -> Result<String, String> {
    let report = Database::repair_best_effort(root).map_err(|error| error.to_string())?;
    Ok(format!(
        "wal_records_preserved={} wal_safe_truncate_offset={} wal_bytes_before={} wal_bytes_after={} wal_truncated={}",
        report.wal_records_preserved,
        report.wal_safe_truncate_offset,
        report.wal_bytes_before,
        report.wal_bytes_after,
        report.wal_truncated
    ))
}

fn wal_path(root: &str) -> std::path::PathBuf {
    Path::new(root).join("db.aclog")
}
