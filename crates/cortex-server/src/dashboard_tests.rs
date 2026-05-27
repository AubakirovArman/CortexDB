#[test]
fn dashboard_endpoint_returns_html() {
    let tmp = std::env::temp_dir().join(format!("cortex-dashboard-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let request = "GET /dashboard HTTP/1.1\r\n\r\n";
    let response = super::handle_http(&tmp, request);
    assert!(
        response.starts_with("HTTP/1.1 200 OK"),
        "expected 200, got: {response}"
    );
    assert!(
        response.contains("Content-Type: text/html"),
        "expected html content type"
    );
    assert!(
        response.contains("CortexDB Console"),
        "expected dashboard title in body"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dashboard_html_exposes_admin_console_surfaces() {
    let html = super::dashboard::html();

    for marker in [
        "CortexDB Console",
        "data-tab=\"cells\"",
        "data-tab=\"search\"",
        "data-tab=\"ann-eval\"",
        "data-tab=\"context\"",
        "data-tab=\"verify\"",
        "data-tab=\"ingest\"",
        "id=\"tenant\"",
        "id=\"history\"",
        "id=\"ingest-job-form\"",
        "/v1/stats",
        "/v1/cell",
        "/v1/search/ann-evaluate",
        "/v1/ingest/jobs/",
        "/v1/ingest/${kind}",
    ] {
        assert!(html.contains(marker), "missing dashboard marker: {marker}");
    }
}

#[test]
fn dashboard_forms_have_accessible_labels_and_live_output() {
    let html = super::dashboard::html();

    for marker in [
        "label for=\"ann-vector\"",
        "label for=\"ann-limit\"",
        "label for=\"tenant\"",
        "label for=\"ingest-document\"",
        "label for=\"ingest-job-id\"",
        "label for=\"ingest-type\"",
        "aria-label=\"Request history\"",
        "role=\"tablist\"",
        "role=\"tabpanel\"",
        "aria-live=\"polite\"",
        "id=\"output\" tabindex=\"0\"",
    ] {
        assert!(
            html.contains(marker),
            "missing accessibility marker: {marker}"
        );
    }
}
