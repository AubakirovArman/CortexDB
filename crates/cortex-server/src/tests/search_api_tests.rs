use crate::handle_http;

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
        "POST /v1/search?scope=project:investments&mode=vector&algorithm=ann&fallback=false&fallback_scan_cap=0&min_recall=1.0&vector=5,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""search_mode":"vector_ann""#));
    assert!(response.contains(r#""min_recall_q16":65535"#));
}
