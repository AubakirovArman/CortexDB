use cortex_aql::{AgentView, BrainId, ScopeId};
use cortex_core::CellDescriptor;

use crate::query::scope_id;

use super::{LogicalPlan, LogicalPlanNode};

pub const POLICY_AGENT_ALLOWED_PREDICATE: &str = "agent_allowed";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadSurface {
    AqlRetrieve,
    AqlExplain,
    Search,
    SearchExplain,
    ContextPack,
    ContextTrace,
    CellGet,
    Verify,
    Graph,
    Memory,
    Feedback,
    Export,
}

pub const READ_SURFACES: &[ReadSurface] = &[
    ReadSurface::AqlRetrieve,
    ReadSurface::AqlExplain,
    ReadSurface::Search,
    ReadSurface::SearchExplain,
    ReadSurface::ContextPack,
    ReadSurface::ContextTrace,
    ReadSurface::CellGet,
    ReadSurface::Verify,
    ReadSurface::Graph,
    ReadSurface::Memory,
    ReadSurface::Feedback,
    ReadSurface::Export,
];

pub struct PolicyRewrite<'a> {
    view: &'a AgentView,
}

impl ReadSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AqlRetrieve => "aql_retrieve",
            Self::AqlExplain => "aql_explain",
            Self::Search => "search",
            Self::SearchExplain => "search_explain",
            Self::ContextPack => "context_pack",
            Self::ContextTrace => "context_trace",
            Self::CellGet => "cell_get",
            Self::Verify => "verify",
            Self::Graph => "graph",
            Self::Memory => "memory",
            Self::Feedback => "feedback",
            Self::Export => "export",
        }
    }

    fn scan_predicate(self) -> String {
        format!("{}_candidates", self.as_str())
    }
}

impl LogicalPlan {
    pub fn for_read_surface(surface: ReadSurface, brain_id: BrainId) -> Self {
        Self {
            nodes: vec![LogicalPlanNode::Scan {
                brain_id,
                predicate: surface.scan_predicate(),
                permission_predicate: None,
            }],
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

    pub fn rewrite_read_surface(&self, surface: ReadSurface, brain_id: BrainId) -> LogicalPlan {
        self.rewrite(&LogicalPlan::for_read_surface(surface, brain_id))
    }

    pub fn can_read_scope(&self, scope_id: ScopeId) -> bool {
        Self::allows_scope(self.view, scope_id)
    }

    pub fn can_read_descriptor(&self, descriptor: &CellDescriptor) -> bool {
        Self::allows_descriptor(self.view, descriptor)
    }

    pub fn readable_scopes(&self) -> impl Iterator<Item = ScopeId> + '_ {
        self.view.readable_scopes.iter().copied()
    }

    pub fn allows_scope(view: &AgentView, scope_id: ScopeId) -> bool {
        view.can_read_scope(scope_id)
    }

    pub fn allows_descriptor(view: &AgentView, descriptor: &CellDescriptor) -> bool {
        Self::allows_scope(view, scope_id(&descriptor.scope))
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
                    .or_else(|| Some(POLICY_AGENT_ALLOWED_PREDICATE.to_owned())),
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
