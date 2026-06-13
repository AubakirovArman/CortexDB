use crate::context::ContextSpanProvenance;
use crate::query::metadata::SourceRef;

pub(super) fn option_or_null(value: Option<&str>) -> &str {
    value.unwrap_or("null")
}

pub(super) fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

pub(super) fn markdown_fence_for(text: &str) -> String {
    let mut max_run = 0usize;
    let mut current = 0usize;
    for ch in text.chars() {
        if ch == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(max_run.max(2) + 1)
}

pub(super) fn trim_final_newline(mut value: String) -> String {
    if value.ends_with('\n') {
        value.pop();
    }
    value
}

pub(super) fn source_ref_inline(source_ref: &SourceRef) -> String {
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

pub(super) fn provenance_inline(provenance: &ContextSpanProvenance) -> String {
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
