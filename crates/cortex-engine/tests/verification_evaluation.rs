use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::verification::{VerificationGuardCode, VerificationStatus};
use cortex_engine::{scope_id, Database};
use serde::Deserialize;

const CASES: &str = include_str!("../../../examples/eval/verification_cases.jsonl");

#[test]
fn verification_evaluation_cases_match_expected_statuses() {
    for case in load_cases() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        for cell in &case.cells {
            db.put_knowledge_cell(
                CellId(cell.cell_id),
                fact_cell(&cell.scope, cell.source.as_deref(), &cell.body),
            )
            .unwrap();
        }

        let report = db
            .verify_fact_aql(&verify_aql(&case.fact), &view("project:investments"))
            .unwrap();
        assert_eq!(
            status_name(report.status),
            case.expected_status,
            "{} status mismatch",
            case.case_id
        );
        assert!(
            report.evidence.len() >= case.min_supporting,
            "{} supporting evidence too small",
            case.case_id
        );
        assert!(
            report.contradicting_evidence.len() >= case.min_contradicting,
            "{} contradicting evidence too small",
            case.case_id
        );

        let guard_codes = report
            .guards
            .iter()
            .map(|guard| guard_code_name(guard.code))
            .collect::<BTreeSet<_>>();
        for expected in &case.expected_guard_codes {
            assert!(
                guard_codes.contains(expected.as_str()),
                "{} missing guard {}",
                case.case_id,
                expected
            );
        }
    }
}

fn load_cases() -> Vec<VerificationCase> {
    CASES
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<VerificationCase>(line).unwrap())
        .collect()
}

fn verify_aql(fact: &str) -> String {
    format!(r#"VERIFY FACT "{fact}" IN BRAIN investment_projects;"#)
}

fn fact_cell(scope: &str, source: Option<&str>, body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: scope.to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: None,
            source: source.map(str::to_owned),
        },
        body,
    )
}

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("verification-evaluation-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
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
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn status_name(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed",
    }
}

fn guard_code_name(code: VerificationGuardCode) -> &'static str {
    match code {
        VerificationGuardCode::MissingCitation => "missing_citation",
        VerificationGuardCode::NumericMismatch => "numeric_mismatch",
    }
}

#[derive(Debug, Deserialize)]
struct VerificationCase {
    case_id: String,
    fact: String,
    expected_status: String,
    expected_guard_codes: Vec<String>,
    min_supporting: usize,
    min_contradicting: usize,
    cells: Vec<VerificationCell>,
}

#[derive(Debug, Deserialize)]
struct VerificationCell {
    cell_id: u64,
    scope: String,
    source: Option<String>,
    body: String,
}
