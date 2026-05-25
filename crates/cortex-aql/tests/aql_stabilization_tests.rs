use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    decimal_to_q16, eval_bitmap_program, parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement,
    BindError, Binder, BitmapHandle, BrainId, DecimalLiteral, MemoryType, MockBitmapProvider,
    RetrievalMode, ScopeId, Q16_ONE, Q16_ZERO,
};

#[derive(Default)]
struct Catalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<String, ScopeId>,
    scope_bitmaps: BTreeMap<ScopeId, BitmapHandle>,
    status_bitmaps: BTreeMap<String, BitmapHandle>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, name: &str) -> Option<ScopeId> {
        self.scopes.get(name).copied()
    }

    fn scope_bitmap(&self, scope: ScopeId) -> Option<BitmapHandle> {
        self.scope_bitmaps.get(&scope).copied()
    }

    fn status_bitmap(&self, status: &str) -> Option<BitmapHandle> {
        self.status_bitmaps.get(status).copied()
    }

    fn cell_type_bitmap(&self, _memory_type: MemoryType) -> Option<BitmapHandle> {
        None
    }

    fn field_is_filterable(&self, field: &str) -> bool {
        matches!(field, "space" | "status")
    }
}

fn catalog() -> Catalog {
    Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        scopes: BTreeMap::from([("project:investments".to_owned(), ScopeId(10))]),
        scope_bitmaps: BTreeMap::from([(ScopeId(10), BitmapHandle(100))]),
        status_bitmaps: BTreeMap::from([("ready".to_owned(), BitmapHandle(200))]),
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([ScopeId(10)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced, RetrievalMode::Fast]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: 3_600,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: ScopeId(99),
    }
}

fn retrieve(query: &str) -> Box<cortex_aql::RawRetrieveContext<'_>> {
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    raw
}

fn set(values: &[u32]) -> BTreeSet<u32> {
    values.iter().copied().collect()
}

#[test]
fn retrieve_without_mode_defaults_to_balanced() {
    let raw = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain BUDGET 100 TOKENS;"#);
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.mode, RetrievalMode::Balanced);
}

#[test]
fn retrieve_without_budget_uses_agent_view_default() {
    let raw = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#);
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.context_policy.budget_tokens, 400);
}

#[test]
fn limit_candidates_is_clamped_by_policy() {
    let raw = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain LIMIT 500 CANDIDATES;"#);
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.context_policy.candidate_limit, 100);
}

#[test]
fn require_confidence_affects_quality_thresholds() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE confidence >= 0.80;"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.quality_thresholds.min_confidence_q16, 52_428);
}

#[test]
fn require_source_trust_affects_quality_thresholds() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE source_trust >= 0.90;"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.quality_thresholds.min_source_trust_q16, 58_982);
}

#[test]
fn require_freshness_affects_quality_thresholds() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE freshness <= 86400 SECONDS;"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.quality_thresholds.max_freshness_seconds, Some(86_400));
}

#[test]
fn require_citations_enables_citation_requirement() {
    let raw = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain REQUIRE citations;"#);
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert!(plan.context_policy.require_citations);
}

#[test]
fn decimal_to_q16_rejects_values_above_one() {
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("0.0")).unwrap(),
        Q16_ZERO
    );
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("0.5")).unwrap(),
        32_768
    );
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("1.0")).unwrap(),
        Q16_ONE
    );
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("1.01")).unwrap_err(),
        BindError::InvalidDecimal
    );
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("2.0")).unwrap_err(),
        BindError::InvalidDecimal
    );
}

#[test]
fn identifier_scope_literal_binds_to_bitmap() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
WHERE space = project:investments;"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert!(plan
        .bitmap_program
        .ops
        .contains(&cortex_aql::BitmapOp::Push(BitmapHandle(100))));
}

#[test]
fn not_status_compiles_and_evaluates_as_universe_complement() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
WHERE NOT status = "ready";"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    let provider = MockBitmapProvider {
        bitmaps: BTreeMap::from([(BitmapHandle(200), set(&[2, 4]))]),
        agent_allowed: set(&[1, 2, 3, 4]),
        live: set(&[1, 2, 3, 4]),
        universe: set(&[1, 2, 3, 4]),
    };
    let mask = eval_bitmap_program(&plan.bitmap_program, &provider).unwrap();
    assert_eq!(mask, set(&[1, 3]));
}
