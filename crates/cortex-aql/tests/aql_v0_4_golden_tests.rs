use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    decimal_to_q16, parse_aql, parse_aql_diagnostic, AgentId, AgentView, AqlCatalog,
    AqlParseErrorKind, AqlStatement, BindError, Binder, BitmapHandle, BoundPlan, BrainId,
    CellTypeId, Condition, Literal, MemoryType, Requirement, RetrievalMode, ScopeId, StatusId,
    Q16_ZERO,
};

#[derive(Default)]
struct GoldenCatalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<(BrainId, String), ScopeId>,
    write_scopes: BTreeMap<String, ScopeId>,
}

impl AqlCatalog for GoldenCatalog {
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

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        Some(BitmapHandle(1_000 + scope.0))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        Some(BitmapHandle(2_000 + status.0))
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        Some(BitmapHandle(3_000 + cell_type.0))
    }

    fn memory_type_bitmap(&self, _brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        Some(BitmapHandle(
            4_000
                + match memory_type {
                    MemoryType::Decision => 1,
                    MemoryType::Preference => 2,
                    MemoryType::WorkflowResult => 3,
                    MemoryType::ErrorLog => 4,
                    MemoryType::Observation => 5,
                },
        ))
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(
            field,
            "space" | "scope" | "status" | "type" | "cell_type" | "memory_type"
        )
    }
}

fn catalog() -> GoldenCatalog {
    GoldenCatalog {
        brains: BTreeMap::from([("investment_projects".to_owned(), BrainId(7))]),
        scopes: BTreeMap::from([
            ((BrainId(7), "project:investments".to_owned()), ScopeId(11)),
            ((BrainId(7), "tenant:secret".to_owned()), ScopeId(12)),
        ]),
        write_scopes: BTreeMap::from([("project:investments".to_owned(), ScopeId(11))]),
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(7)]),
        readable_scopes: BTreeSet::from([ScopeId(11)]),
        writable_scopes: BTreeSet::from([ScopeId(11)]),
        allowed_modes: BTreeSet::from([
            RetrievalMode::Fast,
            RetrievalMode::Balanced,
            RetrievalMode::Semantic,
        ]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision, MemoryType::Observation]),
        max_context_budget_tokens: 8_000,
        default_context_budget_tokens: 2_000,
        max_candidate_limit: 100,
        default_candidate_limit: 25,
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
fn aql_v0_4_parse_retrieve_shape_is_stable() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT
FOR TASK "compare budgets"
IN BRAIN investment_projects
USING MODE semantic
BUDGET 12000 TOKENS
LIMIT 500 CANDIDATES
WHERE space = project:investments AND status IN ["ready", verified]
REQUIRE citations, confidence >= 0.80, source_trust >= 0.90, freshness <= 86400 SECONDS;"#,
    )
    .unwrap();

    let AqlStatement::RetrieveContext(raw) = statement else {
        panic!("expected retrieve");
    };
    assert_eq!(raw.task.node.value, "compare budgets");
    assert_eq!(raw.brain.node.value, "investment_projects");
    assert_eq!(raw.mode.unwrap().node, RetrievalMode::Semantic);
    assert_eq!(raw.budget_tokens.unwrap().node, 12_000);
    assert_eq!(raw.candidate_limit.unwrap().node, 500);
    assert_eq!(raw.requirements.len(), 4);
    assert!(matches!(
        raw.requirements[0].node,
        Requirement::RequireCitations
    ));
    assert!(matches!(
        raw.requirements[1].node,
        Requirement::MinConfidence(_)
    ));
    assert!(matches!(
        raw.requirements[2].node,
        Requirement::SourceTrust(_)
    ));
    assert!(matches!(
        raw.requirements[3].node,
        Requirement::Freshness(_)
    ));

    let Condition::And(lhs, rhs) = raw.where_clause.unwrap().node else {
        panic!("expected AND root");
    };
    assert!(matches!(lhs.node, Condition::Predicate { .. }));
    let Condition::Predicate {
        literal,
        comparator,
        ..
    } = rhs.node
    else {
        panic!("expected status predicate");
    };
    assert_eq!(format!("{:?}", comparator.node), "In");
    let Literal::List(values) = literal.node else {
        panic!("expected IN list");
    };
    assert_eq!(values.len(), 2);
}

