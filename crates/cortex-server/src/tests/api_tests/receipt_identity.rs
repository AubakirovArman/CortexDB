fn context_receipt_db_instance_id(response: &str) -> String {
    let value = response_json(response);
    value["accountability_receipt"]["header"]["db_instance_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn configured_receipts_use_durable_database_instance_id_across_tenants() {
    let dir = tempfile::tempdir().unwrap();
    let options = receipt_options();
    let default_put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha default"
    );
    assert!(handle_http_with_options(dir.path(), default_put, &options).contains(r#""seq":1"#));

    let alpha_put = concat!(
        "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha tenant"
    );
    assert!(handle_http_with_options(dir.path(), alpha_put, &options).contains(r#""seq":1"#));

    let context = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"alpha\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let default_id =
        context_receipt_db_instance_id(&handle_http_with_options(dir.path(), context, &options));

    let alpha_context = concat!(
        "POST /v1/context?tenant=alpha&scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"alpha\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let alpha_id =
        context_receipt_db_instance_id(&handle_http_with_options(dir.path(), alpha_context, &options));

    assert_eq!(default_id, alpha_id);
    assert!(default_id.starts_with("dbi_"));
    assert!(!default_id.starts_with("local:"));

    let identity_file = dir.path().join("cortexdb.database_instance_identity.json");
    let identity = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(identity_file).unwrap(),
    )
    .unwrap();
    assert_eq!(
        identity["schema_version"],
        "cortexdb.database_instance_identity.v1"
    );
    assert_eq!(identity["db_instance_id"], default_id);
}
