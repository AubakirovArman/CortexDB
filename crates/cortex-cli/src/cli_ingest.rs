use cortex_core::CellId;
use cortex_engine::{
    CsvIngestOptions, Database, IngestionProgress, JsonIngestOptions, TextIngestOptions,
};

pub fn load_fixture(path: &str, fixture_path: &str) -> Result<String, String> {
    let jsonl_file = std::path::Path::new(fixture_path).join("cells.jsonl");
    if !jsonl_file.exists() {
        return Err(format!("fixture file not found: {}", jsonl_file.display()));
    }
    let content = std::fs::read_to_string(&jsonl_file).map_err(|e| e.to_string())?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| format!("invalid json line: {e}"))?;
        let cell_id = parsed["cell_id"]
            .as_u64()
            .ok_or_else(|| "missing cell_id in json".to_owned())?;
        let payload = parsed["payload"]
            .as_str()
            .ok_or_else(|| "missing payload in json".to_owned())?;
        db.put_cell(CellId(cell_id), payload.as_bytes().to_vec())
            .map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(format!(
        "successfully loaded cells_count={} from {}",
        count,
        jsonl_file.display()
    ))
}

pub fn text(path: &str, scope: &str, file: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let results = db
        .ingest_text_chunks(
            CellId(1),
            &content,
            TextIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(first_cell_output(
        "ingested_chunks",
        results.len(),
        results.first().map(|cell| cell.cell_id.0),
    ))
}

pub fn json(path: &str, scope: &str, file: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let results = db
        .ingest_json(
            CellId(1),
            &content,
            JsonIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(first_cell_output(
        "ingested_facts",
        results.len(),
        results.first().map(|cell| cell.cell_id.0),
    ))
}

pub fn csv(path: &str, scope: &str, file: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut db = Database::open(path).map_err(|error| error.to_string())?;
    let results = db
        .ingest_csv(
            CellId(1),
            &content,
            CsvIngestOptions {
                scope: scope.to_owned(),
                source: file.to_owned(),
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(first_cell_output(
        "ingested_rows",
        results.len(),
        results.first().map(|cell| cell.cell_id.0),
    ))
}

pub fn jobs(path: &str, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let jobs = db
        .list_ingestion_jobs()
        .map_err(|error| error.to_string())?;
    if json {
        return serde_json::to_string(&jobs).map_err(|error| error.to_string());
    }
    let ids = jobs
        .iter()
        .map(|job| job.job_id.0.to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!("jobs={} ids={ids}", jobs.len()))
}

pub fn job(path: &str, job_id: u64, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let progress = db
        .load_ingestion_job(job_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("ingestion job not found: {job_id}"))?;
    format_job(progress, json)
}

pub fn cancel_job(path: &str, job_id: u64, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let progress = db
        .cancel_ingestion_job(job_id)
        .map_err(|error| error.to_string())?;
    format_job(progress, json)
}

pub fn retry_job(path: &str, job_id: u64, json: bool) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let progress = db
        .retry_ingestion_job(job_id)
        .map_err(|error| error.to_string())?;
    format_job(progress, json)
}

pub fn delete_job(path: &str, job_id: u64) -> Result<String, String> {
    let db = Database::open(path).map_err(|error| error.to_string())?;
    let deleted = db
        .delete_ingestion_job(job_id)
        .map_err(|error| error.to_string())?;
    Ok(format!("deleted={deleted}"))
}

fn first_cell_output(label: &str, count: usize, first_cell_id: Option<u64>) -> String {
    match first_cell_id {
        Some(id) => format!("{label}={count} first_cell_id={id}"),
        None => format!("{label}=0 first_cell_id=null"),
    }
}

fn format_job(progress: IngestionProgress, json: bool) -> Result<String, String> {
    if json {
        return serde_json::to_string(&progress).map_err(|error| error.to_string());
    }
    Ok(format!(
        "job_id={} status={:?} completed_items={} failed_items={} retry_count={} message={}",
        progress.job_id.0,
        progress.status,
        progress.completed_items,
        progress.failed_items,
        progress.retry_count,
        progress.message.unwrap_or_else(|| "null".to_owned())
    )
    .to_ascii_lowercase())
}
