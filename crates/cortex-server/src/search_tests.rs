use super::handle_http;

#[test]
fn v1_ann_evaluate_reports_recall_for_checkpointed_vectors() {
    let dir = tempfile::tempdir().unwrap();
    put_vector(dir.path(), 1, "project:investments", "10,0");
    put_vector(dir.path(), 2, "project:investments", "0,10");
    put_vector(dir.path(), 3, "tenant:private", "0,11");
    assert!(handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n")
        .contains(r#""checkpoint_seq":3"#));

    let request = "POST /v1/search/ann-evaluate?scope=project:investments&vector=0,10&limit=2 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""available":true"#));
    assert!(response.contains(r#""recall_q16":65535"#));
    assert!(response.contains(r#""min_recall_q16":49151"#));
    assert!(response.contains(r#""exact_top_k":[2,1]"#));
    assert!(response.contains(r#""ann_top_k":[2,1]"#));
}

#[test]
fn v1_ann_evaluate_reports_unavailable_before_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    put_vector(dir.path(), 1, "project:investments", "10,0");

    let request =
        "POST /v1/search/ann-evaluate?scope=project:investments&vector=10,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""available":false"#));
    assert!(response.contains("requires_persisted_checkpoint_without_wal_tail"));
}

fn put_vector(path: &std::path::Path, cell_id: u64, scope: &str, vector: &str) {
    let request = format!(
        "POST /v1/cell?cell_id={cell_id} HTTP/1.1\r\n\r\nscope={scope}\nstatus=ready\nvector={vector}\n\nbody"
    );
    assert!(handle_http(path, &request).contains(&format!(r#""cell_id":{cell_id}"#)));
}
