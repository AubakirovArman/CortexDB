use super::helpers::*;

#[test]
fn admin_can_create_list_and_show_agent_views() {
    let dir = tempfile::tempdir().unwrap();
    let options = admin_and_data_options();

    let body = r#"{
      "agent_id":17,
      "label":"finance analyst",
      "readable_scopes":["finance"],
      "writable_scopes":["finance:notes"],
      "allowed_modes":["fast","balanced","audit"],
      "allowed_memory_types":["decision","observation"],
      "allow_audit_mode":true,
      "require_citations_by_default":true,
      "private_scope":"finance:private"
    }"#;
    let create = handle_http_with_options(
        dir.path(),
        &post_with_body("/v1/agents", "admin-secret", body),
        &options,
    );
    assert!(create.contains("200 OK"), "create should succeed: {create}");
    let value = body_json(&create);
    assert_eq!(value["schema_version"], "cortexdb.agent_view.v1");
    assert_eq!(value["agent_id"], 17);
    assert_eq!(value["label"], "finance analyst");
    assert_eq!(value["readable_scopes"][0], scope_id("finance").0);
    assert_eq!(value["writable_scopes"][0], scope_id("finance:notes").0);
    assert_eq!(value["private_scope"], scope_id("finance:private").0);
    assert_eq!(value["allow_audit_mode"], true);

    let list = handle_http_with_options(
        dir.path(),
        "GET /v1/agents HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(list.contains("200 OK"), "list should succeed: {list}");
    let value = body_json(&list);
    assert_eq!(value["schema_version"], "cortexdb.agent_views.v1");
    assert_eq!(value["agents"][0]["agent_id"], 17);

    let show = handle_http_with_options(
        dir.path(),
        "GET /v1/agents/17 HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(show.contains("200 OK"), "show should succeed: {show}");
    let value = body_json(&show);
    assert_eq!(value["agent_id"], 17);
    assert_eq!(value["allowed_modes"][2], "audit");

    let db = Database::open(dir.path()).unwrap();
    let view = db.load_agent_view(AgentId(17)).unwrap().unwrap();
    assert!(view.can_read_scope(scope_id("finance")));
    assert!(view.can_write_scope(scope_id("finance:notes")));
    assert!(view.can_use_audit_mode());
}

#[test]
fn data_token_cannot_manage_agent_views() {
    let dir = tempfile::tempdir().unwrap();
    let options = admin_and_data_options();

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/agents HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "data token should not list agents: {denied}"
    );
    assert!(
        denied.contains("token role is not allowed"),
        "denial should come from role policy: {denied}"
    );
}
