use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::verification::{VerificationGuardCode, VerificationStatus};
use cortex_engine::{scope_id, ContextPackAnomalyCode, ContextPackOptions, Database};
use serde::Deserialize;

const QUALITY_FIXTURE: &str = include_str!("../fixtures/context_verify_quality_v1.cells");
const REAL_DOMAIN_CHUNKS: &str =
    include_str!("../../../examples/real_domains/investment_projects/corpus/chunks.jsonl");

#[test]
fn context_and_verify_quality_fixture_is_stable_before_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_quality_fixture(&mut db);

    assert_context_quality(&db);
    assert_verify_quality(&db);
}

#[test]
fn context_and_verify_quality_fixture_survives_checkpoint_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_quality_fixture(&mut db);
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_context_quality(&db);
    assert_verify_quality(&db);
}

#[test]
fn context_pack_uses_real_domain_investment_project_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_real_domain_chunks(
        &mut db,
        &[
            "wb_p008500_c001",
            "wb_p008500_c002",
            "wb_p008499_c001",
            "wb_p008499_c002",
        ],
    );

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT
FOR TASK "Kazakhstan water infrastructure development project"
IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
LIMIT 4 CANDIDATES;"#,
            &agent_view(true, false),
            ContextPackOptions {
                token_budget_tokens: 2_000,
                require_citations: true,
                reduce_redundancy: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert!(pack.cells.len() >= 2);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    assert!(pack
        .anomalies
        .iter()
        .all(|anomaly| anomaly.code != ContextPackAnomalyCode::MissingCitation));
    assert!(pack.cells.iter().all(|cell| cell.citation.is_some()));

    let water_cells = pack
        .cells
        .iter()
        .filter(|cell| {
            let payload = String::from_utf8_lossy(&cell.payload);
            payload.contains("doc_id=wb_p008500") && payload.contains("Water Supply")
        })
        .count();
    assert_eq!(water_cells, 2);
    assert!(pack.cells.iter().all(|cell| cell
        .explain
        .as_ref()
        .is_some_and(|explain| !explain.matched_terms.is_empty())));
}

fn seed_quality_fixture(db: &mut Database) {
    for (cell_id, payload) in load_quality_fixture() {
        db.put_cell(cell_id, payload.into_bytes()).unwrap();
    }
}

fn seed_real_domain_chunks(db: &mut Database, chunk_ids: &[&str]) {
    for (index, chunk) in load_real_domain_chunks(chunk_ids).into_iter().enumerate() {
        let cell_id = CellId(1_000 + index as u64);
        db.put_cell(cell_id, real_domain_payload(&chunk).into_bytes())
            .unwrap();
    }
}

fn load_quality_fixture() -> Vec<(CellId, String)> {
    QUALITY_FIXTURE
        .trim()
        .split("\n---\n")
        .map(|record| {
            let (header, payload) = record
                .split_once('\n')
                .expect("quality fixture records must include payload");
            let id = header
                .strip_prefix("cell_id=")
                .expect("quality fixture records must start with cell_id=")
                .parse::<u64>()
                .expect("quality fixture cell_id must be u64");
            (CellId(id), payload.to_owned())
        })
        .collect()
}

fn load_real_domain_chunks(chunk_ids: &[&str]) -> Vec<RealDomainChunk> {
    let wanted = chunk_ids.iter().copied().collect::<BTreeSet<_>>();
    let chunks_by_id = REAL_DOMAIN_CHUNKS
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<RealDomainChunk>(line).unwrap())
        .filter(|chunk| wanted.contains(chunk.chunk_id.as_str()))
        .map(|chunk| (chunk.chunk_id.clone(), chunk))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(chunks_by_id.len(), chunk_ids.len());
    chunk_ids
        .iter()
        .map(|chunk_id| chunks_by_id.get(*chunk_id).unwrap().clone())
        .collect()
}

fn real_domain_payload(chunk: &RealDomainChunk) -> String {
    format!(
        "scope=project:investments\nstatus=ready\ntype=document_block\nsource={}:{}\ndoc_id={}\nchunk_id={}\ntitle={}\nsector={}\n\n{}",
        chunk.source,
        chunk.doc_id,
        chunk.doc_id,
        chunk.chunk_id,
        chunk.title,
        chunk.sector,
        chunk.text
    )
}

fn assert_context_quality(db: &Database) {
    let pack = db
        .context_pack_from_aql(
            context_query(),
            &agent_view(true, false),
            ContextPackOptions {
                token_budget_tokens: 1_000,
                require_citations: true,
                reduce_redundancy: true,
                redundancy_threshold_q16: 10,
                citation_overhead_tokens: cortex_engine::context::DEFAULT_CITATION_OVERHEAD_TOKENS,
                token_profile: cortex_engine::ContextTokenProfile::default(),
                large_cell_policy: cortex_engine::ContextLargeCellPolicy::default(),
                span_level_packing: false,
                span_context_lines: 2,
            },
        )
        .unwrap();

    let ids = pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![CellId(1), CellId(2)]);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    assert!(!pack.truncated);
    assert!(pack.citations_required);
    assert!(pack.anomalies.iter().all(|anomaly| {
        anomaly.code != ContextPackAnomalyCode::MissingCitation
            && anomaly.code != ContextPackAnomalyCode::RedundantCell
    }));
    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.citation.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("report-q1.pdf#page=3"), Some("report-q2.pdf#page=5")]
    );
    assert!(pack.cells.iter().all(|cell| cell
        .explain
        .as_ref()
        .is_some_and(|explain| explain.matched_terms.iter().any(|term| term == "budget"))));
}

fn assert_verify_quality(db: &Database) {
    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT for 2025" IN BRAIN investment_projects;"#,
            &agent_view(true, true),
        )
        .unwrap();

    assert_eq!(report.status, VerificationStatus::Mixed);
    assert_eq!(report.evidence.len(), 1);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(
        report.evidence[0].citation.as_deref(),
        Some("report-q1.pdf#page=3")
    );
    assert_eq!(report.contradicting_evidence.len(), 1);
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(2));
    assert_eq!(
        report.contradicting_evidence[0].citation.as_deref(),
        Some("report-q2.pdf#page=5")
    );
    assert!(report.guards.iter().any(|guard| {
        guard.cell_id == Some(CellId(2)) && guard.code == VerificationGuardCode::NumericMismatch
    }));
    assert!(report
        .guards
        .iter()
        .all(|guard| guard.code != VerificationGuardCode::MissingCitation));
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT
FOR TASK "Solar Plant budget 2025"
IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready"
LIMIT 10 CANDIDATES;"#
}

fn agent_view(require_citations: bool, allow_verify: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("quality-fixture-agent".to_owned()),
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
        require_citations_by_default: require_citations,
        private_scope: None,
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RealDomainChunk {
    chunk_id: String,
    doc_id: String,
    source: String,
    sector: String,
    title: String,
    text: String,
}
