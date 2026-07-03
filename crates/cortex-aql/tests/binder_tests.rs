use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    decimal_to_q16, default_weights, parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement,
    BindError, Binder, BitmapHandle, BitmapOp, BrainId, CellTypeId, DecimalLiteral, MemoryType,
    PolicyError, RetrievalMode, ScopeId, StatusId, Q16_ONE, Q16_ZERO,
};

#[derive(Default)]
struct Catalog {
    brains: BTreeMap<String, BrainId>,
    scopes: BTreeMap<String, ScopeId>,
    scope_bitmaps: BTreeMap<ScopeId, BitmapHandle>,
    status_bitmaps: BTreeMap<StatusId, BitmapHandle>,
    cell_type_bitmaps: BTreeMap<CellTypeId, BitmapHandle>,
    memory_type_bitmaps: BTreeMap<MemoryType, BitmapHandle>,
}

impl AqlCatalog for Catalog {
    fn resolve_brain(&self, name: &str) -> Option<BrainId> {
        self.brains.get(name).copied()
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        self.scopes.get(name).copied()
    }

    fn resolve_write_scope(&self, name: &str) -> Option<ScopeId> {
        self.scopes.get(name).copied()
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
        self.scope_bitmaps.get(&scope).copied()
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        self.status_bitmaps.get(&status).copied()
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        self.cell_type_bitmaps.get(&cell_type).copied()
    }

    fn memory_type_bitmap(&self, _brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        self.memory_type_bitmaps.get(&memory_type).copied()
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(
            field,
            "space" | "status" | "type" | "memory_type" | "cell_type"
        )
    }
}

fn catalog() -> Catalog {
    Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        scopes: BTreeMap::from([("scope-a".to_owned(), ScopeId(10))]),
        scope_bitmaps: BTreeMap::from([(ScopeId(10), BitmapHandle(100))]),
        status_bitmaps: BTreeMap::from([
            (StatusId(1), BitmapHandle(200)),
            (StatusId(2), BitmapHandle(201)),
        ]),
        cell_type_bitmaps: BTreeMap::from([
            (CellTypeId(1), BitmapHandle(300)),
            (CellTypeId(2), BitmapHandle(301)),
        ]),
        memory_type_bitmaps: BTreeMap::from([(MemoryType::Decision, BitmapHandle(400))]),
    }
}

fn view(readable_scopes: BTreeSet<ScopeId>, citations: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes,
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: citations,
        private_scope: Some(ScopeId(99)),
    }
}

fn retrieve(query: &str) -> cortex_aql::RawRetrieveContext<'_> {
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    *raw
}

#[test]
fn decimal_to_q16_085() {
    assert_eq!(
        decimal_to_q16(&DecimalLiteral::borrowed("0.85")).unwrap(),
        55_705
    );
}

#[test]
fn default_weights_sum_equals_q16_one() {
    for mode in [
        RetrievalMode::Fast,
        RetrievalMode::Balanced,
        RetrievalMode::Hybrid,
        RetrievalMode::Semantic,
        RetrievalMode::Audit,
    ] {
        assert_eq!(default_weights(mode).sum_q16(), u32::from(Q16_ONE));
    }
}

#[test]
fn fast_mode_preserves_required_citations() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING MODE fast BUDGET 100 TOKENS;"#,
    );
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&raw)
        .unwrap();
    assert!(plan.context_policy.require_citations);
}

#[test]
fn using_diversity_binds_the_lambda_and_is_absent_by_default() {
    // A5/A7.3: the clause parses and binds to a per-query Q16 lambda.
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING DIVERSITY 20000 LIMIT 5 CANDIDATES;"#,
    );
    assert_eq!(raw.diversity_lambda_q16.as_ref().map(|s| s.node), Some(20_000));
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.diversity_lambda_q16, Some(20_000));

    // Absent by default (byte-identical to before the clause existed).
    let plain = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#);
    assert_eq!(plain.diversity_lambda_q16, None);
    assert_eq!(plain.rerank_weight_q16, None);
    let plain_plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&plain)
        .unwrap();
    assert_eq!(plain_plan.diversity_lambda_q16, None);
    assert_eq!(plain_plan.rerank_weight_q16, None);
}

#[test]
fn using_rerank_binds_the_weight() {
    // A7.2: the two-stage dense rerank weight is a per-query AQL option.
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING RERANK 65535 LIMIT 5 CANDIDATES;"#,
    );
    assert_eq!(raw.rerank_weight_q16.as_ref().map(|s| s.node), Some(65_535));
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.rerank_weight_q16, Some(65_535));
}

