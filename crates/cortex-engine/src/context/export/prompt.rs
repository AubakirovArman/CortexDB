use super::text::{
    option_or_null, provenance_inline, push_line, source_ref_inline, trim_final_newline,
};
use crate::context::{ContextPack, ContextPackAnomaly, ContextPackCell};

pub(super) fn to_agent_prompt(pack: &ContextPack) -> String {
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
            pack.token_budget_tokens,
            pack.estimated_tokens,
            pack.truncated,
            pack.citations_required
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Answerability: answerability_q16={}",
            pack.answerability_q16
        ),
    );
    push_line(
        &mut out,
        &format!(
            "Conflict visibility: conflict_visibility_q16={} visible_conflict_count={}",
            pack.conflict_visibility_q16, pack.visible_conflict_count
        ),
    );
    push_line(&mut out, "");
    push_line(&mut out, "Context cells:");
    for (index, cell) in pack.cells.iter().enumerate() {
        push_prompt_cell(&mut out, index + 1, cell);
    }
    if !pack.anomalies.is_empty() {
        push_line(&mut out, "");
        push_line(&mut out, "Context anomalies:");
        for anomaly in &pack.anomalies {
            push_prompt_anomaly(&mut out, anomaly);
        }
    }
    trim_final_newline(out)
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
