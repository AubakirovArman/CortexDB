use cortex_engine::{ContextPackExportFormat, ContextPackOptions, VerificationReportExportFormat};

use crate::cli_json::{context_pack_to_json, verification_report_to_json};
use crate::context::{
    format_context_pack, format_verification_report, remember_view_for_scope,
    verify_view_for_scope, view_for_scope,
};

use super::common::{fmt_engine_error, open_database, parse_cell_id};

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
