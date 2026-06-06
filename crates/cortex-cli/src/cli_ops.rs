use cortex_core::CellId;
use cortex_engine::{
    evaluate_hnsw_no_fallback_rollout, parse_vector_literal, route_search_query, AnnSearchPath,
    AnnSearchPolicy, ContextPackExportFormat, ContextPackOptions, Database, DatabaseSearchResult,
    EngineConfig, EngineError, HnswNoFallbackRolloutPolicy, SearchLimit, SearchMode, SearchQuery,
    SearchRouteInput, SearchRouteStrategy, VerificationReportExportFormat,
};

use crate::cli_json::{
    ann_validate_to_json, context_pack_to_json, no_fallback_profile_to_json, stats_to_json,
    validation_to_json, verification_report_to_json,
};
use crate::context::{
    format_context_pack, format_search_results, format_verification_report,
    remember_view_for_scope, verify_view_for_scope, view_for_scope,
};
use crate::{manifest, wal};

pub(crate) fn fmt_engine_error(e: EngineError) -> String {
    let message = e.to_string();
    match e.cli_hint() {
        Some(hint) => format!("{message}\n  -> {hint}"),
        None => message,
    }
}

pub(crate) fn open_database(path: &str, experimental_hnsw: bool) -> Result<Database, String> {
    let mut options = EngineConfig::from_env()
        .map_err(|error| error.to_string())?
        .database_options();
    options.feature_flags = options
        .feature_flags
        .with_experimental_hnsw(options.feature_flags.experimental_hnsw || experimental_hnsw);
    Database::open_with_options(path, options).map_err(fmt_engine_error)
}

pub(crate) fn validate_tenant_id(tenant: &str) -> bool {
    crate::cli_doctor::is_valid_tenant_arg(tenant)
}

pub fn doctor(path: &str, tenant: Option<&str>) -> Result<String, String> {
    crate::cli_doctor::doctor(path, tenant)
}

pub fn run_demo() -> Result<String, String> {
    let output = std::process::Command::new("./examples/demo/investment_projects/run.sh")
        .output()
        .map_err(|e| format!("Failed to run demo script: {e}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr))
}

pub fn put(path: &str, cell_id: &str, payload: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = open_database(path, false)?;
    let seq = db
        .put_cell(cell_id, payload.as_bytes().to_vec())
        .map_err(fmt_engine_error)?;
    Ok(format!("seq={}", seq.0))
}

pub fn get(path: &str, cell_id: &str, json: bool) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let db = open_database(path, false)?;
    match db.get_latest_cell(cell_id) {
        Some(payload) => {
            if json {
                Ok(crate::cli_json::cell_to_json(cell_id.0, 0, &payload))
            } else {
                Ok(String::from_utf8_lossy(&payload).into_owned())
            }
        }
        None => Ok("null".to_owned()),
    }
}

pub fn tombstone(path: &str, cell_id: &str) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = open_database(path, false)?;
    let seq = db.tombstone_cell(cell_id).map_err(fmt_engine_error)?;
    Ok(format!("seq={}", seq.0))
}

