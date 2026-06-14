use cortex_engine::Database;

#[test]
fn v1_remember_and_verify_work() {
    let dir = tempfile::tempdir().unwrap();
    let remember = concat!(
        "POST /v1/remember?scope=project:investments HTTP/1.1\r\n\r\n",
        "REMEMBER \"ABC budget approved\" IN SCOPE project:investments AS TYPE decision ",
        "TTL 60 SECONDS;"
    );
    let remember_response = handle_http(dir.path(), remember);
    assert!(remember_response.contains(r#""seq":1"#));
    assert!(remember_response.contains(r#""ttl_seconds":60"#));

    let verify = concat!(
        "POST /v1/verify?scope=project:investments HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"ABC budget approved\" IN BRAIN investment_projects;"
    );
    let verify_response = handle_http(dir.path(), verify);
    assert!(verify_response.contains(r#""status":"supported""#));
    assert!(verify_response.contains(r#""matched_terms":"#));
}

#[test]
fn v1_remember_ttl_expiry_disappears_from_context() {
    let dir = tempfile::tempdir().unwrap();
    let remember = concat!(
        "POST /v1/remember?scope=project:investments HTTP/1.1\r\n\r\n",
        "REMEMBER \"Temporary budget preference\" IN SCOPE project:investments AS TYPE decision ",
        "TTL 1 SECONDS;"
    );
    let remember_response = handle_http(dir.path(), remember);
    assert!(remember_response.contains(r#""ttl_seconds":1"#));

    let context = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"temporary budget preference\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND type = \"memory\" AND memory_type = \"decision\" ",
        "LIMIT 10 CANDIDATES;"
    );
    let before_expiry = handle_http(dir.path(), context);
    assert!(before_expiry.contains("Temporary budget preference"));

    let mut db = Database::open(dir.path()).unwrap();
    let expired = db.expire_memory_cells(u64::MAX).unwrap();
    assert_eq!(expired.len(), 1);
    drop(db);

    let after_expiry = handle_http(dir.path(), context);
    assert!(after_expiry.contains(r#""cells":[]"#));
    assert!(!after_expiry.contains("Temporary budget preference"));
}
