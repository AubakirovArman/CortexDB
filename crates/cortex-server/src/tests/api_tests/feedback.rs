#[test]
fn v1_feedback_records_stats_and_context_ranking() {
    let dir = tempfile::tempdir().unwrap();
    let cell_one = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\n",
        "source=doc-one\nalpha budget baseline"
    );
    assert!(handle_http(dir.path(), cell_one).contains(r#""seq":1"#));
    let cell_two = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\n",
        "source=doc-two\nalpha budget preferred"
    );
    assert!(handle_http(dir.path(), cell_two).contains(r#""seq":2"#));

    let feedback = concat!(
        "POST /v1/feedback?source_cell_id=2&useful=true HTTP/1.1\r\n\r\n",
        "agent selected this context"
    );
    let feedback_response = handle_http(dir.path(), feedback);
    assert!(feedback_response.contains(r#""seq":3"#));
    assert!(feedback_response.contains(r#""source_cell_id":2"#));
    assert!(feedback_response.contains(r#""useful":true"#));

    let stats = handle_http(dir.path(), "GET /v1/feedback/stats HTTP/1.1\r\n\r\n");
    assert!(stats.contains(r#""total":1"#));
    assert!(stats.contains(r#""useful":1"#));
    assert!(stats.contains(r#""not_useful":0"#));
    assert!(stats.contains(r#""source_cell_id":2"#));
    assert!(stats.contains(r#""score":1"#));

    let context = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"alpha budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let context_response = handle_http(dir.path(), context);
    assert!(context_response.contains(r#""cells":[{"cell_id":2"#));
    assert!(context_response.contains(r#""name":"feedback_bonus""#));
    assert!(context_response.contains(r#""contribution":5000"#));
}

#[test]
fn v1_feedback_requires_existing_source_cell_and_boolean_useful() {
    let dir = tempfile::tempdir().unwrap();

    let missing = handle_http(
        dir.path(),
        "POST /v1/feedback?source_cell_id=99&useful=true HTTP/1.1\r\n\r\n",
    );
    assert!(missing.contains(r#""code":"not_found""#));

    let source = "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nscope=default\nstatus=ready\nsource";
    assert!(handle_http(dir.path(), source).contains(r#""seq":1"#));

    let invalid = handle_http(
        dir.path(),
        "POST /v1/feedback?source_cell_id=1&useful=maybe HTTP/1.1\r\n\r\n",
    );
    assert!(invalid.contains(r#""code":"bad_request""#));
    assert!(invalid.contains("useful must be boolean"));
}
