use super::{ContextPack, ContextPackAnomaly, ContextPackCell};
use crate::query::metadata::SourceRef;

use text::{markdown_fence_for, option_or_null, push_line, trim_final_newline};

mod json_export;
mod text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPackExportFormat {
    Json,
    Prompt,
    Markdown,
}

impl ContextPack {
    pub fn export(&self, format: ContextPackExportFormat) -> String {
        match format {
            ContextPackExportFormat::Json => self.to_json(),
            ContextPackExportFormat::Prompt => self.to_agent_prompt(),
            ContextPackExportFormat::Markdown => self.to_markdown(),
        }
    }

    pub fn to_json(&self) -> String {
        json_export::to_json(self)
    }

    pub fn to_agent_prompt(&self) -> String {
        let mut out = String::new();
        push_line(&mut out, "CortexDB ContextPack v1");
        push_line(&mut out, "");
        push_line(&mut out, "Use only the context cells below.");
        push_line(&mut out, "Preserve citations when answering.");
        push_line(
            &mut out,
            "Cite citation= or source_ref= values for factual claims.",
        );
        push_line(
            &mut out,
            "If the supplied context is insufficient or conflicting, say so.",
        );
        push_line(
            &mut out,
            "Do not resolve conflicting evidence silently; report the conflict with citations.",
        );
        push_line(&mut out, "");
        push_line(
            &mut out,
            &format!(
                "Budget: token_budget_tokens={} estimated_tokens={} truncated={} citations_required={}",
                self.token_budget_tokens,
                self.estimated_tokens,
                self.truncated,
                self.citations_required
            ),
        );
        push_line(
            &mut out,
            &format!(
                "Answerability: answerability_q16={}",
                self.answerability_q16
            ),
        );
        push_line(
            &mut out,
            &format!(
                "Conflict visibility: conflict_visibility_q16={} visible_conflict_count={}",
                self.conflict_visibility_q16, self.visible_conflict_count
            ),
        );
        push_line(&mut out, "");
        push_line(&mut out, "Context cells:");
        for (index, cell) in self.cells.iter().enumerate() {
            push_prompt_cell(&mut out, index + 1, cell);
        }
        if !self.anomalies.is_empty() {
            push_line(&mut out, "");
            push_line(&mut out, "Context anomalies:");
            for anomaly in &self.anomalies {
                push_prompt_anomaly(&mut out, anomaly);
            }
        }
        trim_final_newline(out)
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        push_line(&mut out, "# CortexDB ContextPack");
        push_line(&mut out, "");
        push_line(&mut out, "- schema_version: `context_pack.v1`");
        push_line(
            &mut out,
            &format!("- token_budget_tokens: `{}`", self.token_budget_tokens),
        );
        push_line(
            &mut out,
            &format!("- estimated_tokens: `{}`", self.estimated_tokens),
        );
        push_line(&mut out, &format!("- truncated: `{}`", self.truncated));
        push_line(
            &mut out,
            &format!("- citations_required: `{}`", self.citations_required),
        );
        push_line(
            &mut out,
            &format!("- answerability_q16: `{}`", self.answerability_q16),
        );
        push_line(
            &mut out,
            &format!(
                "- conflict_visibility_q16: `{}`",
                self.conflict_visibility_q16
            ),
        );
        push_line(
            &mut out,
            &format!(
                "- visible_conflict_count: `{}`",
                self.visible_conflict_count
            ),
        );
        push_line(&mut out, "");
        push_line(&mut out, "## Cells");
        if self.cells.is_empty() {
            push_line(&mut out, "");
            push_line(&mut out, "_No cells selected._");
        } else {
            for (index, cell) in self.cells.iter().enumerate() {
                push_markdown_cell(&mut out, index + 1, cell);
            }
        }
        push_line(&mut out, "");
        push_line(&mut out, "## Anomalies");
        if self.anomalies.is_empty() {
            push_line(&mut out, "");
            push_line(&mut out, "_No anomalies._");
        } else {
            for anomaly in &self.anomalies {
                push_markdown_anomaly(&mut out, anomaly);
            }
        }
        trim_final_newline(out)
    }
}

fn push_prompt_cell(out: &mut String, index: usize, cell: &ContextPackCell) {
    push_line(out, "");
    push_line(out, &format!("[{}] cell_id={}", index, cell.cell_id.0));
    push_line(out, &format!("estimated_tokens={}", cell.estimated_tokens));
    push_line(
        out,
        &format!("citation={}", option_or_null(cell.citation.as_deref())),
    );
    if let Some(source_ref) = &cell.metadata.source_ref {
        push_line(
            out,
            &format!("source_ref={}", source_ref_inline(source_ref)),
        );
    }
    if let Some(provenance) = &cell.provenance {
        push_line(
            out,
            &format!("provenance={}", provenance_inline(provenance)),
        );
    }
    if let Some(explain) = &cell.explain {
        push_line(out, &format!("why_selected={}", explain.why_selected));
        push_line(
            out,
            &format!("matched_terms={}", explain.matched_terms.join(",")),
        );
        push_line(
            out,
            &format!(
                "source_freshness={} q16={} bonus={}",
                explain.source_freshness_category.as_str(),
                explain.source_freshness_q16,
                explain.source_freshness_bonus
            ),
        );
    }
    push_line(out, "text:");
    push_line(out, &String::from_utf8_lossy(&cell.payload));
}

