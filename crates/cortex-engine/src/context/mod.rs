use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::{Database, RetrievedCell};
use crate::error::EngineResult;
use crate::query::CellMetadata;
use crate::search::tokenize;

pub mod dedup;
pub mod explain;

use dedup::{effective_redundancy_threshold, is_redundant, term_set, weighted_jaccard_q16};
use explain::{extract_query_terms, generate_selection_reason};

pub const DEFAULT_REDUNDANCY_THRESHOLD_Q16: u16 = 32_768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackOptions {
    pub token_budget_tokens: u32,
    pub require_citations: bool,
    pub reduce_redundancy: bool,
    pub redundancy_threshold_q16: u16,
}

impl Default for ContextPackOptions {
    fn default() -> Self {
        Self {
            token_budget_tokens: 0,
            require_citations: false,
            reduce_redundancy: false,
            redundancy_threshold_q16: DEFAULT_REDUNDANCY_THRESHOLD_Q16,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextScoreComponent {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextExplain {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub score_components: Vec<ContextScoreComponent>,
    pub base_bm25: u32,
    pub source_trust_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub explain: Option<ContextExplain>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPackAnomalyCode {
    RedundantCell,
    MissingCitation,
    TokenOverload,
    ScopeMismatch,
}

impl ContextPackAnomalyCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedundantCell => "redundant_cell",
            Self::MissingCitation => "missing_citation",
            Self::TokenOverload => "token_overload",
            Self::ScopeMismatch => "scope_mismatch",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackAnomaly {
    pub cell_id: Option<CellId>,
    pub code: ContextPackAnomalyCode,
    pub message: String,
    pub why_excluded: Option<String>,
}

impl Database {
    /// Compile a `RETRIEVE CONTEXT` AQL statement into a scored ContextPack.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # use cortex_aql::{AgentId, AgentView, BrainId, RetrievalMode, ScopeId};
    /// # use cortex_engine::context::ContextPackOptions;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// # db.put_cell(CellId(1), b"scope=project:investments\n\nSolar budget 1.2B".to_vec()).unwrap();
    /// let view = AgentView {
    ///     agent_id: AgentId(1),
    ///     label: None,
    ///     readable_brains: [BrainId(1)].into_iter().collect(),
    ///     readable_scopes: [ScopeId(841510221546309118)].into_iter().collect(),
    ///     writable_scopes: Default::default(),
    ///     allowed_modes: [RetrievalMode::Balanced].into_iter().collect(),
    ///     allowed_memory_types: Default::default(),
    ///     max_context_budget_tokens: 10000,
    ///     default_context_budget_tokens: 1000,
    ///     max_candidate_limit: 100,
    ///     default_candidate_limit: 10,
    ///     min_required_confidence_q16: Default::default(),
    ///     max_ttl_seconds: None,
    ///     allow_remember: false,
    ///     allow_verify_fact: false,
    ///     allow_audit_mode: false,
    ///     require_citations_by_default: false,
    ///     private_scope: None,
    /// };
    /// let pack = db.context_pack_from_aql(
    ///     r#"RETRIEVE CONTEXT FOR TASK "budgets" IN BRAIN default BUDGET 500 TOKENS;"#,
    ///     &view,
    ///     ContextPackOptions {
    ///         token_budget_tokens: 500,
    ///         ..ContextPackOptions::default()
    ///     },
    /// ).unwrap();
    /// assert_eq!(pack.token_budget_tokens, 500);
    /// ```
    pub fn context_pack_from_aql(
        &self,
        aql: &str,
        view: &AgentView,
        options: ContextPackOptions,
    ) -> EngineResult<ContextPack> {
        let budget = effective_budget(view, options.token_budget_tokens);
        let citations_required = options.require_citations || view.require_citations_by_default;
        let cells = order_by_feedback(self.retrieve_aql(aql, view)?, &self.feedback_scores());
        Ok(ContextPack::from_retrieved_with_options(
            cells,
            budget,
            citations_required,
            &options,
            aql,
        ))
    }
}

impl ContextPack {
    pub fn from_retrieved(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
    ) -> Self {
        Self::from_retrieved_with_options(
            cells,
            token_budget_tokens,
            citations_required,
            &ContextPackOptions::default(),
            "",
        )
    }

    pub fn from_retrieved_with_options(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
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
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::TokenOverload,
                    message: "candidate would exceed the remaining token budget".to_owned(),
                    why_excluded: Some(
                        "excluded because estimated_tokens would exceed token_budget_tokens"
                            .to_owned(),
                    ),
                });
                break;
            }
            if pack_cells.is_empty() && cell_tokens > token_budget_tokens {
                truncated = true;
            }

            if options.reduce_redundancy
                && is_redundant(
                    &cell.payload,
                    &pack_cells,
                    effective_redundancy_threshold(options.redundancy_threshold_q16),
                )
            {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::RedundantCell,
                    message: "selected cell is redundant with an earlier packed cell".to_owned(),
                    why_excluded: Some(
                        "excluded because reduce_redundancy is enabled and similarity exceeds the configured threshold"
                            .to_owned(),
                    ),
                });
                continue;
            }

            let citation = extract_citation(&cell.payload);
            if citations_required && citation.is_none() {
                anomalies.push(ContextPackAnomaly {
                    cell_id: Some(cell.cell_id),
                    code: ContextPackAnomalyCode::MissingCitation,
                    message: "selected cell does not include source= or citation=".to_owned(),
                    why_excluded: None,
                });
            }

            let query_terms = extract_query_terms(query);
            let metadata = CellMetadata::from_payload(&cell.payload);
            let cell_body_terms = tokenize(&metadata.body_text)
                .into_iter()
                .collect::<BTreeSet<_>>();
            let matched: Vec<String> = query_terms
                .iter()
                .filter(|term| cell_body_terms.contains(*term))
                .cloned()
                .collect();

            let base_bm25 = (matched.len() as u32) * 10_000;
            let source_trust_bonus = metadata.source_trust_q16.unwrap_or(32768) as u32;

            let mut max_jaccard_similarity_q16 = 0u32;
            for packed in &pack_cells {
                let jaccard =
                    weighted_jaccard_q16(&cell_body_terms, &term_set(&packed.payload)) as u32;
                if jaccard > max_jaccard_similarity_q16 {
                    max_jaccard_similarity_q16 = jaccard;
                }
            }
            let redundancy_penalty = (max_jaccard_similarity_q16 * 10_000) / 65536;

            let score = base_bm25
                .saturating_add(source_trust_bonus)
                .saturating_sub(redundancy_penalty);

            let why_selected =
                generate_selection_reason(score, base_bm25, source_trust_bonus, redundancy_penalty);
            let score_components =
                score_components(base_bm25, source_trust_bonus, redundancy_penalty);

            let explain = Some(ContextExplain {
                score,
                matched_terms: matched,
                why_selected,
                score_components,
                base_bm25,
                source_trust_bonus,
                redundancy_penalty,
            });

            estimated_tokens = estimated_tokens.saturating_add(cell_tokens);
            pack_cells.push(ContextPackCell {
                cell_id: cell.cell_id,
                payload: cell.payload,
                estimated_tokens: cell_tokens,
                citation,
                explain,
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

fn score_components(
    base_bm25: u32,
    source_trust_bonus: u32,
    redundancy_penalty: u32,
) -> Vec<ContextScoreComponent> {
    vec![
        ContextScoreComponent {
            name: "base_bm25".to_owned(),
            value: base_bm25,
            contribution: i32::try_from(base_bm25).unwrap_or(i32::MAX),
            reason: "keyword overlap between query terms and cell body".to_owned(),
        },
        ContextScoreComponent {
            name: "source_trust_bonus".to_owned(),
            value: source_trust_bonus,
            contribution: i32::try_from(source_trust_bonus).unwrap_or(i32::MAX),
            reason: "source_trust_q16 metadata or default provenance trust".to_owned(),
        },
        ContextScoreComponent {
            name: "redundancy_penalty".to_owned(),
            value: redundancy_penalty,
            contribution: -i32::try_from(redundancy_penalty).unwrap_or(i32::MAX),
            reason: "weighted Jaccard overlap with already packed cells".to_owned(),
        },
    ]
}

fn order_by_feedback(
    cells: Vec<RetrievedCell>,
    feedback_scores: &BTreeMap<CellId, i32>,
) -> Vec<RetrievedCell> {
    let mut indexed = cells.into_iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(index, cell)| {
        (
            Reverse(*feedback_scores.get(&cell.cell_id).unwrap_or(&0)),
            *index,
        )
    });
    indexed.into_iter().map(|(_, cell)| cell).collect()
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
    CellMetadata::from_payload(payload)
        .citation()
        .map(str::to_owned)
}
