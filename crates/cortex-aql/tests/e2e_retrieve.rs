use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    eval_bitmap_program, parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement, Binder,
    BitmapHandle, BrainId, CellTypeId, MemoryType, MockBitmapProvider, RetrievalMode, ScopeId,
    StatusId, Q16_ZERO,
};

struct Catalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<String, ScopeId>,
    scope_bitmaps: BTreeMap<ScopeId, BitmapHandle>,
    status_bitmaps: BTreeMap<StatusId, BitmapHandle>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        self.scopes.get(name).copied()
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        (status == "ready").then_some(StatusId(1))
    }

    fn resolve_cell_type(&self, _brain: BrainId, _cell_type: &str) -> Option<CellTypeId> {
        None
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        self.scope_bitmaps.get(&scope).copied()
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        self.status_bitmaps.get(&status).copied()
    }

    fn cell_type_bitmap(&self, _brain: BrainId, _cell_type: CellTypeId) -> Option<BitmapHandle> {
        None
    }

    fn memory_type_bitmap(
        &self,
        _brain: BrainId,
        _memory_type: MemoryType,
    ) -> Option<BitmapHandle> {
        None
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(field, "space" | "status")
    }
}

fn set(values: &[u32]) -> BTreeSet<u32> {
    values.iter().copied().collect()
}

#[test]
fn retrieve_pipeline_produces_candidates_0() {
    let query = r#"RETRIEVE CONTEXT
FOR TASK "Сравнить бюджеты инвестиционных проектов ТОО ABC за 2025 год"
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
WHERE space = "project:investments" AND status = "ready";"#;
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };

    let catalog = Catalog {
        brains: BTreeMap::from([("investment_projects".to_owned(), BrainId(7))]),
        scopes: BTreeMap::from([("project:investments".to_owned(), ScopeId(11))]),
        scope_bitmaps: BTreeMap::from([(ScopeId(11), BitmapHandle(100))]),
        status_bitmaps: BTreeMap::from([(StatusId(1), BitmapHandle(200))]),
    };
    let view = AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(7)]),
        readable_scopes: BTreeSet::from([ScopeId(11)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 16_000,
        default_context_budget_tokens: 4_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(99)),
    };
    let plan = Binder::new(&catalog, &view).bind_retrieve(&raw).unwrap();
    let provider = MockBitmapProvider {
        bitmaps: BTreeMap::from([
            (BitmapHandle(100), set(&[2, 3, 5])),
            (BitmapHandle(200), set(&[3, 4, 5])),
        ]),
        agent_allowed: set(&[1, 2, 3, 4]),
        live: set(&[2, 3, 4, 5]),
        universe: set(&[1, 2, 3, 4, 5]),
    };

    let candidates_0 = eval_bitmap_program(&plan.bitmap_program, &provider).unwrap();
    assert_eq!(candidates_0, set(&[3]));
}
