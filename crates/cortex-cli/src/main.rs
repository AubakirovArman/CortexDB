use std::env;
use std::process::ExitCode;

use cortex_core::CellId;
use cortex_engine::{ContextPackOptions, Database};

mod context;

use context::{format_context_pack, view_for_scope};

fn main() -> ExitCode {
    match run(env::args().collect()) {
        Ok(output) => {
            if !output.is_empty() {
                println!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let [_, command, path, rest @ ..] = args.as_slice() else {
        return Err(usage());
    };
    match command.as_str() {
        "put" => {
            let [cell_id, payload] = rest else {
                return Err(usage());
            };
            let cell_id = parse_cell_id(cell_id)?;
            let payload = payload.as_bytes().to_vec();
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let seq = db
                .put_cell(cell_id, payload)
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        "get" => {
            let [cell_id] = rest else { return Err(usage()) };
            let cell_id = parse_cell_id(cell_id)?;
            let db = Database::open(path).map_err(|error| error.to_string())?;
            Ok(db
                .get_latest_cell(cell_id)
                .map(|payload| String::from_utf8_lossy(&payload).into_owned())
                .unwrap_or_else(|| "null".to_owned()))
        }
        "tombstone" => {
            let [cell_id] = rest else { return Err(usage()) };
            let cell_id = parse_cell_id(cell_id)?;
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        "flush" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            Ok(format!(
                "checkpoint_seq={} cells_flushed={}",
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        "compact" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let stats = db.compact().map_err(|error| error.to_string())?;
            Ok(format!(
                "checkpoint_seq={} cells_flushed={}",
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        "stats" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let stats = db.storage_stats().map_err(|error| error.to_string())?;
            Ok(format!(
                "current_seq={} checkpoint_seq={} live_segments={} retired_segments={} memtable_cells={} memtable_versions={} wal_size_bytes={}",
                stats.current_seq.0,
                stats.checkpoint_seq.0,
                stats.live_segments,
                stats.retired_segments,
                stats.memtable.cell_count,
                stats.memtable.version_count,
                stats.wal_size_bytes
            ))
        }
        "validate" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let validation = db.validate_storage().map_err(|error| error.to_string())?;
            Ok(format!(
                "ok live_segments_checked={} cells_checked={} wal_records_checked={} wal_safe_truncate_offset={}",
                validation.live_segments_checked,
                validation.cells_checked,
                validation.wal_records_checked,
                validation.wal_safe_truncate_offset
            ))
        }
        "repair" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let report = Database::repair_best_effort(path).map_err(|error| error.to_string())?;
            Ok(format!(
                "orphan_temp_files_removed={} wal_records_preserved={} wal_safe_truncate_offset={} wal_bytes_before={} wal_bytes_after={} wal_truncated={}",
                report.orphan_temp_files_removed,
                report.wal_records_preserved,
                report.wal_safe_truncate_offset,
                report.wal_bytes_before,
                report.wal_bytes_after,
                report.wal_truncated
            ))
        }
        "context" => {
            let [scope, aql] = rest else {
                return Err(usage());
            };
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let pack = db
                .context_pack_from_aql(aql, &view_for_scope(scope), ContextPackOptions::default())
                .map_err(|error| error.to_string())?;
            Ok(format_context_pack(&pack))
        }
        "unlock" => {
            let [flag] = rest else {
                return Err(usage());
            };
            if flag != "--force" {
                return Err(usage());
            }
            Database::break_stale_lock(path).map_err(|error| error.to_string())?;
            Ok("stale lock removed".to_owned())
        }
        _ => Err(usage()),
    }
}

fn parse_cell_id(value: &str) -> Result<CellId, String> {
    value
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

fn usage() -> String {
    "usage: cortexdb put <path> <cell_id> <payload> | get <path> <cell_id> | tombstone <path> <cell_id> | flush <path> | compact <path> | stats <path> | validate <path> | repair <path> | context <path> <scope> <aql> | unlock <path> --force"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn usage_is_reported_for_missing_args() {
        assert!(run(vec!["cortexdb".to_owned()])
            .unwrap_err()
            .contains("usage:"));
    }

    #[test]
    fn stats_and_validate_commands_work() {
        let path = unique_path("cortexdb-cli-stats");
        let path_arg = path.to_string_lossy().into_owned();
        run(vec![
            "cortexdb".to_owned(),
            "put".to_owned(),
            path_arg.clone(),
            "1".to_owned(),
            "hello".to_owned(),
        ])
        .unwrap();

        let stats = run(vec![
            "cortexdb".to_owned(),
            "stats".to_owned(),
            path_arg.clone(),
        ])
        .unwrap();
        assert!(stats.contains("current_seq=1"));

        let validation = run(vec!["cortexdb".to_owned(), "validate".to_owned(), path_arg]).unwrap();
        assert!(validation.starts_with("ok "));

        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn context_command_returns_pack_summary() {
        let path = unique_path("cortexdb-cli-context");
        let path_arg = path.to_string_lossy().into_owned();
        run(vec![
            "cortexdb".to_owned(),
            "put".to_owned(),
            path_arg.clone(),
            "1".to_owned(),
            "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_owned(),
        ])
        .unwrap();

        let output = run(vec![
            "cortexdb".to_owned(),
            "context".to_owned(),
            path_arg.clone(),
            "project:investments".to_owned(),
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
        ])
        .unwrap();
        assert!(output.contains("cells=1"));
        assert!(output.contains("citation=doc-a"));

        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn repair_command_reports_best_effort_cleanup() {
        let path = unique_path("cortexdb-cli-repair");
        let path_arg = path.to_string_lossy().into_owned();
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("db.aclog.tmp"), b"bad").unwrap();

        let output = run(vec![
            "cortexdb".to_owned(),
            "repair".to_owned(),
            path_arg.clone(),
        ])
        .unwrap();
        assert!(output.contains("orphan_temp_files_removed=1"));
        assert!(output.contains("wal_truncated=false"));

        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn unlock_force_removes_stale_lock() {
        let path = unique_path("cortexdb-cli-unlock");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("db.lock"), b"stale").unwrap();
        let path_arg = path.to_string_lossy().into_owned();

        let output = run(vec![
            "cortexdb".to_owned(),
            "unlock".to_owned(),
            path_arg,
            "--force".to_owned(),
        ])
        .unwrap();
        assert_eq!(output, "stale lock removed");
        assert!(!path.join("db.lock").exists());

        let _ = std::fs::remove_dir_all(path);
    }

    fn unique_path(prefix: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
