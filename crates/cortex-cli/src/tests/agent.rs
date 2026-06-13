use cortex_engine::{scope_id, ContextPackOptions, Database};
use serde_json::Value;

use super::helpers::*;

#[test]
fn agent_lifecycle_commands_create_list_show_grant_and_revoke() {
    let path = unique_path("cortexdb-cli-agent-lifecycle");
    let path_arg = path.to_string_lossy().into_owned();
    let finance_scope = scope_id("project:finance").0;

    let created = run(vec![
        "cortexdb".to_owned(),
        "agent".to_owned(),
        "create".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
        "--label".to_owned(),
        "finance-agent".to_owned(),
        "--read-scope".to_owned(),
        "project:finance".to_owned(),
        "--write-scope".to_owned(),
        "project:finance".to_owned(),
        "--mode".to_owned(),
        "audit".to_owned(),
        "--memory-type".to_owned(),
        "preference".to_owned(),
        "--allow-audit-mode".to_owned(),
        "--require-citations".to_owned(),
    ])
    .unwrap();
    assert!(created.contains("agent_id=7"));
    assert!(created.contains("label=finance-agent"));
    assert!(created.contains(&format!("readable_scopes={finance_scope}")));

    let listed = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "agent".to_owned(),
        "list".to_owned(),
        path_arg.clone(),
    ])
    .unwrap();
    let listed: Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(listed["agents"][0]["agent_id"], 7);
    assert_eq!(listed["agents"][0]["readable_scopes"][0], finance_scope);

    let grant = run(vec![
        "cortexdb".to_owned(),
        "agent".to_owned(),
        "grant".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
        "project:hr".to_owned(),
        "--access".to_owned(),
        "read_write".to_owned(),
    ])
    .unwrap();
    assert!(grant.contains("action=grant_scope"));
    assert!(grant.contains("readable_scope_count=2"));
    assert!(grant.contains("writable_scope_count=2"));

    let revoke = run(vec![
        "cortexdb".to_owned(),
        "agent".to_owned(),
        "revoke".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
        "project:hr".to_owned(),
        "--access".to_owned(),
        "read".to_owned(),
    ])
    .unwrap();
    assert!(revoke.contains("action=revoke_scope"));
    assert!(revoke.contains("readable_scope_count=1"));
    assert!(revoke.contains("writable_scope_count=2"));

    let shown = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "agent".to_owned(),
        "show".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
    ])
    .unwrap();
    let shown: Value = serde_json::from_str(&shown).unwrap();
    assert_eq!(shown["agent_id"], 7);
    assert_eq!(shown["label"], "finance-agent");
    assert_eq!(shown["readable_scopes"].as_array().unwrap().len(), 1);
    assert_eq!(shown["writable_scopes"].as_array().unwrap().len(), 2);

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn agent_lifecycle_commands_enable_two_agent_scope_isolation() {
    let path = unique_path("cortexdb-cli-agent-isolation");
    let path_arg = path.to_string_lossy().into_owned();

    run(vec![
        "cortexdb".to_owned(),
        "agent".to_owned(),
        "create".to_owned(),
        path_arg.clone(),
        "7".to_owned(),
        "--read-scope".to_owned(),
        "project:finance".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "agent".to_owned(),
        "create".to_owned(),
        path_arg.clone(),
        "8".to_owned(),
        "--read-scope".to_owned(),
        "project:hr".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:finance\nstatus=ready\nfinance budget evidence".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=project:hr\nstatus=ready\nhr onboarding evidence".to_owned(),
    ])
    .unwrap();

    let db = Database::open(&path).unwrap();
    let finance_view = db.load_agent_view(cortex_aql::AgentId(7)).unwrap().unwrap();
    let hr_view = db.load_agent_view(cortex_aql::AgentId(8)).unwrap().unwrap();
    let aql = r#"RETRIEVE CONTEXT FOR TASK "evidence" IN BRAIN default LIMIT 10 CANDIDATES;"#;

    let finance_pack = db
        .context_pack_from_aql(aql, &finance_view, ContextPackOptions::default())
        .unwrap();
    assert_eq!(finance_pack.cells.len(), 1);
    assert_eq!(finance_pack.cells[0].cell_id.0, 1);

    let hr_pack = db
        .context_pack_from_aql(aql, &hr_view, ContextPackOptions::default())
        .unwrap();
    assert_eq!(hr_pack.cells.len(), 1);
    assert_eq!(hr_pack.cells[0].cell_id.0, 2);

    let _ = std::fs::remove_dir_all(path);
}
