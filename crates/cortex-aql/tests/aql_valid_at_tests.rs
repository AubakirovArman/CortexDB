use std::collections::BTreeSet;

use cortex_aql::{
    parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement, BindError, Binder, BitmapHandle,
    BrainId, CellTypeId, MemoryType, Requirement, RetrievalMode, ScopeId, StatusId, Q16_ZERO,
};

#[derive(Default)]
struct Catalog;

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        (name == "brain").then_some(BrainId(1))
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        (name == "project:investments").then_some(ScopeId(10))
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        (status == "ready").then_some(StatusId(1))
    }

    fn resolve_cell_type(&self, _brain: BrainId, _cell_type: &str) -> Option<CellTypeId> {
        None
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        (scope == ScopeId(10)).then_some(BitmapHandle(100))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        (status == StatusId(1)).then_some(BitmapHandle(200))
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

#[test]
fn parse_require_valid_at() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE valid at "2026-01-01";"#,
    );
    assert_eq!(raw.requirements.len(), 1);
    let Requirement::ValidAt(date) = &raw.requirements[0].node else {
        panic!("expected valid_at requirement");
    };
    assert_eq!(date.value, "2026-01-01");
}

#[test]
fn require_valid_at_affects_quality_thresholds() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE valid at "2026-01-01";"#,
    );
    let plan = Binder::new(&Catalog, &view()).bind_retrieve(&raw).unwrap();
    assert_eq!(
        plan.quality_thresholds.valid_at.as_deref(),
        Some("2026-01-01")
    );
}

#[test]
fn require_valid_at_rejects_invalid_dates() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
REQUIRE valid at "2026-02-30";"#,
    );
    let error = Binder::new(&Catalog, &view())
        .bind_retrieve(&raw)
        .unwrap_err();
    assert_eq!(error, BindError::InvalidDate);
}

fn retrieve(query: &str) -> Box<cortex_aql::RawRetrieveContext<'_>> {
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    raw
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([ScopeId(10)]),
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
