use cortex_aql::{AgentView, BoundPlan};

use super::VerificationReport;
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::cache::AqlStatementKind;

impl Database {
    /// Execute a `VERIFY FACT` AQL statement against stored evidence.
    ///
    /// See `tests/verification_tests.rs` for end-to-end examples.
    pub fn verify_fact_aql(&self, aql: &str, view: &AgentView) -> EngineResult<VerificationReport> {
        let cached = self.bind_verify_fact_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::VerifyFact {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::VerifyFact(plan) = cached.bound_plan else {
            return Err(EngineError::InvalidOperation);
        };
        self.execute_verify_fact_plan(*plan, view)
            .map(|execution| execution.report)
    }
}
