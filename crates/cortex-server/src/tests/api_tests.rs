use crate::handle_http;

#[test]
fn v1_context_returns_context_pack() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget"
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
