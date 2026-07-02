//! Track F: per-PR retrieval-recall regression baseline.
//!
//! A small, deterministic, self-contained labeled fixture evaluated through the
//! real `RETRIEVE CONTEXT` ranking path (`retrieve_aql` -> `rank_retrieved_cells`
//! -> the score fusion). It computes mean recall@k and mean reciprocal rank in
//! Q16 and asserts stable floors, so a future change to retrieval or the score
//! fusion (e.g. the A1.1 fusion-normalization work) can be validated against a
//! measurable baseline instead of an ad-hoc per-change procedure. It calls no
//! external service, no LLM judge, and runs in well under a second.

use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database};

const Q16_ONE: u64 = 65_535;

/// Labeled retrieval fixture: (task query, relevant cell ids).
///
/// Cells are seeded with uniform trust/recency so ranking is driven by lexical
/// relevance; each query's terms match its relevant cell most strongly.
const CASES: &[(&str, &[u64])] = &[
    ("solar plant capital budget approved", &[101]),
    ("wind farm operating budget rejected", &[102]),
    ("solar panel maintenance schedule roster", &[103]),
    ("hydro dam construction timeline delayed", &[104]),
    ("solar plant staffing headcount rotation", &[105]),
    ("procurement audit findings vendor risk", &[106]),
];

fn seed_labeled_corpus(db: &mut Database) {
    // Uniform trust + created time: isolate lexical relevance for a stable
    // baseline. Adversarial recency/trust cases belong to the A1.1 fusion work.
    const TRUST: u16 = 50_000;
    const CREATED: u64 = 1_700_000_000;
    for (cell_id, source, body) in [
        (
            101u64,
            "doc-solar-budget",
            "solar plant capital budget approved 1.2B KZT for 2025",
        ),
        (
            102,
            "doc-wind-budget",
            "wind farm operating budget rejected by the finance committee",
        ),
        (
            103,
            "doc-solar-maint",
            "solar panel maintenance schedule quarterly cleaning roster",
        ),
        (
            104,
            "doc-hydro-timeline",
            "hydro dam construction timeline delayed to the next quarter",
        ),
        (
            105,
            "doc-solar-staff",
            "solar plant staffing headcount plan and shift rotation",
        ),
        (
            106,
            "doc-audit",
            "procurement audit findings on vendor risk and controls",
        ),
    ] {
        db.put_cell(
            CellId(cell_id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nsource={source}\nsource_trust_q16={TRUST}\ncreated_unix_seconds={CREATED}\n\n{body}",
            )
            .into_bytes(),
        )
        .unwrap();
    }
}

fn ranked_cell_ids(db: &Database, task: &str, limit: usize) -> Vec<u64> {
    let aql = format!(
        "RETRIEVE CONTEXT FOR TASK \"{task}\" IN BRAIN default \
         WHERE space = project:investments LIMIT {limit} CANDIDATES;"
    );
    db.retrieve_aql(&aql, &view())
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id.0)
        .collect()
}

/// Returns (mean recall@k in Q16, mean reciprocal rank in Q16).
fn evaluate(db: &Database, k: usize) -> (u32, u32) {
    let mut recall_sum = 0u64;
    let mut rr_sum = 0u64;
    for (task, relevant) in CASES {
        let ranked = ranked_cell_ids(db, task, 10.max(k));
        let topk: Vec<u64> = ranked.iter().copied().take(k).collect();
        let found = relevant.iter().filter(|id| topk.contains(id)).count();
        let recall = (found as u64 * Q16_ONE) / relevant.len() as u64;
        recall_sum += recall;
        let rr = ranked
            .iter()
            .position(|id| relevant.contains(id))
            .map(|pos| Q16_ONE / (pos as u64 + 1))
            .unwrap_or(0);
        rr_sum += rr;
        eprintln!(
            "query={task:?} relevant={relevant:?} top{k}={topk:?} recall@{k}={recall} rr={rr}"
        );
    }
    let n = CASES.len() as u64;
    ((recall_sum / n) as u32, (rr_sum / n) as u32)
}

#[test]
fn retrieval_recall_baseline_over_labeled_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_labeled_corpus(&mut db);

    let (mean_recall_at_5, mean_rr) = evaluate(&db, 5);
    eprintln!("mean_recall@5={mean_recall_at_5} mean_reciprocal_rank={mean_rr}");

    // Stable regression floors. These must PASS on current ranking; a future
    // fusion change must not regress below them (ideally it raises mean_rr).
    assert_eq!(
        mean_recall_at_5, 65_535,
        "every relevant cell must be retrieved within the top-5"
    );
    assert!(
        mean_rr >= 32_768,
        "the relevant cell should typically rank in the top ~2 (mean_rr={mean_rr})"
    );
}

