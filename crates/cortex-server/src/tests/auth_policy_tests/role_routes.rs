use super::helpers::*;

#[test]
fn data_token_cannot_access_admin_routes() {
    let dir = tempfile::tempdir().unwrap();
    let options = admin_and_data_options();

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/stats HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(denied.contains("403 Forbidden"), "data token got: {denied}");
    assert!(
        denied.contains("forbidden"),
        "data token denial should use forbidden code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/stats HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "admin token should access stats: {allowed}"
    );
}

#[test]
fn data_token_cannot_access_dashboard() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        dashboard_enabled: true,
        ..admin_and_data_options()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /dashboard HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "dashboard should require admin role: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /dashboard HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "admin token should access dashboard: {allowed}"
    );
}

#[test]
fn data_token_can_access_data_routes_and_health() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("data-secret", AuthRole::Data)],
        ..Default::default()
    };

    let health = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        health.contains("200 OK"),
        "health should be public: {health}"
    );

    let search = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        search.contains("200 OK"),
        "data token should access data route: {search}"
    );
}
