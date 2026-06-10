//! Snapshot/golden tests for additional API endpoints.

use crate::handle_http;

#[test]
fn snapshot_compact_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let response = handle_http(dir.path(), "POST /v1/compact HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""checkpoint_seq":"#));
    assert!(response.contains(r#""cells_flushed":"#));
}

#[test]
fn snapshot_verify_supported_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nThe budget is 1.2B KZT."
    );
    handle_http(dir.path(), put);

    let request = concat!(
        "POST /v1/verify?scope=project:test HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"The budget is 1.2B KZT\" IN BRAIN test;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""verdict":"supported""#));
    assert!(response.contains(r#""supporting":"#));
    assert!(response.contains(r#""contradicting":"#));
    assert!(response.contains(r#""numeric_conflicts":"#));
}

#[test]
fn snapshot_verify_mixed_with_numeric_conflict_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put1 = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\nmetric=budget\nvalue=1.2\ncurrency=KZT\n\nBudget is 1.2B KZT."
    );
    handle_http(dir.path(), put1);
    let put2 = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-b\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nBudget raised to 1.4B KZT."
    );
    handle_http(dir.path(), put2);

    let request = concat!(
        "POST /v1/verify?scope=project:test HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"The budget is 1.2B KZT\" IN BRAIN test;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""verdict":"mixed_evidence""#));
    assert!(response.contains(r#""numeric_conflicts":"#));
    assert!(response.contains(r#""left":"#));
    assert!(response.contains(r#""right":"#));
    assert!(response.contains(r#""metric":"#));
}

#[test]
fn snapshot_search_keyword_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nalpha budget proposal"
    );
    handle_http(dir.path(), put);

    let response = handle_http(
        dir.path(),
        "POST /v1/search?scope=project:test&q=budget HTTP/1.1\r\n\r\n",
    );
    assert!(response.contains(r#""search_mode":"keyword""#));
    assert!(response.contains(r#""results":"#));
}

#[test]
fn snapshot_remember_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let request = concat!(
        "POST /v1/remember?scope=project:test HTTP/1.1\r\n\r\n",
        "REMEMBER \"hello world\" IN SCOPE project:test AS TYPE decision;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""seq":"#));
    assert!(response.contains(r#""cell_id":"#));
    assert!(response.contains(r#""ttl_seconds":"#));
}

#[test]
fn snapshot_ingest_json_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        "POST /v1/ingest/json?scope=default HTTP/1.1\r\n\r\n[{\"text\":\"hello\"}]",
    );
    assert!(response.contains(r#""facts_ingested":"#));
    assert!(response.contains(r#""first_cell_id":"#));
    assert!(response.contains(r#""validation_report":"#));
}

#[test]
fn snapshot_ingest_csv_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        "POST /v1/ingest/csv?scope=default HTTP/1.1\r\n\r\nname,value\nalpha,1\n",
    );
    assert!(response.contains(r#""rows_ingested":"#));
    assert!(response.contains(r#""first_cell_id":"#));
    assert!(response.contains(r#""validation_report":"#));
}

#[test]
fn snapshot_ann_metrics_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let response = handle_http(dir.path(), "GET /v1/ann/metrics HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""graph_nodes":"#));
    assert!(response.contains(r#""total_edges":"#));
    assert!(response.contains(r#""persisted_segments":"#));
    assert!(response.contains(r#""has_checkpoint":"#));
    assert!(response.contains(r#""has_uncheckpointed_changes":"#));
    assert!(response.contains(r#""deleted_vectors":"#));
    assert!(response.contains(r#""rebuild_count":"#));
}

#[test]
fn snapshot_ingest_job_not_found_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/ingest/jobs/99 HTTP/1.1\r\n\r\n");
    assert!(response.contains("404"));
    assert!(response.contains(r#""error":"#));
    assert!(response.contains(r#""message":"#));
}

#[test]
fn snapshot_get_cell_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nhello world"
    );
    handle_http(dir.path(), put);
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let response = handle_http(dir.path(), "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""cell_id":"#));
    assert!(response.contains(r#""payload":"#));
}

#[test]
fn snapshot_flush_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    let response = handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""checkpoint_seq":"#));
    assert!(response.contains(r#""cells_flushed":"#));
}

#[test]
fn snapshot_context_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nalpha budget proposal"
    );
    handle_http(dir.path(), put);
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let request = concat!(
        "POST /v1/context?scope=project:test HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""schema_version":"context_pack.v1""#));
    assert!(response.contains(r#""token_budget_tokens":"#));
    assert!(response.contains(r#""cells":"#));
    assert!(response.contains(r#""estimated_tokens":"#));
}

#[test]
fn snapshot_context_trace_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nalpha budget proposal"
    );
    handle_http(dir.path(), put);
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let request = concat!(
        "POST /v1/context/trace?scope=project:test HTTP/1.1\r\n",
        "Content-Type: application/json\r\n\r\n",
        "{\"retrieve_aql\":\"RETRIEVE CONTEXT FOR TASK \\\"budget\\\" IN BRAIN default LIMIT 10 CANDIDATES;\",",
        "\"verify_aql\":\"VERIFY FACT \\\"alpha budget proposal\\\" IN BRAIN default;\"}"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""schema_version":"context_trace.v1""#));
    assert!(response.contains(r#""context":{"schema_version":"context_pack.v1""#));
    assert!(response.contains(r#""verification":{"fact":"alpha budget proposal""#));
    assert!(response.contains(r#""trace":{"schema_version":"context_pipeline_trace.v1""#));
    assert!(response.contains(r#""name":"retrieve""#));
    assert!(response.contains(r#""name":"pack""#));
    assert!(response.contains(r#""name":"verify""#));
    assert!(response.contains(r#""cells":[{"#));
}

#[test]
fn snapshot_aql_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nalpha budget proposal"
    );
    handle_http(dir.path(), put);
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let request = concat!(
        "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":"#));
}

#[test]
fn snapshot_aql_explain_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nalpha budget proposal"
    );
    handle_http(dir.path(), put);
    let request = concat!(
        "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
        "EXPLAIN RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default ",
        "WHERE space = project:test AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[]"#));
    assert!(response.contains(r#""explain":"#));
    assert!(response.contains(r#""selected_mode":"balanced""#));
    assert!(response.contains(r#""bitmap_plan":"#));
    assert!(response.contains(r#""candidate_counts":"#));
    assert!(response.contains(r#""filters":"#));
}

#[test]
fn snapshot_forget_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:test\nstatus=ready\nsource=doc-a\n\nhello world"
    );
    handle_http(dir.path(), put);
    let request = concat!("POST /v1/forget?cell_id=1 HTTP/1.1\r\n\r\n", "FORGET 1;");
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""seq":"#));
    assert!(response.contains(r#""cell_id":"#));
}

#[test]
fn snapshot_metrics_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    let response = handle_http(dir.path(), "GET /v1/metrics HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""current_seq":"#));
    assert!(response.contains(r#""checkpoint_seq":"#));
    assert!(response.contains(r#""live_segments":"#));
}

#[test]
fn snapshot_error_404_unknown_route_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/unknown HTTP/1.1\r\n\r\n");
    assert!(response.contains("404"));
    assert!(response.contains(r#""code":"not_found""#));
    assert!(response.contains(r#""error":"#));
}

#[test]
fn error_taxonomy_invalid_aql_has_stable_code() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        concat!(
            "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default USING MODE turbo;"
        ),
    );
    assert!(response.contains("400 Bad Request"));
    assert!(response.contains(r#""code":"invalid_aql""#));
    assert!(response.contains(r#""error":"invalid_aql""#));
}

#[test]
fn error_taxonomy_unknown_field_has_stable_code() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        concat!(
            "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default ",
            "WHERE unknown = \"ready\" LIMIT 10 CANDIDATES;"
        ),
    );
    assert!(response.contains("400 Bad Request"));
    assert!(response.contains(r#""code":"unknown_field""#));
    assert!(response.contains(r#""error":"unknown_field""#));
}

#[test]
fn error_taxonomy_unsupported_operator_has_stable_code() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        concat!(
            "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default ",
            "WHERE status != \"ready\" LIMIT 10 CANDIDATES;"
        ),
    );
    assert!(response.contains("400 Bad Request"));
    assert!(response.contains(r#""code":"unsupported_operator""#));
    assert!(response.contains(r#""error":"unsupported_operator""#));
}

#[test]
fn error_taxonomy_denied_scope_has_stable_code() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        concat!(
            "POST /v1/aql?scope=project:test HTTP/1.1\r\n\r\n",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default ",
            "WHERE space = tenant:private LIMIT 10 CANDIDATES;"
        ),
    );
    assert!(response.contains("403 Forbidden"));
    assert!(response.contains(r#""code":"permission_denied""#));
    assert!(response.contains(r#""error":"permission_denied""#));
}

#[test]
fn error_taxonomy_busy_and_corruption_codes_are_stable() {
    let busy = crate::responses::RouterError::DatabaseBusy("database actor busy".to_owned());
    assert_eq!(busy.status_code(), 503);
    assert_eq!(busy.code(), crate::responses::ErrorCode::DatabaseBusy);

    let corruption: crate::responses::RouterError =
        cortex_engine::EngineError::StorageInvariant("manifest mismatch".to_owned()).into();
    assert_eq!(corruption.status_code(), 500);
    assert_eq!(
        corruption.code(),
        crate::responses::ErrorCode::StorageCorruption
    );
}
