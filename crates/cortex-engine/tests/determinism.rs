use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, ContextPackOptions, Database, SearchLimit, VerificationEvidence,
    VerificationNumericConflict, VerificationReport,
};

#[test]
fn search_results_are_repeatable_and_tie_break_by_cell_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_search_tie_cells(&mut db);

    let before = search_snapshot(&db);
    assert_eq!(
        before,
        "10:budget evidence one|20:budget evidence two|30:budget evidence three"
    );
    assert_eq!(search_snapshot(&db), before);

    db.checkpoint().unwrap();
    assert_eq!(search_snapshot(&db), before);
}

#[test]
fn context_pack_output_is_repeatable_and_snapshotted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_determinism_cells(&mut db);

    let before = context_snapshot(&db);
    assert_eq!(
        before,
        [
            "cells=10:doc-a:solar+plant+budget;20:doc-b:solar+plant+budget;30:doc-c:solar+plant+budget",
            "anomalies=",
        ]
        .join("\n")
    );
    assert_eq!(context_snapshot(&db), before);

    db.checkpoint().unwrap();
    assert_eq!(context_snapshot(&db), before);
}

#[test]
fn verification_report_output_is_repeatable_and_snapshotted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_determinism_cells(&mut db);

    let before = verification_snapshot(&db);
    assert_eq!(
        before,
        [
            "status=mixed_evidence",
            "evidence=10:9:50000:doc-a;30:9:50000:doc-c",
            "contradicting=20:6:50000:doc-b",
            "guards=20:numeric_mismatch",
            "numeric_conflicts=20:budget:1.2B KZT:1.4B KZT",
        ]
        .join("\n")
    );
    assert_eq!(verification_snapshot(&db), before);

    db.checkpoint().unwrap();
    assert_eq!(verification_snapshot(&db), before);
}

fn seed_search_tie_cells(db: &mut Database) {
    for (cell_id, source, body) in [
        (30, "doc-c", "budget evidence three"),
        (10, "doc-a", "budget evidence one"),
        (20, "doc-b", "budget evidence two"),
    ] {
        db.put_cell(
            CellId(cell_id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nsource={source}\nsource_trust_q16=50000\n\n{body}",
            )
            .into_bytes(),
        )
        .unwrap();
    }
}

fn seed_determinism_cells(db: &mut Database) {
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
        )
        .unwrap();
    }
}

fn search_snapshot(db: &Database) -> String {
    db.search_keyword("budget", &view(true), SearchLimit(10))
        .unwrap()
        .into_iter()
        .map(|result| {
            format!(
                "{}:{}",
                result.cell_id.0,
                body_line(&result.payload, "budget evidence")
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn context_snapshot(db: &Database) -> String {
    let pack = db
        .context_pack_from_aql(
            query(),
            &view(true),
            ContextPackOptions {
                token_budget_tokens: 1_000,
                require_citations: true,
                reduce_redundancy: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            let matched = cell
                .explain
                .as_ref()
                .map(|explain| explain.matched_terms.join("+"))
                .unwrap_or_default();
            format!(
                "{}:{}:{}",
                cell.cell_id.0,
                cell.citation.as_deref().unwrap_or(""),
                matched
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    let anomalies = pack
        .anomalies
        .iter()
        .map(|anomaly| {
            format!(
                "{}:{}",
                anomaly.cell_id.map(|id| id.0).unwrap_or_default(),
                anomaly.code.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!("cells={cells}\nanomalies={anomalies}")
}

fn verification_snapshot(db: &Database) -> String {
    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT for 2025" IN BRAIN investment_projects;"#,
            &view(true),
        )
        .unwrap();
    format!(
        "status={}\nevidence={}\ncontradicting={}\nguards={}\nnumeric_conflicts={}",
        report.status.as_str(),
        evidence_snapshot(&report.evidence),
        evidence_snapshot(&report.contradicting_evidence),
        guard_snapshot(&report),
        numeric_conflict_snapshot(&report.numeric_conflicts)
    )
}

fn evidence_snapshot(evidence: &[VerificationEvidence]) -> String {
    evidence
        .iter()
        .map(|item| {
            format!(
                "{}:{}:{}:{}",
                item.cell_id.0,
                item.matched_terms,
                item.source_trust_q16,
                item.citation.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn guard_snapshot(report: &VerificationReport) -> String {
    report
        .guards
        .iter()
        .map(|guard| {
            format!(
                "{}:{}",
                guard.cell_id.map(|id| id.0).unwrap_or_default(),
                guard.code.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn numeric_conflict_snapshot(conflicts: &[VerificationNumericConflict]) -> String {
    conflicts
        .iter()
        .map(|conflict| {
            format!(
                "{}:{}:{}:{}",
                conflict.cell_id.0, conflict.metric, conflict.left, conflict.right
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn body_line(payload: &[u8], needle: &str) -> String {
    String::from_utf8_lossy(payload)
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_default()
        .to_owned()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "Solar Plant budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#
}

fn view(allow_verify: bool) -> AgentView {
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
        allow_verify_fact: allow_verify,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
