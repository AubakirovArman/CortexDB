use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    optimize_bitmap_ops, parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement, BindDiagnostic,
    BindError, Binder, BitmapHandle, BitmapOp, BitmapProgram, BrainId, CellTypeId, MemoryType,
    PolicyError, RetrievalMode, ScopeId, StatusId, Q16_ZERO,
};

#[derive(Default)]
struct Catalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<(BrainId, String), ScopeId>,
    scope_bitmaps: BTreeMap<(BrainId, ScopeId), BitmapHandle>,
    status_bitmaps: BTreeMap<(BrainId, StatusId), BitmapHandle>,
    cell_type_bitmaps: BTreeMap<(BrainId, CellTypeId), BitmapHandle>,
    memory_type_bitmaps: BTreeMap<(BrainId, MemoryType), BitmapHandle>,
    estimates: BTreeMap<BitmapHandle, u64>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, brain: BrainId, name: &str) -> Option<ScopeId> {
        self.scopes.get(&(brain, name.to_owned())).copied()
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        match status {
            "ready" => Some(StatusId(1)),
            "verified" => Some(StatusId(2)),
            _ => None,
        }
    }

    fn resolve_cell_type(&self, _brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        match cell_type {
            "fact" => Some(CellTypeId(1)),
            "document_block" => Some(CellTypeId(2)),
            _ => None,
        }
    }

    fn scope_bitmap(&self, brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        self.scope_bitmaps.get(&(brain, scope)).copied()
    }

    fn status_bitmap(&self, brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        self.status_bitmaps.get(&(brain, status)).copied()
    }

    fn cell_type_bitmap(&self, brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        self.cell_type_bitmaps.get(&(brain, cell_type)).copied()
    }

    fn memory_type_bitmap(&self, brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        self.memory_type_bitmaps.get(&(brain, memory_type)).copied()
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(field, "space" | "status" | "type" | "memory_type")
    }

    fn bitmap_estimated_cardinality(&self, _brain: BrainId, handle: BitmapHandle) -> Option<u64> {
        self.estimates.get(&handle).copied()
    }
}

fn catalog() -> Catalog {
    Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        scopes: BTreeMap::from([
            ((BrainId(1), "allowed".to_owned()), ScopeId(10)),
            ((BrainId(1), "secret".to_owned()), ScopeId(11)),
            ((BrainId(2), "allowed".to_owned()), ScopeId(20)),
        ]),
        scope_bitmaps: BTreeMap::from([
            ((BrainId(1), ScopeId(10)), BitmapHandle(100)),
            ((BrainId(1), ScopeId(11)), BitmapHandle(101)),
            ((BrainId(2), ScopeId(20)), BitmapHandle(102)),
        ]),
        status_bitmaps: BTreeMap::from([
            ((BrainId(1), StatusId(1)), BitmapHandle(200)),
            ((BrainId(1), StatusId(2)), BitmapHandle(201)),
        ]),
        cell_type_bitmaps: BTreeMap::from([
            ((BrainId(1), CellTypeId(1)), BitmapHandle(300)),
            ((BrainId(1), CellTypeId(2)), BitmapHandle(301)),
        ]),
        memory_type_bitmaps: BTreeMap::from([(
            (BrainId(1), MemoryType::Decision),
            BitmapHandle(400),
        )]),
        estimates: BTreeMap::from([
            (BitmapHandle(100), 1_000),
            (BitmapHandle(200), 100),
            (BitmapHandle(201), 50),
        ]),
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("dev-agent".to_owned()),
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
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

fn retrieve(query: &str) -> Box<cortex_aql::RawRetrieveContext<'_>> {
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    raw
}

#[test]
fn or_with_unreadable_scope_fails_closed() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
WHERE space = allowed OR space = secret;"#,
    );
    let error = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap_err();
    assert_eq!(
        error,
        BindError::PolicyDenied(PolicyError::ScopeNotReadable)
    );
}

#[test]
fn unknown_scope_safe_diagnostic_hides_name() {
    let diagnostic = BindDiagnostic::from_error(&BindError::UnknownScope);
    let safe = diagnostic.safe_export();
    assert_eq!(safe.message, "requested scope is unavailable");
    assert!(safe.internal_detail.is_none());
}

#[test]
fn status_in_list_compiles_to_or_bytecode() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain
WHERE status IN ["ready", "verified"];"#,
    );
    let plan = Binder::new(&catalog(), &view())
        .bind_retrieve(&raw)
        .unwrap();
    assert!(plan.bitmap_program.ops.contains(&BitmapOp::Or));
    assert!(plan
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(200))));
    assert!(plan
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(201))));
}

#[test]
fn type_and_memory_type_filters_compile() {
    let type_raw = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain WHERE type = fact;"#);
    let memory_raw =
        retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain WHERE memory_type = decision;"#);
    let catalog = catalog();
    let view = view();
    let binder = Binder::new(&catalog, &view);
    assert!(binder
        .bind_retrieve(&type_raw)
        .unwrap()
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(300))));
    assert!(binder
        .bind_retrieve(&memory_raw)
        .unwrap()
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(400))));
}

#[test]
fn bitmap_program_validate_cost_and_debug() {
    let program = BitmapProgram {
        ops: vec![
            BitmapOp::PushAgentAllowed,
            BitmapOp::PushLive,
            BitmapOp::And,
        ],
        max_stack_depth: 2,
    };
    program.validate().unwrap();
    assert_eq!(program.estimated_cost(), 4);
    assert!(program.debug_bytecode().contains("0002: And"));
}

#[test]
fn optimizer_removes_double_not_and_duplicate_and() {
    assert_eq!(
        optimize_bitmap_ops(vec![
            BitmapOp::Push(BitmapHandle(1)),
            BitmapOp::Not,
            BitmapOp::Not,
        ]),
        vec![BitmapOp::Push(BitmapHandle(1))]
    );
    assert_eq!(
        optimize_bitmap_ops(vec![
            BitmapOp::Push(BitmapHandle(1)),
            BitmapOp::Push(BitmapHandle(1)),
            BitmapOp::And,
        ]),
        vec![BitmapOp::Push(BitmapHandle(1))]
    );
}
