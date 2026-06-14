use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BitmapProgram, BoundPlan, BoundRetrievePlan, ContextPolicy,
    QualityThresholds, RetrievalMode, ScopeId, Q16_ZERO,
};
use cortex_core::CellDescriptor;

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
        quality_thresholds: QualityThresholds::default(),
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

#[test]
fn all_read_surfaces_rewrite_to_policy_complete_plans() {
    let view = view();
    let rewrite = PolicyRewrite::new(&view);
    for surface in READ_SURFACES {
        let logical = LogicalPlan::for_read_surface(*surface, BrainId(7));
        let rewritten = rewrite.rewrite_read_surface(*surface, BrainId(7));

        assert!(
            !logical.all_scans_have_permission_predicate(),
            "raw {surface:?} surface plan should expose an uncovered scan before rewrite"
        );
        assert!(
            rewritten.all_scans_have_permission_predicate(),
            "rewritten {surface:?} surface plan must cover every scan"
        );
        assert!(rewritten.to_report().policy_complete);
        assert!(rewritten.to_report().nodes.iter().any(|node| {
            node.kind == "scan"
                && node.detail.contains(surface.as_str())
                && node.permission_predicate.as_deref() == Some("agent_allowed")
        }));
    }
}

#[test]
fn descriptor_read_policy_uses_durable_descriptor_scope() {
    let view = view();
    let allowed = CellDescriptor {
        scope: "project:alpha".to_owned(),
        ..CellDescriptor::default()
    };
    let denied = CellDescriptor {
        scope: "project:beta".to_owned(),
        ..CellDescriptor::default()
    };

    assert!(PolicyRewrite::new(&view).can_read_descriptor(&allowed));
    assert!(!PolicyRewrite::new(&view).can_read_descriptor(&denied));
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(7)]),
        readable_scopes: BTreeSet::from([ScopeId(11), crate::query::scope_id("project:alpha")]),
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
