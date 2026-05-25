use std::env;
use std::process::ExitCode;

use cortex_core::CellId;
use cortex_engine::{parse_vector_literal, ContextPackOptions, Database, SearchLimit};

mod context;
#[cfg(test)]
mod json_tests;
mod manifest;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_tests;
#[cfg(test)]
mod vector_tests;
mod wal;

use context::{
    context_pack_to_json, format_context_pack, format_retrieved_cells, format_search_results,
    format_verification_report, remember_view_for_scope, verification_report_to_json,
    verify_view_for_scope, view_for_scope,
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
    if args.len() == 2 && args[1] == "demo" {
        let output = std::process::Command::new("./examples/demo/investment_projects/run.sh")
            .output()
            .map_err(|e| format!("Failed to run demo script: {}", e))?;
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr));
    }

    let [_, command, path, rest @ ..] = args.as_slice() else {
        return Err(usage());
    };
    let mut rest_vec = rest.to_vec();
    let json_flag = rest_vec
        .iter()
        .position(|r| r == "--json")
        .map(|idx| {
            rest_vec.remove(idx);
            true
        })
        .unwrap_or(false);
    let rest = rest_vec.as_slice();

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
            if json_flag {
                Ok(format!(
                    r#"{{"current_seq":{},"checkpoint_seq":{},"live_segments":{},"retired_segments":{},"memtable_cells":{},"memtable_versions":{},"wal_size_bytes":{},"wal_writer_records":{},"wal_writer_bytes":{},"wal_writer_fsyncs":{},"wal_writer_batches":{}}}"#,
                    stats.current_seq.0,
                    stats.checkpoint_seq.0,
                    stats.live_segments,
                    stats.retired_segments,
                    stats.memtable.cell_count,
                    stats.memtable.version_count,
                    stats.wal_size_bytes,
                    stats.wal_writer.records_written,
                    stats.wal_writer.bytes_written,
                    stats.wal_writer.fsync_count,
                    stats.wal_writer.batches_committed
                ))
            } else {
                Ok(format!(
                    "current_seq={} checkpoint_seq={} live_segments={} retired_segments={} memtable_cells={} memtable_versions={} wal_size_bytes={} wal_writer_records={} wal_writer_bytes={} wal_writer_fsyncs={} wal_writer_batches={}",
                    stats.current_seq.0,
                    stats.checkpoint_seq.0,
                    stats.live_segments,
                    stats.retired_segments,
                    stats.memtable.cell_count,
                    stats.memtable.version_count,
                    stats.wal_size_bytes,
                    stats.wal_writer.records_written,
                    stats.wal_writer.bytes_written,
                    stats.wal_writer.fsync_count,
                    stats.wal_writer.batches_committed
                ))
            }
        }
        "validate" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let validation = db.validate_storage().map_err(|error| error.to_string())?;
            if json_flag {
                Ok(format!(
                    r#"{{"ok":true,"live_segments_checked":{},"cells_checked":{},"wal_records_checked":{},"wal_safe_truncate_offset":{}}}"#,
                    validation.live_segments_checked,
                    validation.cells_checked,
                    validation.wal_records_checked,
                    validation.wal_safe_truncate_offset
                ))
            } else {
                Ok(format!(
                    "ok live_segments_checked={} cells_checked={} wal_records_checked={} wal_safe_truncate_offset={}",
                    validation.live_segments_checked,
                    validation.cells_checked,
                    validation.wal_records_checked,
                    validation.wal_safe_truncate_offset
                ))
            }
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
        "wal-truncate" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            wal::truncate(path)
        }
        "manifest-dump" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            manifest::dump(path)
        }
        "manifest-validate" => {
            if !rest.is_empty() {
                return Err(usage());
            }
            manifest::validate(path)
        }
        "context" => {
            let [scope, aql] = rest else {
                return Err(usage());
            };
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let pack = db
                .context_pack_from_aql(aql, &view_for_scope(scope), ContextPackOptions::default())
                .map_err(|error| error.to_string())?;
            if json_flag {
                Ok(context_pack_to_json(&pack))
            } else {
                Ok(format_context_pack(&pack))
            }
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
            if json_flag {
                Ok(verification_report_to_json(&report, &db))
            } else {
                Ok(format_verification_report(&report))
            }
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
        "search-vector" => {
            let [scope, vector] = rest else {
                return Err(usage());
            };
            let vector = parse_vector_literal(vector)?;
            let db = Database::open(path).map_err(|error| error.to_string())?;
            let results = db
                .search_vector(&vector, &view_for_scope(scope), SearchLimit(20))
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
        "ingest-text" => {
            let [scope, file] = rest else {
                return Err(usage());
            };
            let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let results = db.ingest_text_chunks(CellId(1), &content, cortex_engine::TextIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            }).map_err(|error| error.to_string())?;
            Ok(format!("ingested_chunks={} first_cell_id={}", results.len(), results[0].cell_id.0))
        }
        "ingest-json" => {
            let [scope, file] = rest else {
                return Err(usage());
            };
            let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let results = db.ingest_json(CellId(1), &content, cortex_engine::JsonIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            }).map_err(|error| error.to_string())?;
            Ok(format!("ingested_facts={} first_cell_id={}", results.len(), results[0].cell_id.0))
        }
        "ingest-csv" => {
            let [scope, file] = rest else {
                return Err(usage());
            };
            let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
            let mut db = Database::open(path).map_err(|error| error.to_string())?;
            let results = db.ingest_csv(CellId(1), &content, cortex_engine::CsvIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            }).map_err(|error| error.to_string())?;
            Ok(format!("ingested_rows={} first_cell_id={}", results.len(), results[0].cell_id.0))
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
    "usage: cortexdb put <path> <cell_id> <payload> | get <path> <cell_id> | tombstone <path> <cell_id> | flush <path> | compact <path> | stats <path> | validate <path> | repair <path> | gc-retired <path> | wal-validate <path> | wal-dump <path> | wal-truncate <path> | manifest-dump <path> | manifest-validate <path> | context <path> <scope> <aql> | remember <path> <scope> <aql> | verify <path> <scope> <aql> | aql <path> <scope> <aql> | search <path> <scope> <query> | search-vector <path> <scope> <vector> | unlock <path> --force"
        .to_owned()
}
