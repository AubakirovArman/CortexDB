use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, EngineErrorCode};

const FUZZ_CASES: usize = 96;

#[test]
fn generated_where_not_and_or_queries_do_not_bypass_scope_policy() {
    let dir = tempfile::tempdir().unwrap();
    let mut generated = generated_where_expressions();
    generated.extend(explicit_edge_cases());

    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_scoped_cells(&mut db);
        assert_no_scope_bypass(&db, &generated);
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_no_scope_bypass(&db, &generated);
}

fn assert_no_scope_bypass(db: &Database, where_expressions: &[String]) {
    let view = alpha_only_view();
    let mut ok_count = 0usize;
    let mut denied_count = 0usize;
    for expression in where_expressions {
        let query = format!(
            "RETRIEVE CONTEXT FOR TASK \"fuzz\" IN BRAIN investment_projects \
             WHERE {expression} LIMIT 20 CANDIDATES;"
        );
        match db.retrieve_aql(&query, &view) {
            Ok(cells) => {
                ok_count += 1;
                for cell in cells {
                    let payload = String::from_utf8_lossy(&cell.payload);
                    assert!(
                        payload.contains("scope=project:alpha"),
                        "scope bypass for expression `{expression}` returned cell {:?}: {}",
                        cell.cell_id,
                        payload
                    );
                    assert!(
                        !payload.contains("scope=project:beta"),
                        "unreadable beta cell leaked for expression `{expression}`"
                    );
                }
            }
            Err(error) if error.code() == EngineErrorCode::PermissionDenied => {
                denied_count += 1;
            }
            Err(error) => panic!("unexpected error for expression `{expression}`: {error}"),
        }
    }

    assert!(
        ok_count > 0,
        "fuzz corpus must include successful safe queries"
    );
    assert!(
        denied_count > 0,
        "fuzz corpus must include fail-closed unreadable-scope queries"
    );
}

fn seed_scoped_cells(db: &mut Database) {
    let cells = [
        (
            1,
            "scope=project:alpha\nstatus=ready\ntype=fact\nalpha ready fact",
        ),
        (
            2,
            "scope=project:alpha\nstatus=draft\ntype=document_block\nalpha draft block",
        ),
        (
            3,
            "scope=project:beta\nstatus=ready\ntype=fact\nbeta ready fact",
        ),
        (
            4,
            "scope=project:beta\nstatus=draft\ntype=document_block\nbeta draft block",
        ),
    ];
    for (id, payload) in cells {
        db.put_cell(CellId(id), payload.as_bytes().to_vec())
            .unwrap();
    }
}

fn generated_where_expressions() -> Vec<String> {
    let mut rng = DeterministicRng::new(0x00C0_DBA1_1A5E_C043);
    (0..FUZZ_CASES)
        .map(|case| generate_expression(&mut rng, 1 + (case % 3)))
        .collect()
}

fn explicit_edge_cases() -> Vec<String> {
    vec![
        "NOT (status = \"ready\")".to_owned(),
        "(status = \"ready\") OR (status = \"draft\")".to_owned(),
        "(space = project:alpha) OR (status = \"ready\")".to_owned(),
        "NOT ((space = project:alpha) AND (status = \"ready\"))".to_owned(),
        "(space = project:beta) OR (status = \"ready\")".to_owned(),
        "NOT (space = project:beta)".to_owned(),
    ]
}

fn generate_expression(rng: &mut DeterministicRng, depth: usize) -> String {
    if depth == 0 {
        return generate_predicate(rng);
    }
    match rng.next_usize(4) {
        0 => generate_predicate(rng),
        1 => format!("NOT ({})", generate_expression(rng, depth - 1)),
        2 => format!(
            "({}) AND ({})",
            generate_expression(rng, depth - 1),
            generate_expression(rng, depth - 1)
        ),
        _ => format!(
            "({}) OR ({})",
            generate_expression(rng, depth - 1),
            generate_expression(rng, depth - 1)
        ),
    }
}

fn generate_predicate(rng: &mut DeterministicRng) -> String {
    const PREDICATES: &[&str] = &[
        "space = project:alpha",
        "scope = project:alpha",
        "space = project:beta",
        "scope = project:beta",
        "status = \"ready\"",
        "status = \"draft\"",
        "type = \"fact\"",
        "type = \"document_block\"",
    ];
    PREDICATES[rng.next_usize(PREDICATES.len())].to_owned()
}

fn alpha_only_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:alpha")]),
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
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_usize(&mut self, upper: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((self.0 >> 32) as usize) % upper
    }
}