#[test]
fn aql_v0_4_bind_retrieve_plan_golden_bytecode() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "compare budgets" IN BRAIN investment_projects
USING MODE semantic BUDGET 12000 TOKENS LIMIT 500 CANDIDATES
WHERE space = project:investments AND status IN ["ready", verified]
REQUIRE citations, confidence >= 0.80, source_trust >= 0.90, freshness <= 86400 SECONDS;"#,
    )
    .unwrap();
    let BoundPlan::Retrieve(plan) = Binder::new(&catalog(), &view())
        .bind_statement(&statement)
        .unwrap()
    else {
        panic!("expected retrieve plan");
    };

    assert_eq!(plan.brain_id, BrainId(7));
    assert_eq!(plan.mode, RetrievalMode::Semantic);
    assert_eq!(plan.context_policy.budget_tokens, 8_000);
    assert_eq!(plan.context_policy.candidate_limit, 100);
    assert!(plan.context_policy.require_citations);
    assert_eq!(
        plan.quality_thresholds.min_confidence_q16,
        decimal_to_q16(&cortex_aql::DecimalLiteral::borrowed("0.80")).unwrap()
    );
    assert_eq!(
        plan.quality_thresholds.min_source_trust_q16,
        decimal_to_q16(&cortex_aql::DecimalLiteral::borrowed("0.90")).unwrap()
    );
    assert_eq!(plan.quality_thresholds.max_freshness_seconds, Some(86400));
    assert_eq!(
        plan.bitmap_program.debug_bytecode(),
        "0000: PushAgentAllowed\n0001: PushLive\n0002: And\n0003: Push(BitmapHandle(1011))\n0004: Push(BitmapHandle(2001))\n0005: Push(BitmapHandle(2002))\n0006: Or\n0007: And\n0008: And"
    );
    plan.bitmap_program.validate().unwrap();
}

#[test]
fn aql_v0_4_explain_verify_and_remember_parse_contracts() {
    let explain =
        parse_aql(r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects;"#)
            .unwrap();
    assert!(matches!(explain, AqlStatement::Explain(_)));
    let BoundPlan::Retrieve(explain_plan) = Binder::new(&catalog(), &view())
        .bind_statement(&explain)
        .unwrap()
    else {
        panic!("expected explain retrieve to bind as retrieve");
    };
    assert_eq!(explain_plan.brain_id, BrainId(7));
    assert_eq!(explain_plan.task, "budget");

    let verify =
        parse_aql(r#"VERIFY FACT "Solar Plant budget is approved" IN BRAIN investment_projects;"#)
            .unwrap();
    let BoundPlan::VerifyFact(plan) = Binder::new(&catalog(), &view())
        .bind_statement(&verify)
        .unwrap()
    else {
        panic!("expected verify plan");
    };
    assert_eq!(plan.brain_id, BrainId(7));
    assert_eq!(plan.fact, "Solar Plant budget is approved");

    let remember = parse_aql(
        r#"REMEMBER "Use conservative budget assumptions" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;"#,
    )
    .unwrap();
    let BoundPlan::Remember(plan) = Binder::new(&catalog(), &view())
        .bind_statement(&remember)
        .unwrap()
    else {
        panic!("expected remember plan");
    };
    assert_eq!(plan.scope_id, ScopeId(11));
    assert_eq!(plan.scope_name, "project:investments");
    assert_eq!(plan.memory_type, MemoryType::Decision);
    assert_eq!(plan.ttl_seconds, Some(3_600));
}

#[test]
fn aql_v0_4_diagnostics_and_safe_bind_messages_are_stable() {
    let parse_error = parse_aql_diagnostic(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects USING MODE turbo;"#,
    )
    .unwrap_err();
    assert_eq!(parse_error.kind, AqlParseErrorKind::InvalidMode);
    assert_eq!(parse_error.span.line, 1);
    assert_eq!(parse_error.kind.safe_message(), "invalid retrieval mode");
    assert!(parse_error.message.contains("line 1, column"));

    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects WHERE space = tenant:missing;"#,
    )
    .unwrap();
    let error = Binder::new(&catalog(), &view())
        .bind_statement(&statement)
        .unwrap_err();
    assert_eq!(error.code(), "UnknownScope");
    assert_eq!(error.safe_message(), "requested scope is unavailable");

    let forbidden = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects WHERE space = project:investments OR space = tenant:secret;"#,
    )
    .unwrap();
    let forbidden_error = Binder::new(&catalog(), &view())
        .bind_statement(&forbidden)
        .unwrap_err();
    assert_eq!(forbidden_error.code(), "PolicyDenied");
    assert_eq!(
        forbidden_error.safe_message(),
        "requested scope is not readable"
    );

    let unknown_field = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects WHERE owner = "abc";"#,
    )
    .unwrap();
    let field_error = Binder::new(&catalog(), &view())
        .bind_statement(&unknown_field)
        .unwrap_err();
    assert_eq!(field_error.code(), "FieldNotFilterable");
    assert_eq!(field_error.safe_message(), "field is not filterable");
}

#[test]
fn aql_v0_4_unsupported_comparators_parse_but_do_not_bind() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN investment_projects WHERE status != "ready";"#,
    )
    .unwrap();
    let error = Binder::new(&catalog(), &view())
        .bind_statement(&statement)
        .unwrap_err();
    assert_eq!(error, BindError::UnsupportedComparator);
    assert_eq!(
        error.safe_message(),
        "comparator is not supported for this field"
    );
}