pub fn flush(path: &str, experimental_hnsw: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let stats = db.checkpoint().map_err(fmt_engine_error)?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn compact(path: &str, experimental_hnsw: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let stats = db.compact().map_err(fmt_engine_error)?;
    Ok(format!(
        "checkpoint_seq={} cells_flushed={}",
        stats.checkpoint_seq.0, stats.cells_flushed
    ))
}

pub fn stats(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let stats = db.storage_stats().map_err(fmt_engine_error)?;
    if json {
        return Ok(stats_to_json(&stats));
    }
    Ok(format!(
        "current_seq={} checkpoint_seq={} live_segments={} retired_segments={} memtable_cells={} memtable_versions={} memtable_payload_bytes={} estimated_memtable_bytes={} estimated_index_bytes={} estimated_context_pack_bytes={} estimated_total_memory_bytes={} wal_size_bytes={} wal_writer_records={} wal_writer_bytes={} wal_writer_fsyncs={} wal_writer_batches={}",
        stats.current_seq.0,
        stats.checkpoint_seq.0,
        stats.live_segments,
        stats.retired_segments,
        stats.memtable.cell_count,
        stats.memtable.version_count,
        stats.memtable_payload_bytes,
        stats.estimated_memtable_bytes,
        stats.estimated_index_bytes,
        stats.estimated_context_pack_bytes,
        stats.estimated_total_memory_bytes,
        stats.wal_size_bytes,
        stats.wal_writer.records_written,
        stats.wal_writer.bytes_written,
        stats.wal_writer.fsync_count,
        stats.wal_writer.batches_committed
    ))
}

pub fn validate(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let validation = db.validate_storage().map_err(fmt_engine_error)?;
    if json {
        return Ok(validation_to_json(
            validation.live_segments_checked,
            validation.cells_checked,
            validation
                .wal_records_checked
                .try_into()
                .unwrap_or(u64::MAX),
            validation.wal_safe_truncate_offset,
            true,
        ));
    }
    Ok(format!(
        "ok live_segments_checked={} cells_checked={} wal_records_checked={} wal_safe_truncate_offset={}",
        validation.live_segments_checked,
        validation.cells_checked,
        validation.wal_records_checked,
        validation.wal_safe_truncate_offset
    ))
}

pub fn ann_validate(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let report = db.validate_storage_report();
    let ann_errors: Vec<String> = report
        .errors
        .iter()
        .filter(|e| e.contains("vector index") || e.contains("hnsw") || e.contains("HNSW"))
        .cloned()
        .collect();
    let ok = ann_errors.is_empty();
    if json {
        return Ok(ann_validate_to_json(
            report.vector_indexes_checked,
            report.hnsw_graphs_checked,
            ann_errors,
        ));
    }
    if ok {
        Ok(format!(
            "ok vector_indexes_checked={} hnsw_graphs_checked={}",
            report.vector_indexes_checked, report.hnsw_graphs_checked
        ))
    } else {
        Err(format!(
            "ANN/HNSW validation failed: {}",
            ann_errors.join("; ")
        ))
    }
}

pub fn vector_rebuild(path: &str, experimental_hnsw: bool, json: bool) -> Result<String, String> {
    let mut db = open_database(path, experimental_hnsw)?;
    let report = db
        .rebuild_vector_indexes(experimental_hnsw)
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(crate::cli_json::vector_rebuild_to_json(&report));
    }
    Ok(format!(
        "vector_rebuild segments_checked={} cells_scanned={} vector_candidates={} vector_indexes_rebuilt={} hnsw_graphs_rebuilt={} hnsw_enabled={}",
        report.segments_checked,
        report.cells_scanned,
        report.vector_candidates,
        report.vector_indexes_rebuilt,
        report.hnsw_graphs_rebuilt,
        report.hnsw_enabled
    ))
}

pub fn repair(path: &str, dry_run: bool) -> Result<String, String> {
    let report = if dry_run {
        Database::repair_best_effort_dry_run(path)
    } else {
        Database::repair_best_effort(path)
    }
    .map_err(fmt_engine_error)?;
    Ok(format!(
        "dry_run={} orphan_temp_files_removed={} wal_records_preserved={} wal_safe_truncate_offset={} wal_bytes_before={} wal_bytes_after={} wal_truncated={} wal_truncation_needed={}",
        report.dry_run,
        report.orphan_temp_files_removed,
        report.wal_records_preserved,
        report.wal_safe_truncate_offset,
        report.wal_bytes_before,
        report.wal_bytes_after,
        report.wal_truncated,
        report.wal_truncation_needed
    ))
}

pub fn backup(path: &str, backup_path: &str) -> Result<String, String> {
    let report = Database::backup_path(path, backup_path).map_err(fmt_engine_error)?;
    Ok(format!(
        "files_copied={} bytes_copied={} source_live_segments_checked={} source_cells_checked={} source_wal_records_checked={}",
        report.files_copied,
        report.bytes_copied,
        report.source_validation.live_segments_checked,
        report.source_validation.cells_checked,
        report.source_validation.wal_records_checked
    ))
}

