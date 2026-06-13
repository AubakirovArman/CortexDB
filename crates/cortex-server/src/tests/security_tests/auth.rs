use cortex_aql::AgentId;
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{Database, DatabaseOptions, PayloadResidency};

use crate::auth::AuthRouteContext;
use crate::responses::RouterError;
use crate::router::route_database_with_auth;
use crate::{handle_http_with_options, ServerOptions};

use super::helpers::agent_view;

#[test]
fn v1_api_requires_bearer_token_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        ..Default::default()
    };
    let denied = handle_http_with_options(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n", &options);
    assert!(denied.contains("401 Unauthorized"));

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(allowed.contains(r#""status":"ok""#));
}

#[test]
fn v1_api_rejects_wrong_bearer_token_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        ..Default::default()
    };
    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer wrong-secret\r\n\r\n",
        &options,
    );
    assert!(denied.contains("401 Unauthorized"));
    assert!(denied.contains("unauthorized"));
}

#[test]
fn auth_agent_view_blocks_unreadable_scope_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=secret&q=budget HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "unreadable scope should be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "readable scope should be allowed: {allowed}"
    );
}

#[test]
fn auth_agent_view_blocks_unwritable_cell_scope_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\nscope=secret\n\nhidden",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "unwritable payload scope should be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\nscope=finance\n\nvisible",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "writable payload scope should be allowed: {allowed}"
    );
}

#[test]
fn auth_agent_view_uses_descriptor_scope_for_cell_read_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "project:investments", true))
            .unwrap();
        db.put_knowledge_cell(
            CellId(9),
            KnowledgeCell::new(
                KnowledgeCellMetadata {
                    scope: "tenant:private".to_owned(),
                    status: "ready".to_owned(),
                    cell_type: KnowledgeCellType::Raw,
                    ..Default::default()
                },
                b"scope=project:investments\nstatus=ready\n\nhidden spoof".to_vec(),
            ),
        )
        .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?cell_id=9 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "descriptor scope should deny spoofed payload reads: {denied}"
    );
    assert!(denied.contains("permission_denied"));
}

#[test]
fn denied_cell_routes_authorize_descriptor_before_lazy_payload_read() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "project:investments", true))
            .unwrap();
        db.put_knowledge_cell(
            CellId(9),
            KnowledgeCell::new(
                KnowledgeCellMetadata {
                    scope: "tenant:private".to_owned(),
                    status: "ready".to_owned(),
                    cell_type: KnowledgeCellType::Raw,
                    ..Default::default()
                },
                b"scope=project:investments\nstatus=ready\n\nhidden spoof".to_vec(),
            ),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let mut db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    let auth = AuthRouteContext::for_agent(Some(7));

    for (method, target) in [
        ("GET", "/v1/cell?cell_id=9"),
        ("DELETE", "/v1/cell?cell_id=9"),
        ("POST", "/v1/forget?cell_id=9"),
    ] {
        let err = route_database_with_auth(&mut db, method, target, b"", auth.clone())
            .expect_err("descriptor scope should deny route before payload read");
        assert!(
            matches!(err, RouterError::PermissionDenied(_)),
            "expected permission denial for {method} {target}, got {err:?}"
        );
        assert_eq!(
            db.payload_cache_stats().segment_loads,
            0,
            "{method} {target} should not read segment payload before authz"
        );
    }
}

#[test]
fn auth_agent_id_requires_auth_token_in_server_options() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_agent_id: Some(7),
        ..Default::default()
    };
    let error = crate::serve_with_options(dir.path(), "127.0.0.1:0", options).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn malicious_ingestion_scope_bypass_is_denied_by_agent_view() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/ingest/text?scope=..%2F..%2Fsecret&source=attack HTTP/1.1\r\nAuthorization: Bearer secret\r\ncontent-length: 6\r\n\r\nbudget",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "malicious ingest scope must be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );
    assert!(
        !denied.contains("budget"),
        "denial should not echo request body: {denied}"
    );
}