fn push_prompt_anomaly(out: &mut String, anomaly: &ContextPackAnomaly) {
    push_line(
        out,
        &format!(
            "- code={} cell_id={} message={} why_excluded={}",
            anomaly.code.as_str(),
            anomaly
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            anomaly.message,
            option_or_null(anomaly.why_excluded.as_deref())
        ),
    );
}

fn push_markdown_cell(out: &mut String, index: usize, cell: &ContextPackCell) {
    push_line(out, "");
    push_line(out, &format!("### Cell {}", index));
    push_line(out, "");
    push_line(out, &format!("- cell_id: `{}`", cell.cell_id.0));
    push_line(
        out,
        &format!("- estimated_tokens: `{}`", cell.estimated_tokens),
    );
    push_line(
        out,
        &format!("- citation: `{}`", option_or_null(cell.citation.as_deref())),
    );
    if let Some(source_ref) = &cell.metadata.source_ref {
        push_line(
            out,
            &format!("- source_ref: `{}`", source_ref_inline(source_ref)),
        );
    }
    if let Some(provenance) = &cell.provenance {
        push_line(
            out,
            &format!("- provenance: `{}`", provenance_inline(provenance)),
        );
    }
    if let Some(explain) = &cell.explain {
        push_line(out, &format!("- why_selected: {}", explain.why_selected));
        push_line(
            out,
            &format!(
                "- score: `{}` (base_bm25=`{}`, source_trust_bonus=`{}`, source_freshness_bonus=`{}`, redundancy_penalty=`{}`)",
                explain.score,
                explain.base_bm25,
                explain.source_trust_bonus,
                explain.source_freshness_bonus,
                explain.redundancy_penalty
            ),
        );
        push_line(
            out,
            &format!("- matched_terms: `{}`", explain.matched_terms.join(", ")),
        );
        push_line(
            out,
            &format!(
                "- source_trust: `{}` (`{}`)",
                explain.source_trust_category.as_str(),
                explain.source_trust_q16
            ),
        );
        push_line(
            out,
            &format!(
                "- source_freshness: `{}` (`{}`)",
                explain.source_freshness_category.as_str(),
                explain.source_freshness_q16
            ),
        );
    }
    push_line(out, "");
    let payload = String::from_utf8_lossy(&cell.payload);
    let fence = markdown_fence_for(&payload);
    push_line(out, &format!("{fence}text"));
    push_line(out, &payload);
    push_line(out, &fence);
}

fn push_markdown_anomaly(out: &mut String, anomaly: &ContextPackAnomaly) {
    push_line(out, "");
    push_line(
        out,
        &format!(
            "- code: `{}`; cell_id: `{}`; message: {}; why_excluded: `{}`",
            anomaly.code.as_str(),
            anomaly
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            anomaly.message,
            option_or_null(anomaly.why_excluded.as_deref())
        ),
    );
}

fn source_ref_inline(source_ref: &SourceRef) -> String {
    let mut parts = vec![format!("source_id={}", source_ref.source_id)];
    if let Some(source_url) = &source_ref.source_url {
        parts.push(format!("source_url={source_url}"));
    }
    if let Some(document_id) = &source_ref.document_id {
        parts.push(format!("document_id={document_id}"));
    }
    if let Some(page) = source_ref.page {
        parts.push(format!("page={page}"));
    }
    if let Some(row) = source_ref.row {
        parts.push(format!("row={row}"));
    }
    if let Some(cell_range) = &source_ref.cell_range {
        parts.push(format!("cell_range={cell_range}"));
    }
    if let Some(json_path) = &source_ref.json_path {
        parts.push(format!("json_path={json_path}"));
    }
    parts.push(format!("confidence_q16={}", source_ref.confidence_q16));
    parts.join(";")
}

fn provenance_inline(provenance: &super::ContextSpanProvenance) -> String {
    let mut parts = vec![
        format!("source_cell_id={}", provenance.source_cell_id.0),
        format!("source_byte_start={}", provenance.source_byte_start),
        format!("source_byte_end={}", provenance.source_byte_end),
        format!("source_line_start={}", provenance.source_line_start),
        format!("source_line_end={}", provenance.source_line_end),
    ];
    if let Some(source_ref) = &provenance.source_ref {
        parts.push(format!("source_ref={}", source_ref_inline(source_ref)));
    }
    parts.join(";")
}
