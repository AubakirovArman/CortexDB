use std::path::Path;

use crate::cli_doctor_checks::{
    auth_check, backup_age_check, format_versions_check, lock_check_after_open,
    lock_check_without_open, memory_forecast_check, repair_advice, server_health_check,
    tenant_check, validate_tenant_id, wal_check, DoctorCheck,
};
use crate::cli_ops::open_database;

pub(crate) fn doctor(path: &str, tenant: Option<&str>) -> Result<String, String> {
    let db_path = Path::new(path);
    let mut checks = Vec::new();
    let mut all_ok = true;

    let tenant_result = tenant_check(tenant, db_path);
    all_ok &= tenant_result.ok;
    checks.push(tenant_result);

    let db = match open_database(path, false) {
        Ok(db) => {
            checks.push(DoctorCheck::ok("open", "database opened successfully"));
            db
        }
        Err(error) => {
            checks.push(DoctorCheck::fail(
                "open",
                format!("failed to open: {error}"),
            ));
            checks.push(lock_check_without_open(db_path));
            checks.push(repair_advice(false, db_path));
            return Ok(format_doctor_report(checks, false));
        }
    };

    checks.push(lock_check_after_open(db_path));

    let storage_stats = match db.storage_stats() {
        Ok(stats) => {
            checks.push(DoctorCheck::ok(
                "storage_stats",
                format!(
                    "seq={} segments={} memtable_cells={}",
                    stats.current_seq.0, stats.live_segments, stats.memtable.cell_count
                ),
            ));
            Some(stats)
        }
        Err(error) => {
            checks.push(DoctorCheck::fail("storage_stats", error.to_string()));
            all_ok = false;
            None
        }
    };
    if let Some(stats) = storage_stats.as_ref() {
        let check = memory_forecast_check(stats);
        all_ok &= check.ok;
        checks.push(check);
    }

    let report = db.validate_storage_report();
    let wal = wal_check(&report, db_path);
    all_ok &= wal.ok;
    checks.push(wal);
    if report.errors.is_empty() {
        checks.push(DoctorCheck::ok(
            "validate",
            format!(
                "cells={} wal_records={}",
                report.cells_checked, report.wal_records_checked
            ),
        ));
    } else {
        checks.push(DoctorCheck::fail("validate", report.errors.join("; ")));
        all_ok = false;
    }

    for check in [
        format_versions_check(),
        backup_age_check(db_path),
        server_health_check(),
        auth_check(),
    ] {
        all_ok &= check.ok;
        checks.push(check);
    }

    let ann = db.ann_metrics();
    checks.push(DoctorCheck::ok(
        "ann_metrics",
        format!(
            "graph_nodes={} persisted_segments={} has_checkpoint={}",
            ann.graph_nodes, ann.persisted_segments, ann.has_checkpoint
        ),
    ));

    checks.push(repair_advice(all_ok, db_path));
    Ok(format_doctor_report(checks, all_ok))
}

pub(crate) fn is_valid_tenant_arg(tenant: &str) -> bool {
    tenant == "default" || validate_tenant_id(tenant)
}

fn format_doctor_report(checks: Vec<DoctorCheck>, all_ok: bool) -> String {
    let mut lines = vec![
        "CortexDB Doctor Report".to_owned(),
        "======================".to_owned(),
    ];
    for check in checks {
        let status = if check.ok { "✅" } else { "❌" };
        lines.push(format!("{status} {}: {}", check.name, check.detail));
    }
    lines.push(String::new());
    if all_ok {
        lines.push("All checks passed. Database is healthy.".to_owned());
    } else {
        lines.push("Some checks failed. See details above.".to_owned());
    }
    lines.join("\n")
}
