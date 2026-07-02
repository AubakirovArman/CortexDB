use std::collections::BTreeSet;
use std::fs;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_crypto::hex_lower;
use cortex_engine::canonical::{canonical_context_pack_bytes, canonical_verification_report_bytes};
use cortex_engine::{scope_id, ContextPackOptions, Database, EngineResult};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    context_pack_bytes: String,
    context_determinism_hash: String,
    verify_context_pack_bytes: String,
    verify_report_bytes: String,
    verify_determinism_hash: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "cortexdb-pack-determinism-hash-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;

    let before = {
        let mut db = Database::open(&dir)?;
        seed(&mut db)?;
        let before = snapshot(&db)?;
        db.checkpoint()?;
        before
    };
    let after = {
        let db = Database::open(&dir)?;
        snapshot(&db)?
    };

    print_snapshot("before", &before);
    print_snapshot("after", &after);

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

fn seed(db: &mut Database) -> EngineResult<()> {
    for (cell_id, source, body) in [
        (
            30,
            "doc-c",
            "metric=budget\nSolar Plant budget is 1.2B KZT for 2025\nbudget evidence three",
        ),
        (
            10,
            "doc-a",
            "metric=budget\nSolar Plant budget is 1.2B KZT for 2025\nbudget evidence one",
        ),
        (
            20,
            "doc-b",
            "metric=budget\ncontradicts=Solar Plant budget is 1.2B KZT for 2025\nSolar Plant budget is 1.4B KZT for 2025\nbudget evidence two",
        ),
    ] {
        db.put_cell(
            CellId(cell_id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nsource={source}\nsource_trust_q16=50000\n\n{body}",
            )
            .into_bytes(),
        )?;
    }
    Ok(())
}

fn snapshot(db: &Database) -> EngineResult<Snapshot> {
    let context = db.context_pack_with_receipt_evidence_from_aql(
        context_query(),
        &view(),
        ContextPackOptions {
            token_budget_tokens: 1_000,
            require_citations: true,
            reduce_redundancy: false,
            ..ContextPackOptions::default()
        },
    )?;
    let (report, verify_context) =
        db.verify_fact_with_receipt_evidence_aql(verify_query(), &view())?;

    Ok(Snapshot {
        context_pack_bytes: hex_lower(&canonical_context_pack_bytes(&context.pack)),
        context_determinism_hash: context.determinism_hash(),
        verify_context_pack_bytes: hex_lower(&canonical_context_pack_bytes(&verify_context.pack)),
        verify_report_bytes: hex_lower(&canonical_verification_report_bytes(&report)),
        verify_determinism_hash: verify_context.determinism_hash(),
    })
}

fn print_snapshot(prefix: &str, snapshot: &Snapshot) {
    println!("{prefix}_context_pack={}", snapshot.context_pack_bytes);
    println!(
        "{prefix}_context_determinism_hash={}",
        snapshot.context_determinism_hash
    );
    println!(
        "{prefix}_verify_context_pack={}",
        snapshot.verify_context_pack_bytes
    );
    println!("{prefix}_verify_report={}", snapshot.verify_report_bytes);
    println!(
        "{prefix}_verify_determinism_hash={}",
        snapshot.verify_determinism_hash
    );
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "Solar Plant budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#
}

fn verify_query() -> &'static str {
    r#"VERIFY FACT "Solar Plant budget is 1.2B KZT for 2025" IN BRAIN investment_projects;"#
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("determinism-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 2_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