#[test]
fn suppress_superseded_binds_and_is_off_by_default() {
    // A4.2: temporal supersession is a per-query AQL flag.
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain SUPPRESS SUPERSEDED LIMIT 5 CANDIDATES;"#,
    );
    assert!(raw.suppress_superseded);
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&raw)
        .unwrap();
    assert!(plan.suppress_superseded);

    let plain = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#);
    assert!(!plain.suppress_superseded);
}

#[test]
fn recency_window_binds_the_seconds() {
    // A4.1: the temporal recency window is a per-query AQL option.
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain RECENCY WINDOW 86400 LIMIT 5 CANDIDATES;"#,
    );
    assert_eq!(raw.recency_window_seconds.as_ref().map(|s| s.node), Some(86_400));
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), true))
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(plan.recency_window_seconds, Some(86_400));

    let plain = retrieve(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain;"#);
    assert_eq!(plain.recency_window_seconds, None);
}

#[test]
fn bind_where_scope_and_status_to_bitmap_program() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING MODE balanced BUDGET 100 TOKENS
WHERE space = "scope-a" AND status = "ready";"#,
    );
    let plan = Binder::new(&catalog(), &view(BTreeSet::from([ScopeId(10)]), false))
        .bind_retrieve(&raw)
        .unwrap();
    assert_eq!(
        plan.bitmap_program.ops,
        vec![
            BitmapOp::PushAgentAllowed,
            BitmapOp::PushLive,
            BitmapOp::And,
            BitmapOp::Push(BitmapHandle(100)),
            BitmapOp::Push(BitmapHandle(200)),
            BitmapOp::And,
            BitmapOp::And,
        ]
    );
}

#[test]
fn scope_not_readable_fails() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN brain USING MODE balanced BUDGET 100 TOKENS
WHERE space = "scope-a";"#,
    );
    let error = Binder::new(&catalog(), &view(BTreeSet::new(), false))
        .bind_retrieve(&raw)
        .unwrap_err();
    assert_eq!(
        error,
        BindError::PolicyDenied(PolicyError::ScopeNotReadable)
    );
}

#[test]
fn bind_verify_fact_success() {
    let AqlStatement::VerifyFact(raw) =
        parse_aql(r#"VERIFY FACT "budget is approved" IN BRAIN brain;"#).unwrap()
    else {
        panic!("expected verify fact");
    };
    let mut view = view(BTreeSet::from([ScopeId(10)]), false);
    view.allow_verify_fact = true;
    let plan = Binder::new(&catalog(), &view)
        .bind_verify_fact(&raw)
        .unwrap();
    assert_eq!(plan.brain_id, BrainId(1));
    assert_eq!(plan.fact, "budget is approved");
}

#[test]
fn bind_verify_fact_denied_uses_verify_error() {
    let AqlStatement::VerifyFact(raw) =
        parse_aql(r#"VERIFY FACT "budget is approved" IN BRAIN brain;"#).unwrap()
    else {
        panic!("expected verify fact");
    };
    let error = Binder::new(&catalog(), &view(BTreeSet::new(), false))
        .bind_verify_fact(&raw)
        .unwrap_err();
    assert_eq!(
        error,
        BindError::PolicyDenied(PolicyError::VerifyFactNotAllowed)
    );
}

#[test]
fn bind_remember_success_with_ttl() {
    let AqlStatement::Remember(raw) =
        parse_aql(r#"REMEMBER "ship the MVP" IN SCOPE scope-a AS TYPE decision TTL 60 SECONDS;"#)
            .unwrap()
    else {
        panic!("expected remember");
    };
    let mut view = view(BTreeSet::from([ScopeId(10)]), false);
    view.allow_remember = true;
    view.writable_scopes.insert(ScopeId(10));
    let plan = Binder::new(&catalog(), &view).bind_remember(&raw).unwrap();
    assert_eq!(plan.scope_id, ScopeId(10));
    assert_eq!(plan.memory_type, MemoryType::Decision);
    assert_eq!(plan.ttl_seconds, Some(60));
}

#[test]
fn bind_remember_ttl_too_long_denied() {
    let AqlStatement::Remember(raw) =
        parse_aql(r#"REMEMBER "ship the MVP" IN SCOPE scope-a AS TYPE decision TTL 7200 SECONDS;"#)
            .unwrap()
    else {
        panic!("expected remember");
    };
    let mut view = view(BTreeSet::from([ScopeId(10)]), false);
    view.allow_remember = true;
    view.writable_scopes.insert(ScopeId(10));
    let error = Binder::new(&catalog(), &view)
        .bind_remember(&raw)
        .unwrap_err();
    assert_eq!(error, BindError::PolicyDenied(PolicyError::TtlTooLong));
}
