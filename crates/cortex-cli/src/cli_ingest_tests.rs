use super::run;
use cortex_engine::{Database, IngestionJobId, IngestionJobStatus, IngestionProgress};

#[test]
fn empty_ingestion_commands_return_null_first_cell_id() {
    let path = unique_path("cortexdb-cli-empty-ingest-db");
    let input_dir = unique_path("cortexdb-cli-empty-ingest-inputs");
    std::fs::create_dir_all(&input_dir).unwrap();
    let path_arg = path.to_string_lossy().into_owned();

    let text_file = input_dir.join("empty.txt");
    let json_file = input_dir.join("empty.json");
    let csv_file = input_dir.join("empty.csv");
    std::fs::write(&text_file, "").unwrap();
    std::fs::write(&json_file, "{}").unwrap();
    std::fs::write(&csv_file, "").unwrap();

    let text_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-text".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        text_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(text_output, "ingested_chunks=0 first_cell_id=null");

    let json_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-json".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        json_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(json_output, "ingested_facts=0 first_cell_id=null");

    let csv_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-csv".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        csv_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(csv_output, "ingested_rows=0 first_cell_id=null");

    let _ = std::fs::remove_dir_all(path);
    let _ = std::fs::remove_dir_all(input_dir);
}

#[test]
fn ingestion_job_commands_cover_list_get_retry_cancel_and_delete() {
    let path = unique_path("cortexdb-cli-ingest-jobs-db");
    let path_arg = path.to_string_lossy().into_owned();
    {
        let db = Database::open(&path).unwrap();
        db.save_ingestion_job(&job(7, IngestionJobStatus::Running, None))
            .unwrap();
        db.save_ingestion_job(&job(
            8,
            IngestionJobStatus::Failed,
            Some("temporary failure"),
        ))
        .unwrap();
    }

    let list_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-jobs".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    assert_eq!(list_output, "jobs=2 ids=7,8");

    let get_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-job".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
    ])
    .unwrap();
    assert!(get_output.contains("job_id=7 status=queued"));

    let retry_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-job-retry".to_owned(),
        path_arg.clone(),
        "8".to_owned(),
    ])
    .unwrap();
    assert!(retry_output.contains("job_id=8 status=queued"));
    assert!(retry_output.contains("retry_count=1"));

    let cancel_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-job-cancel".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
    ])
    .unwrap();
    assert!(cancel_output.contains("job_id=7 status=cancelled"));

    let delete_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-job-delete".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
    ])
    .unwrap();
    assert_eq!(delete_output, "deleted=true");

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn ingestion_job_json_command_returns_progress_shape() {
    let path = unique_path("cortexdb-cli-ingest-job-json-db");
    let path_arg = path.to_string_lossy().into_owned();
    {
        let db = Database::open(&path).unwrap();
        db.save_ingestion_job(&job(9, IngestionJobStatus::Failed, Some("parse failed")))
            .unwrap();
    }

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "ingest-job".to_owned(),
        path_arg,
        "9".to_owned(),
    ])
    .unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&output).unwrap();
    assert_eq!(value["job_id"], 9);
    assert_eq!(value["status"], "failed");
    assert_eq!(value["message"], "parse failed");

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

fn job(id: u64, status: IngestionJobStatus, message: Option<&str>) -> IngestionProgress {
    IngestionProgress {
        job_id: IngestionJobId(id),
        label: format!("cli-job-{id}"),
        status,
        total_items: Some(4),
        completed_items: 1,
        failed_items: u64::from(message.is_some()),
        last_cell_id: None,
        message: message.map(str::to_owned),
        retry_count: 0,
        max_retries: 3,
    }
}
