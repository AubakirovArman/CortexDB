use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, ContextPackOptions, Database, EngineResult, SearchLimit, StaleLockPolicy,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = temp_path("cortex-engine-api-compat-db");
    let backup = temp_path("cortex-engine-api-compat-backup");
    let restore = temp_path("cortex-engine-api-compat-restore");
    cleanup(&root);
    cleanup(&backup);
    cleanup(&restore);

    run_compatibility_flow(&root, &backup, &restore)?;

    cleanup(&root);
    cleanup(&backup);
    cleanup(&restore);
    Ok(())
}

fn run_compatibility_flow(root: &PathBuf, backup: &PathBuf, restore: &PathBuf) -> EngineResult<()> {
    let view = agent_view(true);
    let mut db = Database::open(root)?;

    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=api-doc\nABC budget approved\nalpha budget approved".to_vec(),
    )?;
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=api-doc\nsecondary alpha schedule"
            .to_vec(),
    )?;

    assert!(db.current_seq().0 >= 2);
    assert!(db
        .get_latest_cell(CellId(1))
        .expect("cell 1 must be visible")
        .windows(b"ABC budget approved".len())
        .any(|window| window == b"ABC budget approved"));

    let search = db.search_keyword("budget", &view, SearchLimit(10))?;
    assert_eq!(search[0].cell_id, CellId(1));

    let pack = db.context_pack_from_aql(
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
        &view,
        ContextPackOptions {
            token_budget_tokens: 256,
            ..ContextPackOptions::default()
        },
    )?;
    assert!(!pack.cells.is_empty());

    let verification = db.verify_fact_aql(
        r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
        &view,
    )?;
    assert_eq!(verification.status.as_str(), "supported");

    let checkpoint = db.checkpoint()?;
    assert!(checkpoint.cells_flushed >= 2);
    db.close()?;

    let backup_report = Database::backup_path(root, backup)?;
    assert!(backup_report.files_copied > 0);
    let restore_report = Database::restore_from_backup(backup, restore)?;
    assert!(restore_report.files_copied > 0);

    let restored = Database::open_with_options(
        restore,
        cortex_engine::DatabaseOptions {
            stale_lock_policy: StaleLockPolicy::Reject,
            ..cortex_engine::DatabaseOptions::default()
        },
    )?;
    assert!(restored.get_latest_cell(CellId(1)).is_some());
    restored.close()?;
    Ok(())
}

fn agent_view(allow_verify: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("external-sample".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: allow_verify,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn temp_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn cleanup(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}
