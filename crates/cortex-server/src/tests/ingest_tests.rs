use cortex_engine::{
    Database, DatabaseOptions, IngestionBackpressurePolicy, IngestionJobId, IngestionJobStatus,
    IngestionProgress,
};

use crate::{handle_http, handle_http_with_options, ServerOptions};

#[test]
fn empty_ingestion_endpoints_safety() {
    let dir = tempfile::tempdir().unwrap();

    let text_request = "POST /v1/ingest/text?scope=project:investments HTTP/1.1\r\n\r\n";
    let text_response = handle_http(dir.path(), text_request);
    assert!(text_response.contains(r#""chunks_ingested":0"#));
    assert!(text_response.contains(r#""first_cell_id":null"#));
    assert!(text_response.contains(r#""job_id":1"#));
    assert!(text_response.contains(r#""reason":"no_cells_emitted""#));
    assert!(text_response.contains(r#""input_ref":"ingest_text""#));

    let json_request = "POST /v1/ingest/json?scope=project:investments HTTP/1.1\r\n\r\n{}";
    let json_response = handle_http(dir.path(), json_request);
    assert!(json_response.contains(r#""facts_ingested":0"#));
    assert!(json_response.contains(r#""first_cell_id":null"#));
    assert!(json_response.contains(r#""job_id":2"#));
    assert!(json_response.contains(r#""input_ref":"ingest_json""#));

    let csv_request = "POST /v1/ingest/csv?scope=project:investments HTTP/1.1\r\n\r\n";
    let csv_response = handle_http(dir.path(), csv_request);
    assert!(csv_response.contains(r#""rows_ingested":0"#));
    assert!(csv_response.contains(r#""first_cell_id":null"#));
    assert!(csv_response.contains(r#""job_id":3"#));
    assert!(csv_response.contains(r#""input_ref":"ingest_csv""#));
}

#[test]
fn text_ingestion_rejects_non_boolean_embed_param() {
    // The opt-in `embed` flag is parsed fail-closed: a non-boolean value is a
    // 400 before any ingest work happens, so a typo can never silently skip
    // embedding.
    let dir = tempfile::tempdir().unwrap();
    let request =
        "POST /v1/ingest/text?scope=project:investments&embed=maybe HTTP/1.1\r\n\r\nalpha budget";

    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""code":"bad_request""#));
    assert!(response.contains("embed must be boolean"));
}

#[test]
fn text_ingestion_without_embed_writes_no_vector_header() {
    // Default path (no `embed` param) stays network-free and writes no vector.
    let dir = tempfile::tempdir().unwrap();
    let request =
        "POST /v1/ingest/text?scope=project:investments&source=memo.md HTTP/1.1\r\n\r\nalpha budget";
    assert!(handle_http(dir.path(), request).contains(r#""chunks_ingested":1"#));

    let cell = handle_http(dir.path(), "GET /v1/cell?cell_id=10001 HTTP/1.1\r\n\r\n");
    assert!(!cell.contains("vector="));
}

/// Live end-to-end check of the `embed=true` HTTP path against a real embedding
/// provider. Ignored by default; opt in by exporting `CORTEXDB_EMBEDDING_URL`
/// (and any auth/model vars) and running:
/// `cargo test -p cortex-server --lib text_ingestion_embed_true_writes_vector_header_live -- --ignored --nocapture`
#[test]
#[ignore = "requires a live CORTEXDB_EMBEDDING_URL endpoint"]
fn text_ingestion_embed_true_writes_vector_header_live() {
    if std::env::var("CORTEXDB_EMBEDDING_URL").is_err() {
        eprintln!("skipping: CORTEXDB_EMBEDDING_URL not set");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let request = "POST /v1/ingest/text?scope=project:investments&source=live.md&embed=true HTTP/1.1\r\n\r\nsolar plant capital budget approved for 2025";

    let response = handle_http(dir.path(), request);
    assert!(
        response.contains(r#""chunks_ingested":1"#),
        "unexpected ingest response: {response}"
    );

    let cell = handle_http(dir.path(), "GET /v1/cell?cell_id=10001 HTTP/1.1\r\n\r\n");
    assert!(
        cell.contains("vector="),
        "expected a vector= header on the embedded cell: {cell}"
    );
}

#[test]
fn text_ingestion_response_reports_source_refs() {
    let dir = tempfile::tempdir().unwrap();
    let request = "POST /v1/ingest/text?scope=project:investments&source=memo.md HTTP/1.1\r\n\r\nalpha budget";

    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""validation_report":"#));
    assert!(response.contains(r#""cells_seen":1"#));
    assert!(response.contains(r#""has_source_ref":true"#));
    assert!(response.contains(r#""source_id":"memo.md""#));
    assert!(response.contains(r#""chunk_id":"memo.md#chunk-0001""#));
}

#[test]
fn ingestion_jobs_list_and_get() {
    let dir = tempfile::tempdir().unwrap();

    let csv_request =
        "POST /v1/ingest/csv?scope=project:investments HTTP/1.1\r\n\r\nname,value\nalpha,1\nbeta,2";
    let csv_response = handle_http(dir.path(), csv_request);
    assert!(csv_response.contains(r#""job_id":1"#));

    let list_request = "GET /v1/ingest/jobs HTTP/1.1\r\n\r\n";
    let list_response = handle_http(dir.path(), list_request);
    assert!(list_response.contains(r#""job_id":1"#));
    assert!(list_response.contains("ingest_csv"));
    assert!(list_response.contains("completed"));

    let get_request = "GET /v1/ingest/jobs/1 HTTP/1.1\r\n\r\n";
    let get_response = handle_http(dir.path(), get_request);
    assert!(get_response.contains(r#""job_id":1"#));
    assert!(get_response.contains("ingest_csv"));
    assert!(get_response.contains(r#""completed_items":2"#));
    assert!(get_response.contains(r#""last_cell_id":10002"#));

    let missing_request = "GET /v1/ingest/jobs/999 HTTP/1.1\r\n\r\n";
    let missing_response = handle_http(dir.path(), missing_request);
    assert!(missing_response.contains("job not found"));
}

#[test]
fn ingestion_job_cancel_and_retry_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_ingestion_job(&job(7, IngestionJobStatus::Running, None, 0))
            .unwrap();
        db.save_ingestion_job(&job(
            8,
            IngestionJobStatus::Failed,
            Some("temporary parser failure"),
            0,
        ))
        .unwrap();
    }

    let cancel_request = "POST /v1/ingest/jobs/7/cancel HTTP/1.1\r\n\r\n";
    let cancel_response = handle_http(dir.path(), cancel_request);
    assert!(cancel_response.contains(r#""job_id":7"#));
    assert!(cancel_response.contains(r#""status":"cancelled""#));

    let retry_request = "POST /v1/ingest/jobs/8/retry HTTP/1.1\r\n\r\n";
    let retry_response = handle_http(dir.path(), retry_request);
    assert!(retry_response.contains(r#""job_id":8"#));
    assert!(retry_response.contains(r#""status":"queued""#));
    assert!(retry_response.contains(r#""retry_count":1"#));
    assert!(!retry_response.contains("temporary parser failure"));

    let delete_request = "DELETE /v1/ingest/jobs/7 HTTP/1.1\r\n\r\n";
    let delete_response = handle_http(dir.path(), delete_request);
    assert!(delete_response.contains(r#""deleted":true"#));
}

#[test]
fn ingestion_backpressure_payload_too_large_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        engine_database_options: DatabaseOptions {
            ingestion_backpressure: IngestionBackpressurePolicy {
                max_input_bytes: 4,
                ..IngestionBackpressurePolicy::default()
            },
            ..DatabaseOptions::default()
        },
        ..ServerOptions::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "POST /v1/ingest/text?scope=default HTTP/1.1\r\n\r\nabcde",
        &options,
    );

    assert!(response.contains("413 Payload Too Large"), "{response}");
    assert!(response.contains(r#""error":"payload_too_large""#));
}

#[test]
fn ingestion_backpressure_queue_limit_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_ingestion_job(&job(20, IngestionJobStatus::Queued, None, 0))
            .unwrap();
    }
    let options = ServerOptions {
        engine_database_options: DatabaseOptions {
            ingestion_backpressure: IngestionBackpressurePolicy {
                max_queued_jobs: 1,
                ..IngestionBackpressurePolicy::default()
            },
            ..DatabaseOptions::default()
        },
        ..ServerOptions::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "POST /v1/ingest/text?scope=default HTTP/1.1\r\n\r\nhello",
        &options,
    );

    assert!(response.contains("503 Service Unavailable"), "{response}");
    assert!(response.contains(r#""error":"database_busy""#));
    assert!(response.contains("queued job limit exceeded"));
}

#[test]
fn forget_endpoint_tombstones_cell() {
    let dir = tempfile::tempdir().unwrap();
    let put_request = "POST /v1/cell?cell_id=42 HTTP/1.1\r\n\r\nhello world";
    handle_http(dir.path(), put_request);

    let forget_request = "POST /v1/forget?cell_id=42 HTTP/1.1\r\n\r\n";
    let forget_response = handle_http(dir.path(), forget_request);
    assert!(forget_response.contains(r#""cell_id":42"#));

    let get_request = "GET /v1/cell?cell_id=42 HTTP/1.1\r\n\r\n";
    let get_response = handle_http(dir.path(), get_request);
    assert!(get_response.contains(r#""cell":null"#));
}

fn job(
    id: u64,
    status: IngestionJobStatus,
    message: Option<&str>,
    retry_count: u32,
) -> IngestionProgress {
    IngestionProgress {
        job_id: IngestionJobId(id),
        label: format!("test-job-{id}"),
        status,
        total_items: Some(10),
        completed_items: 2,
        failed_items: u64::from(message.is_some()),
        last_cell_id: None,
        message: message.map(str::to_owned),
        retry_count,
        max_retries: 3,
    }
}
