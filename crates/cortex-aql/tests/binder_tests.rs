use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{
    decimal_to_q16, default_weights, parse_aql, AgentId, AgentView, AqlCatalog, AqlStatement,
    BindError, Binder, BitmapHandle, BitmapOp, BrainId, DecimalLiteral, MemoryType, PolicyError,
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
        matches!(field, "space" | "status" | "memory_type" | "cell_type")
    }
}

fn catalog() -> Catalog {
    Catalog {
        brains: BTreeMap::from([("brain".to_owned(), BrainId(1))]),
        scopes: BTreeMap::from([("scope-a".to_owned(), ScopeId(10))]),
        scope_bitmaps: BTreeMap::from([(ScopeId(10), BitmapHandle(100))]),
        status_bitmaps: BTreeMap::from([("ready".to_owned(), BitmapHandle(200))]),
    }
}

fn view(readable_scopes: BTreeSet<ScopeId>, citations: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
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
        max_ttl_seconds: 3_600,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: citations,
        private_scope: ScopeId(99),
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
