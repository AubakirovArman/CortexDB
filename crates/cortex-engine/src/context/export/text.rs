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
