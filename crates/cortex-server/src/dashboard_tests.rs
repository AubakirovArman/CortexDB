fn dashboard_options() -> super::ServerOptions {
    super::ServerOptions {
        dashboard_enabled: true,
        ..super::ServerOptions::default()
    }
}

#[test]
fn dashboard_endpoint_returns_html() {
    let tmp = std::env::temp_dir().join(format!("cortex-dashboard-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let request = "GET /dashboard HTTP/1.1\r\n\r\n";
    let response = super::handle_http_with_options(&tmp, request, &dashboard_options());
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
fn dashboard_endpoint_is_disabled_by_default() {
    let tmp = std::env::temp_dir().join(format!(
        "cortex-dashboard-disabled-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let request = "GET /dashboard HTTP/1.1\r\n\r\n";
    let response = super::handle_http(&tmp, request);
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found"),
        "expected dashboard to be disabled by default, got: {response}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dashboard_route_pages_return_html() {
    let tmp = std::env::temp_dir().join(format!(
        "cortex-dashboard-route-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    for route in [
        "/dashboard/permissions",
        "/dashboard/search",
        "/dashboard/storage",
        "/dashboard/cluster",
    ] {
        let request = format!("GET {route} HTTP/1.1\r\n\r\n");
        let response = super::handle_http_with_options(&tmp, &request, &dashboard_options());
        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "expected 200 for {route}, got: {response}"
        );
        assert!(
            response.contains("Content-Type: text/html"),
            "expected html content type for {route}"
        );
        assert!(response.contains("CortexDB Console"));
    }

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
        "/dashboard/assets/v1/reporting_common.js",
        "/dashboard/assets/v1/reporting_retrieval.js",
        "/dashboard/assets/v1/reporting_operations.js",
        "/dashboard/assets/v1/reporting_slo.js",
        "/dashboard/assets/v1/reporting_ingest.js",
        "/dashboard/assets/v1/reporting_audit.js",
        "/dashboard/assets/v1/reporting.js",
        "/dashboard/assets/v1/app.js",
        "href=\"/dashboard/cells\"",
        "href=\"/dashboard/permissions\"",
        "href=\"/dashboard/search\"",
        "href=\"/dashboard/ann-eval\"",
        "href=\"/dashboard/context\"",
        "href=\"/dashboard/verify\"",
        "href=\"/dashboard/ingest\"",
        "id=\"tenant\"",
        "id=\"read-only-mode\"",
        "id=\"permission-report\"",
        "id=\"permissions-report\"",
        "id=\"status-report\"",
        "id=\"slo-report\"",
        "Health, stats, validation, backup/restore posture, and request error state",
        "Availability, latency, backup freshness, validation status, and error budget",
        "id=\"history\"",
        "id=\"ingest-job-form\"",
        "id=\"ingest-jobs-list-button\"",
        "id=\"error-report\"",
        "id=\"cell-report\"",
        "id=\"ingest-report\"",
        "id=\"cluster-report\"",
        "Request issue",
        "id=\"search-report\"",
        "id=\"aql-report\"",
        "id=\"verify-report\"",
        "id=\"ann-report\"",
        "id=\"context-report\"",
        "id=\"storage-report\"",
    ] {
        assert!(html.contains(marker), "missing dashboard marker: {marker}");
    }

    for marker in [
        "/v1/stats",
        "/v1/compatibility",
        "/v1/cell",
        "/v1/search/ann-evaluate",
        "/v1/ingest/jobs/",
        "/v1/ingest/${kind}",
        "dashboard_status.v1",
        "summarizeCompatibilityResult",
        "backup_posture",
        "backup_restore_view",
        "dashboard_backup_restore.v1",
        "last_request_error",
        "dashboard_slo.v1",
        "buildSloDashboard",
        "backup_freshness",
        "validation_status",
        "error_budget",
        "incident_timeline",
        "buildIncidentTimeline",
        "audit_event",
        "rate_limit_event",
        "storage_event",
        "backup_event",
        "make backup-restore-production-pack-check",
        "dashboard_permissions.v1",
        "selected_scopes",
        "server_token_policy",
        "anonymous_synthetic_view",
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
        "label class=\"inline-control\" for=\"read-only-mode\"",
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
    let common = super::dashboard::asset("/dashboard/assets/v1/reporting_common.js")
        .expect("dashboard reporting common asset");
    let retrieval = super::dashboard::asset("/dashboard/assets/v1/reporting_retrieval.js")
        .expect("dashboard reporting retrieval asset");
    let operations = super::dashboard::asset("/dashboard/assets/v1/reporting_operations.js")
        .expect("dashboard reporting operations asset");
    let slo = super::dashboard::asset("/dashboard/assets/v1/reporting_slo.js")
        .expect("dashboard reporting slo asset");
    let ingest = super::dashboard::asset("/dashboard/assets/v1/reporting_ingest.js")
        .expect("dashboard reporting ingest asset");
    let audit = super::dashboard::asset("/dashboard/assets/v1/reporting_audit.js")
        .expect("dashboard reporting audit asset");
    let reporting = super::dashboard::asset("/dashboard/assets/v1/reporting.js")
        .expect("dashboard reporting asset");

    assert_eq!(style.content_type, "text/css; charset=utf-8");
    assert_eq!(script.content_type, "application/javascript; charset=utf-8");
    assert_eq!(common.content_type, "application/javascript; charset=utf-8");
    assert_eq!(
        retrieval.content_type,
        "application/javascript; charset=utf-8"
    );
    assert_eq!(
        operations.content_type,
        "application/javascript; charset=utf-8"
    );
    assert_eq!(slo.content_type, "application/javascript; charset=utf-8");
    assert_eq!(ingest.content_type, "application/javascript; charset=utf-8");
    assert_eq!(audit.content_type, "application/javascript; charset=utf-8");
    assert_eq!(
        reporting.content_type,
        "application/javascript; charset=utf-8"
    );
    assert!(style.body.contains(".tab[aria-current=\"page\"]"));
    assert!(script.body.contains("addEventListener(\"submit\""));
    assert!(script.body.contains("pushState"));
    assert!(script.body.contains("popstate"));
    assert!(common.body.contains("dashboard-reports.v1"));
    assert!(common.body.contains("function card"));
    assert!(retrieval.body.contains("renderAqlReport"));
    assert!(retrieval.body.contains("renderContextPack"));
    assert!(retrieval.body.contains("Citation explorer"));
    assert!(retrieval.body.contains("Explain explorer"));
    assert!(retrieval.body.contains("Anomaly explorer"));
    assert!(retrieval.body.contains("score_components"));
    assert!(retrieval.body.contains("why_excluded"));
    assert!(retrieval.body.contains("renderSearchReport"));
    assert!(retrieval.body.contains("renderVerificationReport"));
    assert!(retrieval.body.contains("Mixed evidence"));
    assert!(retrieval.body.contains("Contradicting evidence"));
    assert!(retrieval.body.contains("Numeric conflict explorer"));
    assert!(retrieval.body.contains("Guard explorer"));
    assert!(retrieval.body.contains("numeric_conflicts"));
    assert!(operations.body.contains("renderAnnEvaluation"));
    assert!(operations.body.contains("renderCellReport"));
    assert!(operations.body.contains("renderClusterReport"));
    assert!(operations.body.contains("renderIngestReport"));
    assert!(operations.body.contains("renderOperationalStatus"));
    assert!(operations.body.contains("Version compatibility"));
    assert!(operations.body.contains("API / SDK / storage / migration"));
    assert!(operations.body.contains("Backup posture"));
    assert!(operations.body.contains("Last error"));
    assert!(operations.body.contains("renderIncidentEvent"));
    assert!(operations.body.contains("Incident timeline"));
    assert!(operations.body.contains("audit / rate / storage / backup"));
    assert!(operations.body.contains("renderPermissionsView"));
    assert!(operations.body.contains("Permissions explorer"));
    assert!(operations.body.contains("Token / role / scope / AgentView"));
    assert!(operations.body.contains("Scope probes"));
    assert!(operations.body.contains("AgentView policy"));
    assert!(operations.body.contains("renderRequestIssue"));
    assert!(operations.body.contains("clearRequestIssue"));
    assert!(operations.body.contains("Use an admin token"));
    assert!(operations.body.contains("AgentView can read"));
    assert!(operations.body.contains("renderStorageValidation"));
    assert!(slo.body.contains("renderSloDashboard"));
    assert!(slo.body.contains("Availability"));
    assert!(slo.body.contains("Latency"));
    assert!(slo.body.contains("Backup freshness"));
    assert!(slo.body.contains("Validation status"));
    assert!(slo.body.contains("Error budget"));
    assert!(ingest.body.contains("ingestionJobDashboard"));
    assert!(ingest.body.contains("Ingestion job records"));
    assert!(ingest.body.contains("failure reason"));
    assert!(ingest.body.contains("Ingestion chunks and SourceRefs"));
    assert!(audit.body.contains("renderAuditReadiness"));
    assert!(reporting.body.contains("facadeLoaded"));
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
        (
            "/dashboard/assets/v1/reporting_common.js",
            "Content-Type: application/javascript; charset=utf-8",
            "dashboard-reports.v1",
        ),
        (
            "/dashboard/assets/v1/reporting_retrieval.js",
            "Content-Type: application/javascript; charset=utf-8",
            "renderSearchReport",
        ),
        (
            "/dashboard/assets/v1/reporting_operations.js",
            "Content-Type: application/javascript; charset=utf-8",
            "renderCellReport",
        ),
        (
            "/dashboard/assets/v1/reporting_ingest.js",
            "Content-Type: application/javascript; charset=utf-8",
            "ingestionJobDashboard",
        ),
        (
            "/dashboard/assets/v1/reporting_audit.js",
            "Content-Type: application/javascript; charset=utf-8",
            "renderAuditReadiness",
        ),
        (
            "/dashboard/assets/v1/reporting.js",
            "Content-Type: application/javascript; charset=utf-8",
            "facadeLoaded",
        ),
    ] {
        let request = format!("GET {path} HTTP/1.1\r\n\r\n");
        let response = super::handle_http_with_options(&tmp, &request, &dashboard_options());
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
