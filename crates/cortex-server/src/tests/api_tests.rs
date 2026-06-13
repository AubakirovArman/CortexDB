use crate::handle_http;

#[test]
fn v1_context_returns_context_pack() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\n",
        "source_id=doc-a\nsource_url=https://example.test/doc-a\n",
        "document_id=doc-1\nconfidence_q16=60000\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains(r#""citation":"doc-a""#));
    assert!(response.contains(r#""source_url":"https://example.test/doc-a""#));
}

#[test]
fn v1_context_returns_prompt_and_markdown_exports() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let prompt = concat!(
        "POST /v1/context?scope=project:investments&format=prompt HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let prompt_response = handle_http(dir.path(), prompt);
    assert!(prompt_response.contains("CortexDB ContextPack v1"));
    assert!(prompt_response.contains("Use only the context cells below."));

    let markdown = concat!(
        "POST /v1/context?scope=project:investments&format=markdown HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let markdown_response = handle_http(dir.path(), markdown);
    assert!(markdown_response.contains("# CortexDB ContextPack"));
    assert!(markdown_response.contains("### Cell 1"));
}

#[test]
fn v1_aql_returns_retrieved_cells() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains("alpha budget"));
}

#[test]
fn v1_aql_explain_returns_plan_filters_counts_and_mode() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments HTTP/1.1\r\n\r\n",
        "EXPLAIN RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "USING MODE balanced WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[]"#));
    assert!(response.contains(r#""selected_mode":"balanced""#));
    assert!(response.contains(r#""cost_model":{"selected_path":"bitmap-first""#));
    assert!(response.contains(r#""after_bitmap":1"#));
    assert!(response.contains(r#""kind":"where""#));
    assert!(response.contains("BitmapProgram(max_stack_depth="));
}

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
fn v1_verify_returns_markdown_and_audit_exports() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=report.pdf\nsource_trust_q16=60000\nmetric=budget\n\n",
        "Solar Plant budget changed to 1.4B KZT."
    );
    handle_http(dir.path(), put);

    let markdown = concat!(
        "POST /v1/verify?scope=project:investments&format=markdown HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
    let markdown_response = handle_http(dir.path(), markdown);
    assert!(markdown_response.contains("# CortexDB Verification Report"));
    assert!(markdown_response.contains("## Numeric Conflicts"));

    let audit = concat!(
        "POST /v1/verify?scope=project:investments&format=audit HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
    let audit_response = handle_http(dir.path(), audit);
    assert!(audit_response.contains("CortexDB Verification Audit v1"));
    assert!(audit_response.contains("numeric_conflict.0.metric=budget"));
}