pub fn backup_encrypted(
    path: &str,
    archive_path: &str,
    passphrase: &str,
) -> Result<String, String> {
    let report = Database::encrypted_backup_path(path, archive_path, passphrase)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "files_archived={} plaintext_bytes={} ciphertext_bytes={} source_live_segments_checked={} source_cells_checked={} source_wal_records_checked={}",
        report.files_archived,
        report.plaintext_bytes,
        report.ciphertext_bytes,
        report.source_validation.live_segments_checked,
        report.source_validation.cells_checked,
        report.source_validation.wal_records_checked
    ))
}

pub fn restore(backup_path: &str, path: &str, dry_run: bool) -> Result<String, String> {
    if dry_run {
        let report =
            Database::restore_from_backup_dry_run(backup_path, path).map_err(fmt_engine_error)?;
        return Ok(format!(
            "dry_run=true restore_path={} files_checked={} bytes_checked={} version_compatible={} backup_live_segments_checked={} backup_cells_checked={} backup_wal_records_checked={}",
            report.restore_path.display(),
            report.files_checked,
            report.bytes_checked,
            report.version_compatible,
            report.backup_validation.live_segments_checked,
            report.backup_validation.cells_checked,
            report.backup_validation.wal_records_checked
        ));
    }
    let report = Database::restore_from_backup(backup_path, path).map_err(fmt_engine_error)?;
    Ok(format!(
        "files_copied={} bytes_copied={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.files_copied,
        report.bytes_copied,
        report.restored_validation.live_segments_checked,
        report.restored_validation.cells_checked,
        report.restored_validation.wal_records_checked
    ))
}

pub fn restore_encrypted(
    archive_path: &str,
    path: &str,
    passphrase: &str,
) -> Result<String, String> {
    let report = Database::restore_from_encrypted_backup(archive_path, path, passphrase)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "files_restored={} plaintext_bytes={} ciphertext_bytes={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.files_restored,
        report.plaintext_bytes,
        report.ciphertext_bytes,
        report.restored_validation.live_segments_checked,
        report.restored_validation.cells_checked,
        report.restored_validation.wal_records_checked
    ))
}

pub fn backup_drill(path: &str, backup_path: &str, restore_path: &str) -> Result<String, String> {
    let report = Database::backup_restore_drill_path(path, backup_path, restore_path)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "backup_files_copied={} backup_bytes_copied={} restored_files_copied={} restored_bytes_copied={} restored_live_segments_checked={} restored_cells_checked={} restored_wal_records_checked={}",
        report.backup.files_copied,
        report.backup.bytes_copied,
        report.restore.files_copied,
        report.restore.bytes_copied,
        report.restore.restored_validation.live_segments_checked,
        report.restore.restored_validation.cells_checked,
        report.restore.restored_validation.wal_records_checked
    ))
}

pub fn backup_prune(
    backup_root: &str,
    prefix: &str,
    keep_latest: usize,
    dry_run: bool,
) -> Result<String, String> {
    let report = if dry_run {
        Database::prune_backup_retention_dry_run(backup_root, prefix, keep_latest)
    } else {
        Database::prune_backup_retention(backup_root, prefix, keep_latest)
    }
    .map_err(fmt_engine_error)?;
    Ok(format!(
        "dry_run={} backups_seen={} backups_kept={} backups_removed={} bytes_removed={}",
        report.dry_run,
        report.backups_seen,
        report.backups_kept,
        report.backups_removed,
        report.bytes_removed
    ))
}

pub fn backup_offsite_stage(
    backup_path: &str,
    offsite_root: &str,
    backup_id: &str,
) -> Result<String, String> {
    let report = Database::stage_backup_offsite(backup_path, offsite_root, backup_id)
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "adapter={} target_path={} published={} files_copied={} bytes_copied={} drill_restored_files_copied={} drill_restored_cells_checked={} staged_live_segments_checked={} staged_cells_checked={} staged_wal_records_checked={}",
        report.adapter,
        report.target_path.display(),
        report.published,
        report.files_copied,
        report.bytes_copied,
        report.drill_restore.files_copied,
        report.drill_restore.restored_validation.cells_checked,
        report.staged_validation.live_segments_checked,
        report.staged_validation.cells_checked,
        report.staged_validation.wal_records_checked
    ))
}

