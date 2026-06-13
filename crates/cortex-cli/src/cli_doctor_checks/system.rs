use std::fs;
use std::path::Path;

use cortex_engine::{compatibility_summary, StorageStats, StorageValidationReport};

use super::DoctorCheck;

pub(crate) fn wal_check(report: &StorageValidationReport, path: &Path) -> DoctorCheck {
    if report.wal_ok {
        DoctorCheck::ok(
            "wal",
            format!(
                "records_checked={} safe_truncate_offset={}",
                report.wal_records_checked, report.wal_safe_truncate_offset
            ),
        )
    } else {
        DoctorCheck::fail(
            "wal",
            format!(
                "WAL validation failed; run cortexdb wal-validate {} then cortexdb repair --dry-run {}",
                path.display(),
                path.display()
            ),
        )
    }
}

pub(crate) fn format_versions_check() -> DoctorCheck {
    let summary = compatibility_summary();
    let markers = summary
        .storage_formats
        .iter()
        .map(|format| format.current_magic.as_str())
        .collect::<Vec<_>>()
        .join(",");
    DoctorCheck::ok(
        "format_versions",
        format!(
            "schema={} storage_formats={} current_markers={markers}",
            summary.schema_version,
            summary.storage_formats.len()
        ),
    )
}

pub(crate) fn memory_forecast_check(stats: &StorageStats) -> DoctorCheck {
    memory_forecast_check_bytes(
        stats.estimated_total_memory_bytes as u64,
        available_memory_bytes(),
    )
}

fn memory_forecast_check_bytes(estimated: u64, available: Option<u64>) -> DoctorCheck {
    match available {
        Some(bytes) if estimated > bytes => DoctorCheck::fail(
            "memory_forecast",
            format!(
                "estimated_total_memory_bytes={estimated} exceeds available_memory_bytes={bytes}; reduce cache/payload residency or run on a larger host"
            ),
        ),
        Some(bytes) => DoctorCheck::ok(
            "memory_forecast",
            format!("estimated_total_memory_bytes={estimated} available_memory_bytes={bytes}"),
        ),
        None => DoctorCheck::ok(
            "memory_forecast",
            format!("estimated_total_memory_bytes={estimated} available_memory_bytes=unknown"),
        ),
    }
}

fn available_memory_bytes() -> Option<u64> {
    if let Ok(value) = std::env::var("CORTEXDB_DOCTOR_AVAILABLE_MEMORY_BYTES") {
        return value.trim().parse().ok();
    }
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    meminfo.lines().find_map(|line| {
        let (key, rest) = line.split_once(':')?;
        if key != "MemAvailable" {
            return None;
        }
        let kib = rest.split_whitespace().next()?.parse::<u64>().ok()?;
        kib.checked_mul(1024)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wal_check_reports_corrupt_wal_advice() {
        let report = StorageValidationReport {
            wal_ok: false,
            wal_records_checked: 2,
            wal_safe_truncate_offset: 128,
            ..StorageValidationReport::default()
        };
        let check = wal_check(&report, Path::new("/tmp/cortexdb"));
        assert!(!check.ok);
        assert!(check.detail.contains("wal-validate"));
        assert!(check.detail.contains("repair --dry-run"));
    }

    #[test]
    fn memory_forecast_fails_when_estimate_exceeds_available_memory() {
        let check = memory_forecast_check_bytes(2048, Some(1024));
        assert!(!check.ok);
        assert!(check.detail.contains("exceeds available_memory_bytes"));
    }

    #[test]
    fn format_versions_check_exposes_current_storage_markers() {
        let check = format_versions_check();
        assert!(check.ok);
        assert!(check.detail.contains("storage_formats="));
        assert!(check.detail.contains("ACLOG"));
    }
}
