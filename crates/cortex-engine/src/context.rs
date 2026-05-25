use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::EngineResult;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextPackOptions {
    pub token_budget_tokens: u32,
    pub require_citations: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pub cells: Vec<ContextPackCell>,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub anomalies: Vec<ContextPackAnomaly>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackAnomaly {
    pub cell_id: Option<CellId>,
    pub code: &'static str,
    pub message: String,
}

impl Database {
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        let budget = effective_budget(view, options.token_budget_tokens);
        let citations_required = options.require_citations || view.require_citations_by_default;
        let cells = self.retrieve_aql(aql, view)?;
        Ok(ContextPack::from_retrieved(
            cells,
            budget,
            citations_required,
        ))
    }
}

impl ContextPack {
    pub fn from_retrieved(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
    ) -> Self {
        let mut pack_cells = Vec::new();
        let mut estimated_tokens = 0u32;
        let mut truncated = false;
        let mut anomalies = Vec::new();

        for cell in cells {
            let cell_tokens = estimate_tokens(&cell.payload);
            let would_exceed = !pack_cells.is_empty()
                && estimated_tokens.saturating_add(cell_tokens) > token_budget_tokens;
            if would_exceed {
                truncated = true;
                break;
            }
            if pack_cells.is_empty() && cell_tokens > token_budget_tokens {
                truncated = true;
            }

            let citation = extract_citation(&cell.payload);
            if citations_required && citation.is_none() {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: "missing_citation",
                    message: "selected cell does not include source= or citation=".to_owned(),
                });
            }
            estimated_tokens = estimated_tokens.saturating_add(cell_tokens);
            pack_cells.push(ContextPackCell {
                cell_id: cell.cell_id,
                payload: cell.payload,
                estimated_tokens: cell_tokens,
                citation,
            });
        }

        Self {
            cells: pack_cells,
            token_budget_tokens,
            estimated_tokens,
            truncated,
            citations_required,
            anomalies,
        }
    }
}

pub fn estimate_tokens(payload: &[u8]) -> u32 {
    if payload.is_empty() {
        return 0;
    }
    let bytes = match u32::try_from(payload.len()) {
        Ok(value) => value,
        Err(_) => return u32::MAX,
    };
    bytes.saturating_add(3) / 4
}

fn effective_budget(view: &AgentView, requested: u32) -> u32 {
    let budget = if requested == 0 {
        view.default_context_budget_tokens
    } else {
        requested
    };
    view.effective_budget(budget)
}

fn extract_citation(payload: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(payload);
    text.lines().find_map(|line| {
        line.strip_prefix("source=")
            .or_else(|| line.strip_prefix("citation="))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}