pub fn gc_retired(path: &str) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    let report = db
        .garbage_collect_retired_segments()
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "retired_segments_removed={} files_removed={}",
        report.retired_segments_removed, report.files_removed
    ))
}

pub fn context(
    path: &str,
    scope: &str,
    aql: &str,
    json: bool,
    format: &str,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let pack = db
        .context_pack_from_aql(aql, &view_for_scope(scope), ContextPackOptions::default())
        .map_err(fmt_engine_error)?;
    match if json { "json" } else { format } {
        "json" => Ok(context_pack_to_json(&pack)),
        "summary" => Ok(format_context_pack(&pack)),
        "prompt" => Ok(pack.export(ContextPackExportFormat::Prompt)),
        "markdown" => Ok(pack.export(ContextPackExportFormat::Markdown)),
        value => Err(format!(
            "unsupported context format '{value}' (expected summary, json, prompt, or markdown)"
        )),
    }
}

pub fn forget(path: &str, cell_id: &str, json: bool) -> Result<String, String> {
    let cell_id = parse_cell_id(cell_id)?;
    let mut db = open_database(path, false)?;
    db.forget_cell(cell_id).map_err(fmt_engine_error)?;
    if json {
        Ok(crate::cli_json::cell_to_json(cell_id.0, 0, b""))
    } else {
        Ok(format!("cell_id={} forgotten (tombstoned)", cell_id.0))
    }
}

pub fn remember(path: &str, scope: &str, aql: &str, json: bool) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    let result = db
        .remember_aql(aql, &remember_view_for_scope(scope))
        .map_err(fmt_engine_error)?;
    if json {
        Ok(crate::cli_json::remember_to_json(&result))
    } else {
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
}

pub fn verify(
    path: &str,
    scope: &str,
    aql: &str,
    json: bool,
    format: &str,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let report = db
        .verify_fact_aql(aql, &verify_view_for_scope(scope))
        .map_err(fmt_engine_error)?;
    match if json { "json" } else { format } {
        "summary" => Ok(format_verification_report(&report)),
        "json" => Ok(verification_report_to_json(&report, &db)),
        "markdown" => Ok(report.export(VerificationReportExportFormat::Markdown)),
        "audit" => Ok(report.export(VerificationReportExportFormat::Audit)),
        other => Err(format!(
            "unsupported verify format '{other}' (expected summary, json, markdown, or audit)"
        )),
    }
}

pub fn search(
    path: &str,
    scope: &str,
    query: &str,
    json: bool,
    mode: &str,
    vector: Option<&str>,
    algorithm: &str,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let route = route_search_query(SearchRouteInput {
        requested_mode: mode,
        algorithm,
        text_available: !query.trim().is_empty(),
        vector_available: vector
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false),
    })?;
    let view = view_for_scope(scope);
    let results = match route.selected_strategy {
        SearchRouteStrategy::Keyword => db.search_keyword(query, &view, SearchLimit(20)),
        SearchRouteStrategy::VectorExact => {
            let vector = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector_exact(&vector, &view, SearchLimit(20))
        }
        SearchRouteStrategy::VectorAnn => {
            let vector = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector(&vector, &view, SearchLimit(20))
        }
        SearchRouteStrategy::Hybrid => {
            let vector = vector.ok_or_else(|| "mode=hybrid requires --vector".to_owned())?;
            let vector = parse_vector_literal(vector)?;
            db.search_cells(
                SearchQuery {
                    text: query,
                    vector: Some(&vector),
                    limit: 20,
                    mode: SearchMode::Hybrid,
                },
                &view,
            )
        }
    }
    .map_err(fmt_engine_error)?;
    if json {
        Ok(crate::cli_json::search_to_json(
            &results,
            route.search_mode(),
            Some(&route),
        ))
    } else {
        Ok(format!(
            "routing requested_mode={} selected_strategy={} reason={}\n{}",
            route.requested_mode,
            route.selected_strategy.as_str(),
            route.reason,
            format_search_results(&results)
        ))
    }
}

