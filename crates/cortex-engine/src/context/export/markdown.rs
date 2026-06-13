use super::text::{
    markdown_fence_for, option_or_null, provenance_inline, push_line, source_ref_inline,
    trim_final_newline,
};
use crate::context::{ContextPack, ContextPackAnomaly, ContextPackCell};

pub(super) fn to_markdown(pack: &ContextPack) -> String {
    let mut out = String::new();
    push_line(&mut out, "# CortexDB ContextPack");
    push_line(&mut out, "");
    push_line(&mut out, "- schema_version: `context_pack.v1`");
    push_line(
        &mut out,
        &format!("- token_budget_tokens: `{}`", pack.token_budget_tokens),
    );
    push_line(
        &mut out,
        &format!("- estimated_tokens: `{}`", pack.estimated_tokens),
    );
    push_line(&mut out, &format!("- truncated: `{}`", pack.truncated));
    push_line(
        &mut out,
        &format!("- citations_required: `{}`", pack.citations_required),
    );
    push_line(
        &mut out,
        &format!("- answerability_q16: `{}`", pack.answerability_q16),
    );
    push_line(
        &mut out,
        &format!(
            "- conflict_visibility_q16: `{}`",
            pack.conflict_visibility_q16
        ),
    );
    push_line(
        &mut out,
        &format!(
            "- visible_conflict_count: `{}`",
            pack.visible_conflict_count
        ),
    );
    push_line(&mut out, "");
    push_line(&mut out, "## Cells");
    if pack.cells.is_empty() {
        push_line(&mut out, "");
        push_line(&mut out, "_No cells selected._");
    } else {
        for (index, cell) in pack.cells.iter().enumerate() {
            push_markdown_cell(&mut out, index + 1, cell);
        }
    }
    push_line(&mut out, "");
    push_line(&mut out, "## Anomalies");
    if pack.anomalies.is_empty() {
        push_line(&mut out, "");
        push_line(&mut out, "_No anomalies._");
    } else {
        for anomaly in &pack.anomalies {
            push_markdown_anomaly(&mut out, anomaly);
        }
    }
    trim_final_newline(out)
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
