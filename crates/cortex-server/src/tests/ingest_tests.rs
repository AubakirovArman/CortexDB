use crate::handle_http;

#[test]
fn empty_ingestion_endpoints_safety() {
    let dir = tempfile::tempdir().unwrap();

    let text_request = "POST /v1/ingest/text?scope=project:investments HTTP/1.1\r\n\r\n";
    let text_response = handle_http(dir.path(), text_request);
    assert!(text_response.contains(r#""chunks_ingested":0"#));
    assert!(text_response.contains(r#""first_cell_id":null"#));
    assert!(text_response.contains(r#""job_id":1"#));

    let json_request = "POST /v1/ingest/json?scope=project:investments HTTP/1.1\r\n\r\n{}";
    let json_response = handle_http(dir.path(), json_request);
    assert!(json_response.contains(r#""facts_ingested":0"#));
    assert!(json_response.contains(r#""first_cell_id":null"#));
    assert!(json_response.contains(r#""job_id":2"#));

    let csv_request = "POST /v1/ingest/csv?scope=project:investments HTTP/1.1\r\n\r\n";
    let csv_response = handle_http(dir.path(), csv_request);
    assert!(csv_response.contains(r#""rows_ingested":0"#));
    assert!(csv_response.contains(r#""first_cell_id":null"#));
    assert!(csv_response.contains(r#""job_id":3"#));
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

    let missing_request = "GET /v1/ingest/jobs/999 HTTP/1.1\r\n\r\n";
    let missing_response = handle_http(dir.path(), missing_request);
    assert!(missing_response.contains("job not found"));
}
