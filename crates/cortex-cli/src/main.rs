use std::env;
use std::process::ExitCode;

use cortex_core::CellId;
use cortex_engine::{ContextPackOptions, Database, SearchLimit};

mod context;
#[cfg(test)]
mod tests;
mod wal;

use context::{
    format_context_pack, format_retrieved_cells, format_search_results, format_verification_report,
    remember_view_for_scope, verify_view_for_scope, view_for_scope,
};

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
        "gc-retired" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let report = db
                .garbage_collect_retired_segments()
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "retired_segments_removed={} files_removed={}",
                report.retired_segments_removed, report.files_removed
            ))
        }
        "wal-validate" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            wal::validate(path)
        }
        "wal-dump" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            wal::dump(path)
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
        "remember" => {
            let [scope, aql] = rest else {
                return Err(usage());
            };
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let result = db
                .remember_aql(aql, &remember_view_for_scope(scope))
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "seq={} cell_id={} ttl_seconds={}",
                result.commit_seq.0,
                result.cell_id.0,
                result
                    .ttl_seconds
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "null".to_owned())
            ))
        }
        "verify" => {
            let [scope, aql] = rest else {
                return Err(usage());
            };
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let report = db
                .verify_fact_aql(aql, &verify_view_for_scope(scope))
                .map_err(|error| error.to_string())?;
            Ok(format_verification_report(&report))
        }
        "aql" => {
            let [scope, aql] = rest else {
                return Err(usage());
            };
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let cells = db
                .retrieve_aql(aql, &view_for_scope(scope))
                .map_err(|error| error.to_string())?;
            Ok(format_retrieved_cells(&cells))
        }
        "search" => {
            let [scope, query] = rest else {
                return Err(usage());
            };
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let results = db
                .search_keyword(query, &view_for_scope(scope), SearchLimit(20))
                .map_err(|error| error.to_string())?;
            Ok(format_search_results(&results))
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
    "usage: cortexdb put <path> <cell_id> <payload> | get <path> <cell_id> | tombstone <path> <cell_id> | flush <path> | compact <path> | stats <path> | validate <path> | repair <path> | gc-retired <path> | wal-validate <path> | wal-dump <path> | context <path> <scope> <aql> | remember <path> <scope> <aql> | verify <path> <scope> <aql> | aql <path> <scope> <aql> | search <path> <scope> <query> | unlock <path> --force"
        .to_owned()
}