pub fn search_explain(
    path: &str,
    scope: &str,
    query: &str,
    mode: &str,
    vector: Option<&str>,
) -> Result<String, String> {
    let db = open_database(path, false)?;
    let diagnostics = db.search_diagnostics(query).map_err(fmt_engine_error)?;
    let results = match mode {
        "keyword" => db.search_keyword(query, &view_for_scope(scope), SearchLimit(20)),
        "vector" => {
            let v = parse_vector_literal(vector.unwrap_or(query))?;
            db.search_vector(&v, &view_for_scope(scope), SearchLimit(20))
        }
        "hybrid" => {
            let vector = vector.ok_or_else(|| "mode=hybrid requires --vector".to_owned())?;
            let vector = parse_vector_literal(vector)?;
            db.search_cells(
                SearchQuery {
                    text: query,
                    vector: Some(&vector),
                    limit: 20,
                    mode: SearchMode::Hybrid,
                },
                &view_for_scope(scope),
            )
        }
        _ => return Err("mode must be keyword, vector, or hybrid".to_owned()),
    }
    .map_err(fmt_engine_error)?;
    let mut lines = vec![diagnostics];
    for (rank, result) in results.iter().enumerate() {
        lines.push(format_search_explain_line(rank + 1, result));
    }
    Ok(lines.join("\n"))
}

fn format_search_explain_line(rank: usize, result: &DatabaseSearchResult) -> String {
    let total = result.lexical_score.saturating_add(result.vector_score);
    let lexical_q16 = result
        .lexical_score
        .saturating_mul(65_535)
        .checked_div(total)
        .unwrap_or(0);
    let vector_q16 = result
        .vector_score
        .saturating_mul(65_535)
        .checked_div(total)
        .unwrap_or(0);
    let preview = String::from_utf8_lossy(&result.payload)
        .chars()
        .take(80)
        .collect::<String>();
    format!(
        "rank={} cell_id={} score={} lexical={} vector={} lexical_q16={} vector_q16={} fusion={} preview={}",
        rank,
        result.cell_id.0,
        result.score,
        result.lexical_score,
        result.vector_score,
        lexical_q16,
        vector_q16,
        result.lexical_score > 0 && result.vector_score > 0,
        preview
    )
}

pub struct SearchVectorOptions<'a> {
    pub path: &'a str,
    pub scope: &'a str,
    pub vector: &'a str,
    pub exact: bool,
    pub policy: Option<AnnSearchPolicy>,
    pub rollout_policy: Option<HnswNoFallbackRolloutPolicy>,
    pub use_no_fallback_profile: bool,
    pub experimental_hnsw: bool,
}

pub fn search_vector(options: SearchVectorOptions<'_>) -> Result<String, String> {
    let vector = parse_vector_literal(options.vector)?;
    let db = open_database(options.path, options.experimental_hnsw)?;
    let rollout_policy =
        resolve_no_fallback_profile(&db, options.rollout_policy, options.use_no_fallback_profile)?;
    let view = view_for_scope(options.scope);
    if options.exact {
        let results = db
            .search_vector_exact(&vector, &view, SearchLimit(20))
            .map_err(fmt_engine_error)?;
        Ok(format_search_results(&results))
    } else {
        let search_policy = options.policy.unwrap_or_default();
        let outcome = db
            .search_vector_with_report_with_policy(&vector, &view, SearchLimit(20), search_policy)
            .map_err(fmt_engine_error)?;
        let mut lines = Vec::new();
        lines.push(format_search_results(&outcome.results));
        if let Some(report) = outcome.ann_report {
            if let Some(rollout_policy) = rollout_policy {
                lines.push(crate::cli_ann::format_no_fallback_decision(
                    &evaluate_hnsw_no_fallback_rollout(rollout_policy, search_policy, &report),
                ));
            }
            lines.push(format_ann_search_report(&report));
        }
        Ok(lines.join("\n"))
    }
}

pub fn hnsw_no_fallback_profile_show(path: &str, json: bool) -> Result<String, String> {
    let db = open_database(path, false)?;
    let policy = db.hnsw_no_fallback_rollout_policy();
    if json {
        return Ok(no_fallback_profile_to_json(policy));
    }
    Ok(format_no_fallback_profile(policy))
}

