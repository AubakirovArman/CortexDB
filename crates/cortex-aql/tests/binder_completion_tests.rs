use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    optimize_bitmap_ops, parse_aql, AgentId, AgentView, AqlCatalog, AqlParseErrorKind, BindError,
    Binder, BitmapHandle, BitmapOp, BoundPlan, BrainCatalog, BrainId, CellTypeCatalog, CellTypeId,
    MemoryType, PolicyError, RetrievalMode, ScopeCatalog, ScopeId, StatusCatalog, StatusId,
    Q16_ZERO,
};

#[derive(Default)]
struct Catalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<(BrainId, String), ScopeId>,
    write_scopes: BTreeMap<String, ScopeId>,
    scope_bitmaps: BTreeMap<(BrainId, ScopeId), BitmapHandle>,
    status_bitmaps: BTreeMap<(BrainId, StatusId), BitmapHandle>,
    cell_type_bitmaps: BTreeMap<(BrainId, CellTypeId), BitmapHandle>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, brain: BrainId, name: &str) -> Option<ScopeId> {
        self.scopes.get(&(brain, name.to_owned())).copied()
    }

    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId> {
        self.write_scopes.get(name).copied()
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        match status {
            "ready" => Some(StatusId(1)),
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

    fn memory_type_bitmap(
        &self,
        _brain: BrainId,
        _memory_type: MemoryType,
    ) -> Option<BitmapHandle> {
        None
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(field, "space" | "status" | "type" | "cell_type")
    }
}

fn catalog() -> Catalog {
    Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        scopes: BTreeMap::from([((BrainId(1), "allowed".to_owned()), ScopeId(10))]),
        write_scopes: BTreeMap::from([("allowed".to_owned(), ScopeId(10))]),
        scope_bitmaps: BTreeMap::from([((BrainId(1), ScopeId(10)), BitmapHandle(100))]),
        status_bitmaps: BTreeMap::from([((BrainId(1), StatusId(1)), BitmapHandle(200))]),
        cell_type_bitmaps: BTreeMap::from([
            ((BrainId(1), CellTypeId(1)), BitmapHandle(300)),
            ((BrainId(1), CellTypeId(2)), BitmapHandle(301)),
        ]),
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([ScopeId(10)]),
        writable_scopes: BTreeSet::from([ScopeId(10)]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[test]
fn split_catalog_traits_delegate_to_aql_catalog() {
    let catalog = catalog();
    assert_eq!(
        BrainCatalog::resolve_brain(&catalog, "brain"),
        Some(BrainId(1))
    );
    assert_eq!(
        ScopeCatalog::resolve_scope(&catalog, BrainId(1), "allowed"),
        Some(ScopeId(10))
    );
    assert_eq!(
        StatusCatalog::resolve_status(&catalog, BrainId(1), "ready"),
        Some(StatusId(1))
    );
    assert_eq!(
        CellTypeCatalog::resolve_cell_type(&catalog, BrainId(1), "fact"),
        Some(CellTypeId(1))
    );
}

#[test]
fn bind_statement_returns_bound_plan_variants() {
    let catalog = catalog();
    let view = view();
    let binder = Binder::new(&catalog, &view);

    let retrieve = parse_aql(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#).unwrap();
    assert!(matches!(
        binder.bind_statement(&retrieve).unwrap(),
        BoundPlan::Retrieve(_)
    ));

    let verify = parse_aql(r#"VERIFY FACT "x" IN BRAIN brain;"#).unwrap();
    assert!(matches!(
        binder.bind_statement(&verify).unwrap(),
        BoundPlan::VerifyFact(_)
    ));

    let remember =
        parse_aql(r#"REMEMBER "x" IN SCOPE allowed AS TYPE decision TTL 60 SECONDS;"#).unwrap();
    assert!(matches!(
        binder.bind_statement(&remember).unwrap(),
        BoundPlan::Remember(_)
    ));
}

#[test]
fn bind_statement_preserves_policy_errors() {
    let catalog = catalog();
    let mut view = view();
    view.allow_verify_fact = false;
    let statement = parse_aql(r#"VERIFY FACT "x" IN BRAIN brain;"#).unwrap();
    let error = Binder::new(&catalog, &view)
        .bind_statement(&statement)
        .unwrap_err();
    assert_eq!(
        error,
        BindError::PolicyDenied(PolicyError::VerifyFactNotAllowed)
    );
}

#[test]
fn type_in_list_compiles_to_or_bytecode() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain WHERE type IN ["fact", "document_block"];"#,
    )
    .unwrap();
    let BoundPlan::Retrieve(plan) = Binder::new(&catalog(), &view())
        .bind_statement(&statement)
        .unwrap()
    else {
        panic!("expected retrieve plan");
    };
    assert!(plan.bitmap_program.ops.contains(&BitmapOp::Or));
    assert!(plan
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(300))));
    assert!(plan
        .bitmap_program
        .ops
        .contains(&BitmapOp::Push(BitmapHandle(301))));
}

#[test]
fn bitmap_program_explain_and_or_optimizer_are_available() {
    let optimized = optimize_bitmap_ops(vec![
        BitmapOp::Push(BitmapHandle(1)),
        BitmapOp::Push(BitmapHandle(1)),
        BitmapOp::Or,
    ]);
    assert_eq!(optimized, vec![BitmapOp::Push(BitmapHandle(1))]);

    let statement = parse_aql(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#).unwrap();
    let BoundPlan::Retrieve(plan) = Binder::new(&catalog(), &view())
        .bind_statement(&statement)
        .unwrap()
    else {
        panic!("expected retrieve plan");
    };
    let explain = plan.bitmap_program.explain();
    assert!(explain.contains("BitmapProgram(max_stack_depth=2)"));
    assert!(explain.contains("0002: And"));
}

#[test]
fn parse_error_has_diagnostic_type_and_pretty_display() {
    let error =
        parse_aql(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING MODE turbo;"#).unwrap_err();
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.kind, AqlParseErrorKind::InvalidMode);
    assert_eq!(diagnostic.to_string(), error.to_string());
    assert!(diagnostic.to_string().contains("line 1, column"));
}
