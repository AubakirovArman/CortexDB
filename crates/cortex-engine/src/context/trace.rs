use serde::Serialize;

use super::{ContextPack, ContextScoreComponent};
use crate::verification::{VerificationReport, VerificationStatus};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextPipelineTrace {
    pub schema_version: &'static str,
    pub total_duration_ms: Option<u64>,
    pub stages: Vec<ContextPipelineStageTrace>,
    pub cells: Vec<ContextPipelineCellTrace>,
    pub verification: Option<ContextPipelineVerificationTrace>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextPipelineStageTrace {
    pub name: String,
    pub duration_ms: Option<u64>,
    pub input_items: u64,
    pub output_items: u64,
    pub notes: Vec<String>,
}

impl ContextPipelineStageTrace {
    pub fn new(
        name: impl Into<String>,
        duration_ms: Option<u64>,
        input_items: u64,
        output_items: u64,
        notes: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            duration_ms,
            input_items,
            output_items,
            notes,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextPipelineCellTrace {
    pub cell_id: u64,
    pub packed_rank: u32,
    pub estimated_tokens: u32,
    pub score: Option<u32>,
    pub matched_terms: Vec<String>,
    pub score_components: Vec<ContextScoreComponentTrace>,
    pub why_selected: Option<String>,
    pub citation_present: bool,
    pub provenance_present: bool,
    pub access_decision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextScoreComponentTrace {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

impl From<&ContextScoreComponent> for ContextScoreComponentTrace {
    fn from(value: &ContextScoreComponent) -> Self {
        Self {
            name: value.name.clone(),
            value: value.value,
            contribution: value.contribution,
            reason: value.reason.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextPipelineVerificationTrace {
    pub fact: String,
    pub status: String,
    pub evidence_count: u64,
    pub contradicting_evidence_count: u64,
    pub guard_count: u64,
    pub numeric_conflict_count: u64,
    pub evidence_cell_ids: Vec<u64>,
    pub contradicting_cell_ids: Vec<u64>,
}

impl ContextPipelineTrace {
    pub fn from_pack(
        pack: &ContextPack,
        verification: Option<&VerificationReport>,
        stages: Vec<ContextPipelineStageTrace>,
        total_duration_ms: Option<u64>,
    ) -> Self {
        let cells = pack
            .cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let (score, matched_terms, score_components, why_selected) = cell
                    .explain
                    .as_ref()
                    .map(|explain| {
                        (
                            Some(explain.score),
                            explain.matched_terms.clone(),
                            explain
                                .score_components
                                .iter()
                                .map(ContextScoreComponentTrace::from)
                                .collect(),
                            Some(explain.why_selected.clone()),
                        )
                    })
                    .unwrap_or_else(|| (None, Vec::new(), Vec::new(), None));

                ContextPipelineCellTrace {
                    cell_id: cell.cell_id.0,
                    packed_rank: (index as u32) + 1,
                    estimated_tokens: cell.estimated_tokens,
                    score,
                    matched_terms,
                    score_components,
                    why_selected,
                    citation_present: cell.citation.is_some(),
                    provenance_present: cell.provenance.is_some(),
                    access_decision: cell
                        .access_decision
                        .as_ref()
                        .map(|decision| decision.decision.as_str().to_owned()),
                }
            })
            .collect();

        Self {
            schema_version: "context_pipeline_trace.v1",
            total_duration_ms,
            stages,
            cells,
            verification: verification.map(ContextPipelineVerificationTrace::from),
        }
    }
}

impl From<&VerificationReport> for ContextPipelineVerificationTrace {
    fn from(report: &VerificationReport) -> Self {
        Self {
            fact: report.fact.clone(),
            status: verification_status(report.status).to_owned(),
            evidence_count: report.evidence.len() as u64,
            contradicting_evidence_count: report.contradicting_evidence.len() as u64,
            guard_count: report.guards.len() as u64,
            numeric_conflict_count: report.numeric_conflicts.len() as u64,
            evidence_cell_ids: report.evidence.iter().map(|item| item.cell_id.0).collect(),
            contradicting_cell_ids: report
                .contradicting_evidence
                .iter()
                .map(|item| item.cell_id.0)
                .collect(),
        }
    }
}

fn verification_status(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    }
}

#[cfg(test)]
mod tests {
    use cortex_core::CellId;

    use super::*;
    use crate::context::{ContextPack, ContextPackCell};
    use crate::query::CellMetadata;

    #[test]
    fn pipeline_trace_summarizes_pack_and_verification() {
        let pack = ContextPack {
            cells: vec![ContextPackCell {
                cell_id: CellId(7),
                payload: b"source=doc-a\n\nbudget approved".to_vec(),
                metadata: CellMetadata::from_payload(b"source=doc-a\n\nbudget approved"),
                estimated_tokens: 12,
                citation: Some("doc-a".to_owned()),
                provenance: None,
                explain: None,
                access_decision: None,
            }],
            token_budget_tokens: 100,
            estimated_tokens: 12,
            truncated: false,
            citations_required: true,
            answerability_q16: 65_535,
            conflict_visibility_q16: 65_535,
            visible_conflict_count: 0,
            anomalies: Vec::new(),
        };
        let verification = VerificationReport {
            fact: "budget approved".to_owned(),
            status: VerificationStatus::Supported,
            confidence_q16: 65_535,
            evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            guards: Vec::new(),
            numeric_conflicts: Vec::new(),
        };

        let trace = ContextPipelineTrace::from_pack(
            &pack,
            Some(&verification),
            vec![ContextPipelineStageTrace::new(
                "pack",
                Some(1),
                1,
                1,
                Vec::new(),
            )],
            Some(1),
        );

        assert_eq!(trace.schema_version, "context_pipeline_trace.v1");
        assert_eq!(trace.cells[0].cell_id, 7);
        assert_eq!(trace.cells[0].packed_rank, 1);
        assert!(trace.cells[0].citation_present);
        assert_eq!(trace.verification.as_ref().unwrap().status, "supported");
    }
}
