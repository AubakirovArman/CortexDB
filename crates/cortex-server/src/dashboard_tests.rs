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
