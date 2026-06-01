use crate::handle_http;
use tempfile::tempdir;

#[test]
fn v1_search_returns_scope_filtered_results() {
    let dir = tempfile::tempdir().unwrap();
    let put_a = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    let put_b = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=tenant:private\nstatus=ready\nhidden budget"
    );
    assert!(handle_http(dir.path(), put_a).contains(r#""seq":1"#));
    assert!(handle_http(dir.path(), put_b).contains(r#""seq":2"#));

    let request = "POST /v1/search?scope=project:investments&q=budget HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"keyword""#));
    assert!(response.contains(r#""cell_id":1"#));
    assert!(!response.contains(r#""cell_id":2"#));
}

#[test]
fn v1_vector_search_accepts_query_vector() {
    let dir = tempfile::tempdir().unwrap();
    let put_a = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha"
    );
    let put_b = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=tenant:private\nstatus=ready\nvector=9,0\nhidden"
    );
    assert!(handle_http(dir.path(), put_a).contains(r#""seq":1"#));
    assert!(handle_http(dir.path(), put_b).contains(r#""seq":2"#));

    let request = "POST /v1/search?scope=project:investments&mode=vector&algorithm=exact&vector=2,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"vector_exact""#));
    assert!(response.contains(r#""cell_id":1"#));
    assert!(response.contains(r#""vector_score":"#));
    assert!(!response.contains(r#""cell_id":2"#));
}

#[test]
fn v1_vector_search_can_request_ann_mode() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request =
        "POST /v1/search?scope=project:investments&mode=vector&algorithm=ann&vector=2,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"vector_ann""#));
    assert!(response.contains(r#""ann_report":{"path":"exact_fallback""#));
    assert!(response.contains(r#""fallback_reason":"no_persisted_segments""#));
    assert!(response.contains(r#""fallback_performed":true"#));
    assert!(response.contains(r#""production_safe":false"#));
    assert!(response.contains(r#""cell_id":1"#));
}

#[test]
fn v1_cluster_status_returns_single_node_view() {
    let dir = tempfile::tempdir().unwrap();
    let request = "GET /v1/cluster/status HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""local_node":1"#));
    assert!(response.contains(r#""replication_factor":1"#));
    assert!(response.contains(r#""distributed_enabled":false"#));
}

#[test]
fn v1_search_ann_policy_is_applied_when_passing_query_params() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));
    let flush = "POST /v1/flush HTTP/1.1\r\n\r\n";
    assert!(handle_http(dir.path(), flush).contains(r#""checkpoint_seq":1"#));

    let request =
        "POST /v1/search?scope=project:investments&mode=vector&algorithm=ann&fallback=false&fallback_scan_cap=0&min_recall=1.0&require_slo=true&no_fallback_rollout=true&no_fallback_min_recall=1.0&vector=5,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"vector_ann""#));
    assert!(response.contains(r#""min_recall_q16":65535"#));
    assert!(response.contains(r#""require_slo":true"#));
    assert!(response.contains(r#""no_fallback_decision":{"allowed":true,"reasons":[]}"#));
}

#[test]
fn v1_hnsw_no_fallback_profile_persists_and_drives_ann_decision() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));
    assert!(handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n")
        .contains(r#""checkpoint_seq":1"#));

    let profile = concat!(
        "PUT /v1/admin/search/hnsw/no-fallback-profile HTTP/1.1\r\n\r\n",
        r#"{"rollout_enabled":true,"min_recall_q16":32767,"require_upper_layers":true}"#
    );
    let response = handle_http(dir.path(), profile);
    assert!(response.contains(r#""configured":true"#));
    assert!(response.contains(r#""min_recall_q16":32767"#));

    let get = "GET /v1/admin/search/hnsw/no-fallback-profile HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), get);
    assert!(response.contains(r#""configured":true"#));

    let request =
        "POST /v1/search?scope=project:investments&mode=vector&algorithm=ann&fallback=false&fallback_scan_cap=0&min_recall=50%25&require_slo=true&no_fallback_profile=active&vector=5,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""no_fallback_decision":{"allowed":true,"reasons":[]}"#));

    let delete = "DELETE /v1/admin/search/hnsw/no-fallback-profile HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), delete);
    assert!(response.contains(r#""configured":false"#));
}

#[test]
fn v1_search_ann_policy_decodes_encoded_recall_percent() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=1,2\nalpha"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));
    let flush = "POST /v1/flush HTTP/1.1\r\n\r\n";
    assert!(handle_http(dir.path(), flush).contains(r#""checkpoint_seq":1"#));

    let request =
        "POST /v1/search?scope=project%3Ainvestments&mode=vector&algorithm=ann&fallback=true&min_recall=50%25&require_slo=false&vector=1,2 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"vector_ann""#));
    assert!(response.contains(r#""require_slo":false"#));
    assert!(response.contains(r#""fallback_performed":false"#));
}

#[test]
fn v1_search_rejects_invalid_encoded_ann_param() {
    let dir = tempdir().unwrap();
    let request = "POST /v1/search?scope=project%3Ainvestments&mode=vector&algorithm=ann&min_recall=%ZZ&vector=1,2 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains("400 Bad Request"));
    assert!(response.contains("\"error\":\"bad_request\""));
}