#[test]
fn retrieval_recall_is_deterministic_across_runs() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_labeled_corpus(&mut db);

    let first = evaluate(&db, 5);
    let second = evaluate(&db, 5);
    assert_eq!(first, second, "retrieval metrics must be deterministic");
}

/// Adversarial A1.1 case: a strong lexical match must outrank a query-irrelevant
/// cell that merely has maximum source trust and the newest timestamp. Under
/// `USING MODE fast` the lexical weight (0.55) exceeds recency+trust (0.25+0.10),
/// so a scale-correct fusion ranks the relevant cell first. The pre-fix fusion
/// multiplied recency/trust by 1024, letting them swamp the lexical signal and
/// rank the irrelevant-but-recent cell first.
#[test]
fn fast_mode_lexical_match_outranks_recent_high_trust_distractor() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    // Relevant to the query, but OLD and LOW trust.
    db.put_cell(
        CellId(301),
        b"scope=project:investments\nstatus=ready\ntype=fact\nsource=doc-relevant\nsource_trust_q16=10000\ncreated_unix_seconds=1000000000\n\nquantum encryption protocol key exchange design"
            .to_vec(),
    )
    .unwrap();
    // Irrelevant to the query, but NEWEST and MAX trust.
    db.put_cell(
        CellId(302),
        b"scope=project:investments\nstatus=ready\ntype=fact\nsource=doc-distractor\nsource_trust_q16=65535\ncreated_unix_seconds=2000000000\n\nweekly cafeteria lunch menu and parking notice"
            .to_vec(),
    )
    .unwrap();

    let aql = "RETRIEVE CONTEXT FOR TASK \"quantum encryption protocol\" IN BRAIN default \
               USING MODE fast WHERE space = project:investments LIMIT 10 CANDIDATES;";
    let ranked: Vec<u64> = db
        .retrieve_aql(aql, &view())
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id.0)
        .collect();

    assert_eq!(
        ranked.first().copied(),
        Some(301),
        "the lexical match (301) must outrank the recent/high-trust distractor (302); ranked={ranked:?}"
    );
}

/// Cross-mode fusion behavior: in `USING MODE audit` (trust weight 0.40, the
/// highest) two equally-relevant cells are ordered by source trust. This can
/// only hold once trust is min-max normalized to a scale comparable with the
/// other components.
#[test]
fn audit_mode_prioritizes_higher_source_trust() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let body = "quarterly budget report figures and totals";
    for (cell_id, source, trust) in [
        (401u64, "doc-low-trust", 20_000),
        (402, "doc-high-trust", 65_000),
    ] {
        db.put_cell(
            CellId(cell_id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nsource={source}\nsource_trust_q16={trust}\ncreated_unix_seconds=1700000000\n\n{body}",
            )
            .into_bytes(),
        )
        .unwrap();
    }

    let aql = "RETRIEVE CONTEXT FOR TASK \"quarterly budget report\" IN BRAIN default \
               USING MODE audit WHERE space = project:investments LIMIT 10 CANDIDATES;";
    let ranked: Vec<u64> = db
        .retrieve_aql(aql, &view())
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id.0)
        .collect();

    assert_eq!(
        ranked.first().copied(),
        Some(402),
        "audit mode must rank the higher-trust cell first; ranked={ranked:?}"
    );
}

/// Among equally-relevant, equally-trusted cells, recency breaks the tie: the
/// more recent cell ranks first because its normalized recency component is
/// higher.
#[test]
fn recency_breaks_ties_between_equally_relevant_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let body = "annual maintenance summary report";
    for (cell_id, source, created) in [
        (501u64, "doc-older", 1_000_000_000u64),
        (502, "doc-newer", 1_900_000_000),
    ] {
        db.put_cell(
            CellId(cell_id),
            format!(
                "scope=project:investments\nstatus=ready\ntype=fact\nsource={source}\nsource_trust_q16=50000\ncreated_unix_seconds={created}\n\n{body}",
            )
            .into_bytes(),
        )
        .unwrap();
    }

    let aql = "RETRIEVE CONTEXT FOR TASK \"annual maintenance summary\" IN BRAIN default \
               WHERE space = project:investments LIMIT 10 CANDIDATES;";
    let ranked: Vec<u64> = db
        .retrieve_aql(aql, &view())
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id.0)
        .collect();

    assert_eq!(
        ranked.first().copied(),
        Some(502),
        "the more recent cell must break the tie; ranked={ranked:?}"
    );
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([
            RetrievalMode::Balanced,
            RetrievalMode::Fast,
            RetrievalMode::Audit,
        ]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 2_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: true,
        require_citations_by_default: false,
        private_scope: None,
    }
}
