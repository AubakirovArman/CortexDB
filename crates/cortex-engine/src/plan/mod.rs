use cortex_aql::{AgentView, BoundPlan, BrainId, RetrievalMode, RetrievalWeights};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalPlan {
    pub nodes: Vec<LogicalPlanNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogicalPlanNode {
    Scan {
        brain_id: BrainId,
        predicate: String,
        permission_predicate: Option<String>,
    },
    Filter {
        predicate: String,
    },
    Rank {
        mode: RetrievalMode,
        weights: RetrievalWeights,
    },
    Limit {
        candidate_limit: u32,
    },
    Budget {
        budget_tokens: u32,
    },
    Pack {
        require_citations: bool,
    },
    Verify {
        fact: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalPlanReport {
    pub nodes: Vec<LogicalPlanNodeReport>,
    pub policy_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalPlanNodeReport {
    pub id: usize,
    pub kind: String,
    pub detail: String,
    pub permission_predicate: Option<String>,
}

pub struct PolicyRewrite<'a> {
    view: &'a AgentView,
}

impl LogicalPlan {
    pub fn from_bound_plan(bound: &BoundPlan, where_expression: Option<&str>) -> Self {
        match bound {
            BoundPlan::Retrieve(plan) => Self {
                nodes: vec![
                    LogicalPlanNode::Scan {
                        brain_id: plan.brain_id,
                        predicate: "bitmap_candidates".to_owned(),
                        permission_predicate: None,
                    },
                    LogicalPlanNode::Filter {
                        predicate: where_expression.unwrap_or("true").to_owned(),
                    },
                    LogicalPlanNode::Rank {
                        mode: plan.mode,
                        weights: plan.weights.clone(),
                    },
                    LogicalPlanNode::Limit {
                        candidate_limit: plan.context_policy.candidate_limit,
                    },
                    LogicalPlanNode::Budget {
                        budget_tokens: plan.context_policy.budget_tokens,
                    },
                    LogicalPlanNode::Pack {
                        require_citations: plan.context_policy.require_citations,
                    },
                ],
            },
            BoundPlan::VerifyFact(plan) => Self {
                nodes: vec![
                    LogicalPlanNode::Scan {
                        brain_id: plan.brain_id,
                        predicate: "verification_candidates".to_owned(),
                        permission_predicate: None,
                    },
                    LogicalPlanNode::Verify {
                        fact: plan.fact.clone(),
                    },
                ],
            },
            BoundPlan::Remember(_) => Self { nodes: Vec::new() },
        }
    }

    pub fn all_scans_have_permission_predicate(&self) -> bool {
        self.nodes.iter().all(|node| match node {
            LogicalPlanNode::Scan {
                permission_predicate,
                ..
            } => permission_predicate.is_some(),
            _ => true,
        })
    }

    pub fn to_report(&self) -> LogicalPlanReport {
        LogicalPlanReport {
            nodes: self
                .nodes
                .iter()
                .enumerate()
                .map(|(id, node)| node.to_report(id))
                .collect(),
            policy_complete: self.all_scans_have_permission_predicate(),
        }
    }
}

impl<'a> PolicyRewrite<'a> {
    pub fn new(view: &'a AgentView) -> Self {
        Self { view }
    }

    pub fn rewrite(&self, plan: &LogicalPlan) -> LogicalPlan {
        LogicalPlan {
            nodes: plan
                .nodes
                .iter()
                .cloned()
                .map(|node| self.rewrite_node(node))
                .collect(),
        }
    }

    fn rewrite_node(&self, node: LogicalPlanNode) -> LogicalPlanNode {
        match node {
            LogicalPlanNode::Scan {
                brain_id,
                predicate,
                permission_predicate,
            } => LogicalPlanNode::Scan {
                brain_id,
                predicate,
                permission_predicate: permission_predicate
                    .or_else(|| Some("agent_allowed".to_owned())),
            },
            LogicalPlanNode::Limit { candidate_limit } => LogicalPlanNode::Limit {
                candidate_limit: self.view.effective_candidate_limit(candidate_limit),
            },
            LogicalPlanNode::Budget { budget_tokens } => LogicalPlanNode::Budget {
                budget_tokens: self.view.effective_budget(budget_tokens),
            },
            other => other,
        }
    }
}

impl LogicalPlanNode {
    fn to_report(&self, id: usize) -> LogicalPlanNodeReport {
        match self {
            LogicalPlanNode::Scan {
                brain_id,
                predicate,
                permission_predicate,
            } => LogicalPlanNodeReport {
                id,
                kind: "scan".to_owned(),
                detail: format!("brain_id={} predicate={predicate}", brain_id.0),
                permission_predicate: permission_predicate.clone(),
            },
            LogicalPlanNode::Filter { predicate } => LogicalPlanNodeReport {
                id,
                kind: "filter".to_owned(),
                detail: predicate.clone(),
                permission_predicate: None,
            },
            LogicalPlanNode::Rank { mode, weights } => LogicalPlanNodeReport {
                id,
                kind: "rank".to_owned(),
                detail: format!(
                    "mode={mode:?} weights=lexical:{} semantic:{} recency:{} trust:{}",
                    weights.lexical_q16,
                    weights.semantic_q16,
                    weights.recency_q16,
                    weights.trust_q16
                ),
                permission_predicate: None,
            },
            LogicalPlanNode::Limit { candidate_limit } => LogicalPlanNodeReport {
                id,
                kind: "limit".to_owned(),
                detail: format!("candidate_limit={candidate_limit}"),
                permission_predicate: None,
            },
            LogicalPlanNode::Budget { budget_tokens } => LogicalPlanNodeReport {
                id,
                kind: "budget".to_owned(),
                detail: format!("budget_tokens={budget_tokens}"),
                permission_predicate: None,
            },
            LogicalPlanNode::Pack { require_citations } => LogicalPlanNodeReport {
                id,
                kind: "pack".to_owned(),
                detail: format!("require_citations={require_citations}"),
                permission_predicate: None,
            },
            LogicalPlanNode::Verify { fact } => LogicalPlanNodeReport {
                id,
                kind: "verify".to_owned(),
                detail: fact.clone(),
                permission_predicate: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
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
}