pub fn hnsw_no_fallback_profile_set(
    path: &str,
    policy: HnswNoFallbackRolloutPolicy,
    json: bool,
) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    db.set_hnsw_no_fallback_rollout_policy(policy)
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(no_fallback_profile_to_json(Some(policy)));
    }
    Ok(format!(
        "hnsw_no_fallback_profile set\n{}",
        format_no_fallback_profile(Some(policy))
    ))
}

pub fn hnsw_no_fallback_profile_clear(path: &str, json: bool) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    db.clear_hnsw_no_fallback_rollout_policy()
        .map_err(fmt_engine_error)?;
    if json {
        return Ok(no_fallback_profile_to_json(None));
    }
    Ok("hnsw_no_fallback_profile cleared".to_owned())
}

pub(crate) fn resolve_no_fallback_profile(
    db: &Database,
    explicit: Option<HnswNoFallbackRolloutPolicy>,
    use_profile: bool,
) -> Result<Option<HnswNoFallbackRolloutPolicy>, String> {
    if explicit.is_some() && use_profile {
        return Err("use either --no-fallback-rollout or --use-no-fallback-profile".to_owned());
    }
    if !use_profile {
        return Ok(explicit);
    }
    db.hnsw_no_fallback_rollout_policy()
        .map(Some)
        .ok_or_else(|| "no persisted HNSW no-fallback profile is configured".to_owned())
}

fn format_no_fallback_profile(policy: Option<HnswNoFallbackRolloutPolicy>) -> String {
    match policy {
        Some(policy) => format!(
            "hnsw_no_fallback_profile configured=true rollout_enabled={} min_recall_q16={} require_upper_layers={}",
            policy.rollout_enabled, policy.min_recall_q16, policy.require_upper_layers
        ),
        None => "hnsw_no_fallback_profile configured=false".to_owned(),
    }
}

fn format_ann_search_report(report: &cortex_engine::AnnSearchReport) -> String {
    let fallback_reason = report
        .fallback_reason
        .map(|reason| reason.as_str())
        .unwrap_or("none");
    let returned = match report.path {
        AnnSearchPath::HnswGraph => "hnsw_graph",
        AnnSearchPath::ExactFallback => "exact_fallback",
    };
    let recall = report
        .recall_q16
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let min_recall = report
        .min_recall_q16
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_owned());
    let visited = report.visited_candidates;
    let max_visited = report
        .max_visited_candidates
        .map_or_else(|| "none".to_owned(), |value| value.to_string());
    let violations = if report.slo_violations.is_empty() {
        "none".to_owned()
    } else {
        report
            .slo_violations
            .iter()
            .map(|violation| violation.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };

    format!(
        "ann_path={returned} fallback_reason={fallback_reason} fallback_performed={} recall_q16={recall} min_recall_q16={min_recall} allowed_candidates={} visited_candidates={visited} max_visited_candidates={max_visited} hnsw_ef_construction={} require_slo={} production_safe={} slo_violations={violations}",
        report.fallback_performed,
        report.allowed_candidates,
        report.hnsw_ef_construction,
        report.require_slo,
        report.production_safe
    )
}

pub fn unlock(path: &str, force: bool) -> Result<String, String> {
    if !force {
        return Err("unlock requires --force. Warning: this may corrupt data if another process is using the database.\n  → try: cortexdb unlock <path> --force".to_owned());
    }
    Database::break_stale_lock(path).map_err(fmt_engine_error)?;
    Ok("stale lock removed".to_owned())
}

pub fn wal_validate(path: &str) -> Result<String, String> {
    wal::validate(path)
}

pub fn wal_dump(path: &str) -> Result<String, String> {
    wal::dump(path)
}

pub fn wal_truncate(path: &str) -> Result<String, String> {
    wal::truncate(path)
}

pub fn manifest_dump(path: &str) -> Result<String, String> {
    manifest::dump(path)
}

pub fn manifest_validate(path: &str) -> Result<String, String> {
    manifest::validate(path)
}

fn parse_cell_id(value: &str) -> Result<CellId, String> {
    value.parse::<u64>().map(CellId).map_err(|_| {
        format!(
            "cell_id must be a positive integer, got: {value:?}\n  → example: cortexdb get ./db 42"
        )
    })
}
