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
    let script = super::dashboard::asset("/dashboard/assets/v1/app.js")
        .expect("dashboard script asset")
        .body;

    for marker in [
        "CortexDB Console",
        "/dashboard/assets/v1/style.css",
        "/dashboard/assets/v1/app.js",
        "href=\"#/cells\"",
        "href=\"#/search\"",
        "href=\"#/ann-eval\"",
        "href=\"#/context\"",
        "href=\"#/verify\"",
        "href=\"#/ingest\"",
        "id=\"tenant\"",
        "id=\"history\"",
        "id=\"ingest-job-form\"",
    ] {
        assert!(html.contains(marker), "missing dashboard marker: {marker}");
    }

    for marker in [
        "/v1/stats",
        "/v1/cell",
        "/v1/search/ann-evaluate",
        "/v1/ingest/jobs/",
        "/v1/ingest/${kind}",
    ] {
        assert!(
            script.contains(marker),
            "missing dashboard script marker: {marker}"
        );
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
        "aria-label=\"Console pages\"",
        "aria-current=\"page\"",
        "aria-live=\"polite\"",
        "id=\"output\" tabindex=\"0\"",
    ] {
        assert!(
            html.contains(marker),
            "missing accessibility marker: {marker}"
        );
    }
}

#[test]
fn dashboard_static_assets_are_versioned_and_typed() {
    let style =
        super::dashboard::asset("/dashboard/assets/v1/style.css").expect("dashboard style asset");
    let script =
        super::dashboard::asset("/dashboard/assets/v1/app.js").expect("dashboard script asset");

    assert_eq!(style.content_type, "text/css; charset=utf-8");
    assert_eq!(script.content_type, "application/javascript; charset=utf-8");
    assert!(style.body.contains(".tab[aria-current=\"page\"]"));
    assert!(script.body.contains("addEventListener(\"submit\""));
    assert!(script.body.contains("hashchange"));
    assert!(super::dashboard::asset("/dashboard/assets/v2/app.js").is_none());
}

#[test]
fn dashboard_asset_endpoint_serves_css_and_js() {
    let tmp = std::env::temp_dir().join(format!(
        "cortex-dashboard-assets-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    for (path, content_type, marker) in [
        (
            "/dashboard/assets/v1/style.css",
            "Content-Type: text/css; charset=utf-8",
            ".panel.active",
        ),
        (
            "/dashboard/assets/v1/app.js",
            "Content-Type: application/javascript; charset=utf-8",
            "run(\"stats\"",
        ),
    ] {
        let request = format!("GET {path} HTTP/1.1\r\n\r\n");
        let response = super::handle_http(&tmp, &request);
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "expected 200 for {path}, got: {response}"
        );
        assert!(
            response.contains(content_type),
            "expected {content_type} for {path}"
        );
        assert!(response.contains(marker), "missing asset marker: {marker}");
    }

    let _ = std::fs::remove_dir_all(&tmp);
}
