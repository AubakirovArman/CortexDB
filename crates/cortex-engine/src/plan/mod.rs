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
    include!("tests.rs");
}
