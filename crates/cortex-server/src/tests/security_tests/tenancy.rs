use std::thread;

use crate::{handle_http_with_options, ServerOptions};

use super::helpers::request;

#[test]
fn tenant_realms_isolate_cell_data_over_http() {
    let dir = tempfile::tempdir().unwrap();

    let alpha_put = concat!(
        "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha-only-payload"
    );
    let beta_put = concat!(
        "POST /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nbeta-only-payload"
    );
    assert!(
        handle_http_with_options(dir.path(), alpha_put, &ServerOptions::default())
            .contains(r#""seq":1"#)
    );
    assert!(
        handle_http_with_options(dir.path(), beta_put, &ServerOptions::default())
            .contains(r#""seq":1"#)
    );

    let alpha_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(alpha_get.contains("alpha-only-payload"));
    assert!(!alpha_get.contains("beta-only-payload"));

    let beta_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(beta_get.contains("beta-only-payload"));
    assert!(!beta_get.contains("alpha-only-payload"));

    let default_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(default_get.contains(r#""cell":null"#));

    assert!(dir.path().join("realms").join("alpha").is_dir());
    assert!(dir.path().join("realms").join("beta").is_dir());
}

#[test]
fn parallel_tenant_realms_do_not_share_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let mut handles = Vec::new();

    for index in 0..8u64 {
        let root = root.clone();
        handles.push(thread::spawn(move || {
            let tenant = format!("tenant_{index}");
            let payload = format!("tenant-{index}-payload");
            let put = format!(
                "POST /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\nscope=project:investments\nstatus=ready\n{payload}"
            );
            let put_response = handle_http_with_options(&root, &put, &ServerOptions::default());
            assert!(
                put_response.contains(r#""seq":1"#),
                "put failed for {tenant}: {put_response}"
            );

            let get = format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n");
            let get_response = handle_http_with_options(&root, &get, &ServerOptions::default());
            assert!(
                get_response.contains(&payload),
                "get failed for {tenant}: {get_response}"
            );
            (tenant, payload)
        }));
    }

    let completed = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    for (tenant, payload) in completed {
        assert!(dir.path().join("realms").join(&tenant).is_dir());
        let response = handle_http_with_options(
            dir.path(),
            &format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n"),
            &ServerOptions::default(),
        );
        assert!(response.contains(&payload));
    }
}

#[test]
fn test_tenant_validation_unit_cases() {
    // Accepted tenants
    assert!(crate::validate_tenant_id("default"));
    assert!(crate::validate_tenant_id("tenant1"));
    assert!(crate::validate_tenant_id("tenant-1"));
    assert!(crate::validate_tenant_id("tenant_1"));
    assert!(crate::validate_tenant_id("project_1"));
    // tenant:1 and project:investments are now rejected because ':' is
    // disallowed for cross-platform safety (Windows reserves it in paths).
    assert!(!crate::validate_tenant_id("tenant:1"));
    assert!(!crate::validate_tenant_id("project:investments"));

    // Rejected — path traversal patterns
    assert!(!crate::validate_tenant_id("../../escape"));
    assert!(!crate::validate_tenant_id("..%2f..%2fescape"));
    assert!(!crate::validate_tenant_id("a/b"));
    assert!(!crate::validate_tenant_id("a%2Fb"));
    assert!(!crate::validate_tenant_id("."));
    assert!(!crate::validate_tenant_id(".."));
    assert!(!crate::validate_tenant_id("../x"));

    // Rejected — length and empty
    assert!(!crate::validate_tenant_id(""));
    assert!(!crate::validate_tenant_id(&"a".repeat(65)));

    // Rejected — special characters
    assert!(!crate::validate_tenant_id("tenant@home"));
    assert!(!crate::validate_tenant_id("tenant space"));
    assert!(!crate::validate_tenant_id("tenant\nline"));
    assert!(!crate::validate_tenant_id("tenant:alpha"));
}

#[test]
fn tenant_validation_generated_reject_cases_do_not_panic_or_create_realms() {
    let dir = tempfile::tempdir().unwrap();
    let generated = generated_invalid_tenants();

    for tenant in &generated {
        assert!(
            !crate::validate_tenant_id(tenant),
            "generated tenant should be invalid: {tenant:?}"
        );
        let request = format!(
            "GET /v1/health?tenant={} HTTP/1.1\r\n\r\n",
            encode_tenant_query_value(tenant)
        );
        let response = handle_http_with_options(dir.path(), &request, &ServerOptions::default());
        assert!(
            response.contains("invalid_tenant"),
            "invalid tenant should return invalid_tenant: {tenant:?} -> {response}"
        );
    }

    assert!(
        !dir.path().join("realms").exists(),
        "invalid tenants must not create tenant realm directories"
    );
}

#[test]
fn test_query_param_percent_decoding() {
    // Scope with colon
    assert_eq!(
        crate::router::query_param_decoded("scope=project%3Ainvestments", "scope").unwrap(),
        "project:investments"
    );
    // Search query with space
    assert_eq!(
        crate::router::query_param_decoded("q=Solar%20Plant", "q").unwrap(),
        "Solar Plant"
    );
    // Plus sign (common form encoding)
    assert_eq!(
        crate::router::query_param_decoded("q=Solar+Plant", "q").unwrap(),
        "Solar Plant"
    );
    // Unencoded value passes through
    assert_eq!(
        crate::router::query_param_decoded("scope=finance", "scope").unwrap(),
        "finance"
    );
}

#[test]
fn test_tenant_path_traversal_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    let bad_tenants = [
        "../../escape",
        "..%2f..%2fescape",
        "a/b",
        "a%2Fb",
        ".",
        "..",
        "../x",
    ];

    for tenant in &bad_tenants {
        let req = format!("GET /v1/health?tenant={} HTTP/1.1\r\n\r\n", tenant);
        let resp_str = request(local_addr, &req);
        assert!(
            resp_str.contains("400 Bad Request"),
            "tenant='{}' should be rejected with 400, got: {}",
            tenant,
            resp_str
        );
        assert!(
            resp_str.contains("invalid_tenant"),
            "tenant='{}' response should contain invalid_tenant, got: {}",
            tenant,
            resp_str
        );
    }
}

fn generated_invalid_tenants() -> Vec<String> {
    let fragments = [
        "..",
        ".",
        "alpha/beta",
        "alpha\\beta",
        "alpha%2Fbeta",
        "alpha%5Cbeta",
        "alpha:beta",
        "alpha beta",
        "alpha\nbeta",
        "alpha\tbeta",
        "%2e%2e",
        "tenant@home",
    ];
    fragments
        .into_iter()
        .flat_map(|fragment| {
            [
                fragment.to_owned(),
                format!("{fragment}_suffix"),
                format!("prefix_{fragment}"),
            ]
        })
        .collect()
}

fn encode_tenant_query_value(value: &str) -> String {
    value
        .replace('\\', "%5C")
        .replace(' ', "%20")
        .replace('\n', "%0A")
        .replace('\t', "%09")
}
