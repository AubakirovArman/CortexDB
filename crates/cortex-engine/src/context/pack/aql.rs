use cortex_aql::{AgentView, BoundPlan};

use crate::context::{ContextPack, ContextPackOptions, ContextPackWithTools};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::exec::PackOp;
use crate::feedback::current_unix_seconds;
use crate::query::{cache::AqlStatementKind, EngineAqlProvider};

impl Database {
    /// Compile a `RETRIEVE CONTEXT` AQL statement into a scored ContextPack.
    ///
    /// The same bound retrieve plan drives bitmap filtering, AQL `LIMIT`,
    /// `BUDGET`, `REQUIRE` gates, and the ContextPack citation policy.
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let provider = EngineAqlProvider::new(index, view);
        let requested_budget = if options.token_budget_tokens == 0 {
            plan.context_policy.budget_tokens
        } else {
            options.token_budget_tokens
        };
        let budget = view.effective_budget(requested_budget);
        let citations_required = options.require_citations || plan.context_policy.require_citations;
        let cells = self.retrieve_cells(&plan, &provider)?;
        let feedback_scores = self.feedback_scores_for_cells_at(
            cells.iter().map(|cell| cell.cell_id),
            current_unix_seconds(),
        );
        let cells = crate::context::scoring::order_by_feedback(cells, &feedback_scores);
        Ok(PackOp::execute(
            cells,
            budget,
            citations_required,
            &options,
            aql,
            &feedback_scores,
            Some(view),
        )
        .pack)
    }

    pub fn context_pack_with_tool_recommendations_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
        tool_limit: usize,
    ) -> EngineResult<ContextPackWithTools> {
        let pack = self.context_pack_from_aql(aql, view, options)?;
        let (cached, _) = self.bind_aql_cached(aql, view)?;
        let BoundPlan::Retrieve(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        let tool_recommendations = self.recommend_tools_for_task(view, &plan.task, tool_limit);
        Ok(ContextPackWithTools {
            pack,
            tool_recommendations,
        })
    }
}
