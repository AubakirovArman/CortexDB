//! Final public endpoint snapshot coverage.

use crate::handle_http;

#[test]
fn snapshot_cluster_status_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/cluster/status HTTP/1.1\r\n\r\n");

    assert!(response.contains("200 OK"));
    assert!(response.contains(r#""local_node":"#));
    assert!(response.contains(r#""nodes":"#));
    assert!(response.contains(r#""replication_factor":"#));
    assert!(response.contains(r#""distributed_enabled":"#));
}

#[test]
fn snapshot_delete_cell_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=7 HTTP/1.1\r\n\r\nhello");

    let response = handle_http(dir.path(), "DELETE /v1/cell?cell_id=7 HTTP/1.1\r\n\r\n");

    assert!(response.contains("200 OK"));
    assert!(response.contains(r#""seq":"#));
    assert!(response.contains(r#""cell_id":7"#));
}

#[test]
fn snapshot_search_explain_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(
        dir.path(),
        concat!(
            "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
            "scope=project:test\nstatus=ready\nsource=doc-a\n\nsolar budget plan"
        ),
    );

    let response = handle_http(
        dir.path(),
        "POST /v1/search/explain?scope=project:test&q=budget&limit=5 HTTP/1.1\r\n\r\n",
    );

    assert!(response.contains("200 OK"));
    assert!(response.contains(r#""query_terms":"#));
    assert!(response.contains(r#""search_mode":"keyword""#));
    assert!(response.contains(r#""routing":"#));
    assert!(response.contains(r#""selected_strategy":"keyword""#));
    assert!(response.contains(r#""rank":"#));
    assert!(response.contains(r#""matched_terms":"#));
    assert!(response.contains(r#""matched_fields":"#));
    assert!(response.contains(r#""term_contributions":"#));
    assert!(response.contains(r#""contribution_summary":"#));
    assert!(response.contains(r#""payload_preview":"#));
}

#[test]
fn snapshot_ann_evaluate_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        "POST /v1/search/ann-evaluate?scope=default&vector=1,2,3&limit=10 HTTP/1.1\r\n\r\n",
    );

    assert!(response.contains("200 OK"));
    assert!(response.contains(r#""available":"#));
    assert!(response.contains(r#""ann_report":"#));
    assert!(response.contains(r#""exact_top_k":"#));
    assert!(response.contains(r#""ann_top_k":"#));
    assert!(response.contains(r#""recall_q16":"#));
}

#[test]
fn snapshot_ingest_job_lifecycle_response_shapes() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(
        dir.path(),
        "POST /v1/ingest/text?scope=default&source=snapshot HTTP/1.1\r\n\r\nhello world",
    );

    let list = handle_http(dir.path(), "GET /v1/ingest/jobs HTTP/1.1\r\n\r\n");
    assert!(list.contains("200 OK"));
    assert!(list.contains(r#""job_id":1"#));
    assert!(list.contains(r#""status":"completed""#));

    let get = handle_http(dir.path(), "GET /v1/ingest/jobs/1 HTTP/1.1\r\n\r\n");
    assert!(get.contains("200 OK"));
    assert!(get.contains(r#""label":"ingest_text""#));
    assert!(get.contains(r#""completed_items":"#));

    let delete = handle_http(dir.path(), "DELETE /v1/ingest/jobs/1 HTTP/1.1\r\n\r\n");
    assert!(delete.contains("200 OK"));
    assert!(delete.contains(r#""deleted":true"#));

    let retry_missing = handle_http(dir.path(), "POST /v1/ingest/jobs/1/retry HTTP/1.1\r\n\r\n");
    assert!(retry_missing.contains("400 Bad Request"));
    assert!(retry_missing.contains(r#""code":"bad_request""#));
}
