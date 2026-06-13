use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BitmapProgram, BoundPlan, BoundRetrievePlan, ContextPolicy,
    QualityThresholds, RetrievalMode, ScopeId, Q16_ZERO,
};

use super::*;

#[test]
fn policy_rewrite_adds_permission_predicates_to_all_scans() {
    let plan = LogicalPlan {
        nodes: vec![
            LogicalPlanNode::Scan {
                brain_id: BrainId(7),
                predicate: "bitmap_candidates".to_owned(),
                permission_predicate: None,
            },
            LogicalPlanNode::Limit {
                candidate_limit: 1_000,
            },
            LogicalPlanNode::Budget {
                budget_tokens: 1_000_000,
            },
        ],
    };

    let rewritten = PolicyRewrite::new(&view()).rewrite(&plan);

    assert!(!plan.all_scans_have_permission_predicate());
    assert!(rewritten.all_scans_have_permission_predicate());
    assert_eq!(
        rewritten.nodes.iter().find_map(|node| match node {
            LogicalPlanNode::Scan {
                permission_predicate,
                ..
            } => permission_predicate.as_deref(),
            _ => None,
        }),
        Some("agent_allowed")
    );
    assert!(rewritten.nodes.iter().any(|node| matches!(
        node,
        LogicalPlanNode::Limit {
            candidate_limit: 100
        }
    )));
    assert!(rewritten.nodes.iter().any(|node| matches!(
        node,
        LogicalPlanNode::Budget {
            budget_tokens: 8_000
        }
    )));
}

#[test]
fn bound_retrieve_plan_has_inspectable_logical_nodes_before_and_after_policy() {
    let bound = BoundPlan::Retrieve(Box::new(BoundRetrievePlan {
        brain_id: BrainId(7),
        task: "budget".to_owned(),
        mode: RetrievalMode::Balanced,
        bitmap_program: BitmapProgram {
            ops: Vec::new(),
            max_stack_depth: 0,
        },
        context_policy: ContextPolicy {
            budget_tokens: 2_000,
            candidate_limit: 25,
            require_citations: true,
        },
        quality_thresholds: QualityThresholds {
            min_confidence_q16: Q16_ZERO,
            min_source_trust_q16: Q16_ZERO,
            max_freshness_seconds: None,
        },
        weights: cortex_aql::default_weights(RetrievalMode::Balanced),
    }));

    let logical = LogicalPlan::from_bound_plan(&bound, Some("status = \"ready\""));
    let rewritten = PolicyRewrite::new(&view()).rewrite(&logical);

    assert_eq!(logical.nodes.len(), 6);
    assert!(!logical.all_scans_have_permission_predicate());
    assert!(rewritten.all_scans_have_permission_predicate());
    assert!(rewritten
        .to_report()
        .nodes
        .iter()
        .any(|node| node.kind == "filter" && node.detail == "status = \"ready\""));
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(7)]),
        readable_scopes: BTreeSet::from([ScopeId(11)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::new(),
        max_context_budget_tokens: 8_000,
        default_context_budget_tokens: 2_000,
        max_candidate_limit: 100,
        default_candidate_limit: 25,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
