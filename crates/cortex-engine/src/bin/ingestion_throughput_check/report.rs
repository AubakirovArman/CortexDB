use std::fs;
use std::path::Path;

use serde_json::Value;

pub fn write_report(path: &Path, report: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to serialize report: {error}"))?;
    fs::write(path, content + "\n")
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn write_markdown(path: &Path, report: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let throughput = &report["throughput"];
    let batching = &report["batching"];
    let resume = &report["resume"];
    let body = format!(
        "# Ingestion Throughput\n\n\
         - status: `{}`\n\
         - docs: `{}`\n\
         - end_to_end_docs_per_sec: `{}`\n\
         - ingest_docs_per_sec: `{}`\n\
         - embedding_docs_per_sec: `{}`\n\
         - ingestion_write_batches: `{}`\n\
         - embedding_batches: `{}`\n\
         - embedding_write_batches: `{}`\n\
         - resume_after_docs: `{}`\n\
         - first_run_final_debt_items: `{}`\n\
         - final_debt_items: `{}`\n",
        if report["ok"].as_bool() == Some(true) {
            "passed"
        } else {
            "failed"
        },
        report["docs"].as_u64().unwrap_or(0),
        throughput["end_to_end_docs_per_sec"]
            .as_f64()
            .unwrap_or(0.0),
        throughput["ingest_docs_per_sec"].as_f64().unwrap_or(0.0),
        throughput["embedding_docs_per_sec"].as_f64().unwrap_or(0.0),
        batching["ingestion_write_batches"].as_u64().unwrap_or(0),
        batching["embedding_batches"].as_u64().unwrap_or(0),
        batching["embedding_write_batches"].as_u64().unwrap_or(0),
        resume["resume_after_docs"].as_u64().unwrap_or(0),
        resume["first_run_final_debt_items"].as_u64().unwrap_or(0),
        resume["final_debt_items"].as_u64().unwrap_or(0),
    );
    fs::write(path, body).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
