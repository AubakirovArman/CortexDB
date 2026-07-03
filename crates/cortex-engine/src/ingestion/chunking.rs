use crate::error::{EngineError, EngineResult};

pub const DEFAULT_TEXT_CHUNK_MAX_CHARS: usize = 1_000;
pub const DEFAULT_TEXT_CHUNK_OVERLAP_CHARS: usize = 120;
pub const DEFAULT_TEXT_CHUNK_MIN_CHARS: usize = 1;
pub const DEFAULT_JSON_CHUNK_PATH_SEPARATOR: char = '.';
pub const DEFAULT_TABLE_CHUNK_FIRST_DATA_ROW: u32 = 2;
pub const DEFAULT_TABLE_CHUNK_CELL_RANGE_PREFIX: &str = "row-";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextChunkPolicy {
    pub max_chars: usize,
    pub overlap_chars: usize,
    pub min_chars: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextChunk {
    pub index: u32,
    pub chunk_id: String,
    pub text: String,
}

impl Default for TextChunkPolicy {
    fn default() -> Self {
        Self {
            max_chars: DEFAULT_TEXT_CHUNK_MAX_CHARS,
            overlap_chars: DEFAULT_TEXT_CHUNK_OVERLAP_CHARS,
            min_chars: DEFAULT_TEXT_CHUNK_MIN_CHARS,
        }
    }
}

impl TextChunkPolicy {
    pub fn validate(self) -> EngineResult<Self> {
        if self.max_chars == 0 || self.min_chars == 0 || self.min_chars > self.max_chars {
            return Err(EngineError::InvalidOperation);
        }
        if self.overlap_chars >= self.max_chars {
            return Err(EngineError::InvalidOperation);
        }
        Ok(self)
    }

    pub fn overlap_policy(self) -> TextOverlapPolicy {
        TextOverlapPolicy::FixedChars {
            chars: self.overlap_chars,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextOverlapPolicy {
    FixedChars { chars: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JsonChunkPolicy {
    pub path_separator: char,
    pub sort_paths: bool,
}

impl Default for JsonChunkPolicy {
    fn default() -> Self {
        Self {
            path_separator: DEFAULT_JSON_CHUNK_PATH_SEPARATOR,
            sort_paths: true,
        }
    }
}

impl JsonChunkPolicy {
    pub fn validate(self) -> EngineResult<Self> {
        if matches!(self.path_separator, '\0' | '\n' | '\r') {
            return Err(EngineError::InvalidOperation);
        }
        Ok(self)
    }

    pub fn join_path(self, prefix: &str, child: &str) -> String {
        if prefix.is_empty() {
            child.to_owned()
        } else {
            format!("{prefix}{}{child}", self.path_separator)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableChunkPolicy {
    pub first_data_row: u32,
    pub cell_range_prefix: &'static str,
}

impl Default for TableChunkPolicy {
    fn default() -> Self {
        Self {
            first_data_row: DEFAULT_TABLE_CHUNK_FIRST_DATA_ROW,
            cell_range_prefix: DEFAULT_TABLE_CHUNK_CELL_RANGE_PREFIX,
        }
    }
}

impl TableChunkPolicy {
    pub fn validate(self) -> EngineResult<Self> {
        if self.first_data_row == 0
            || self.cell_range_prefix.is_empty()
            || self.cell_range_prefix.contains('\n')
            || self.cell_range_prefix.contains('\r')
        {
            return Err(EngineError::InvalidOperation);
        }
        Ok(self)
    }

    pub fn source_row_number(self, zero_based_data_index: usize) -> EngineResult<u32> {
        let offset = u32::try_from(zero_based_data_index)
            .map_err(|_| EngineError::StorageInvariant("table row overflow".to_owned()))?;
        self.first_data_row
            .checked_add(offset)
            .ok_or_else(|| EngineError::StorageInvariant("table row overflow".to_owned()))
    }

    pub fn cell_range(self, source_row: u32) -> String {
        format!("{}{source_row}", self.cell_range_prefix)
    }
}

pub fn split_text_chunks(
    document_id: &str,
    text: &str,
    policy: TextChunkPolicy,
) -> EngineResult<Vec<TextChunk>> {
    let policy = policy.validate()?;
    let mut chunks = Vec::<String>::new();
    let mut current_chunk = String::new();

    for paragraph in text.split("\n\n").map(str::trim) {
        if paragraph.is_empty() {
            continue;
        }
        if char_len(paragraph) > policy.max_chars {
            flush_chunk(&mut chunks, &mut current_chunk, policy);
            split_long_text(paragraph, policy, &mut chunks);
            continue;
        }
        if current_chunk.is_empty() {
            current_chunk = paragraph.to_owned();
            continue;
        }
        let combined_chars = char_len(&current_chunk) + 2 + char_len(paragraph);
        if combined_chars <= policy.max_chars {
            current_chunk.push_str("\n\n");
            current_chunk.push_str(paragraph);
        } else {
            flush_chunk(&mut chunks, &mut current_chunk, policy);
            current_chunk = paragraph.to_owned();
        }
    }
    flush_chunk(&mut chunks, &mut current_chunk, policy);

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let index = u32::try_from(index + 1).map_err(|_| {
                EngineError::StorageInvariant("text chunk count exceeds u32".to_owned())
            })?;
            Ok(TextChunk {
                index,
                chunk_id: stable_chunk_id(document_id, index),
                text,
            })
        })
        .collect()
}

pub fn count_text_chunks(
    document_id: &str,
    text: &str,
    policy: TextChunkPolicy,
) -> EngineResult<usize> {
    split_text_chunks(document_id, text, policy).map(|chunks| chunks.len())
}

pub fn stable_chunk_id(document_id: &str, index: u32) -> String {
    format!("{}#chunk-{index:04}", sanitize_chunk_component(document_id))
}

pub(crate) fn sanitize_header_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

fn split_long_text(text: &str, policy: TextChunkPolicy, chunks: &mut Vec<String>) {
    let chars = text.chars().collect::<Vec<_>>();
    let mut start = 0usize;
    while start < chars.len() {
        let end = start.saturating_add(policy.max_chars).min(chars.len());
        let chunk = chars[start..end].iter().collect::<String>();
        push_if_large_enough(chunks, chunk, policy);
        if end == chars.len() {
            break;
        }
        let next_start = end.saturating_sub(policy.overlap_chars);
        start = if next_start <= start { end } else { next_start };
    }
}

fn flush_chunk(chunks: &mut Vec<String>, current_chunk: &mut String, policy: TextChunkPolicy) {
    if current_chunk.is_empty() {
        return;
    }
    let chunk = std::mem::take(current_chunk);
    push_if_large_enough(chunks, chunk, policy);
}

fn push_if_large_enough(chunks: &mut Vec<String>, chunk: String, policy: TextChunkPolicy) {
    if char_len(chunk.trim()) >= policy.min_chars {
        chunks.push(chunk);
    }
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn sanitize_chunk_component(value: &str) -> String {
    let mut out = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.') {
            out.push(character);
        } else if character.is_whitespace() || matches!(character, '/' | '\\' | '#') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_owned();
    if out.is_empty() {
        "document".to_owned()
    } else {
        out
    }
}

// ---- A8.1: structure-aware chunking -----------------------------------------

/// Whether a structured chunk is the document-summary parent or a body child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredChunkRole {
    Parent,
    Child,
}

impl StructuredChunkRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parent => "parent",
            Self::Child => "child",
        }
    }
}

/// A body/parent chunk with the structural metadata A7.1 field weights and
/// parent-context expansion consume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuredChunk {
    pub index: u32,
    pub chunk_id: String,
    pub text: String,
    pub role: StructuredChunkRole,
    pub parent_id: Option<String>,
    /// Heading breadcrumb, e.g. `"Runbook > Recovery > Steps"` (empty for the
    /// document preamble and the parent summary).
    pub breadcrumb: String,
}

struct HeadingSection {
    breadcrumb: String,
    body: String,
}

/// A8.1: structure-aware chunking. Splits a document on Markdown headings
/// (tracking a heading breadcrumb), never cuts a fenced code block or a run of
/// table rows, and emits a document-summary **parent** chunk (the heading
/// outline) plus **child** body chunks carrying `parent_id` + `breadcrumb`.
/// Fully deterministic — the same bytes yield the same chunk-ids, roles, and
/// order. This is an additive path; [`split_text_chunks`] (paragraph-based) is
/// unchanged, so existing ingestion and its goldens are untouched.
pub fn split_text_chunks_structured(
    document_id: &str,
    text: &str,
    policy: TextChunkPolicy,
) -> EngineResult<Vec<StructuredChunk>> {
    let policy = policy.validate()?;
    let sections = segment_by_headings(text);

    let mut out = Vec::new();
    let mut next_index = 1u32;
    let parent_id = stable_chunk_id(document_id, next_index);
    out.push(StructuredChunk {
        index: next_index,
        chunk_id: parent_id.clone(),
        text: parent_summary(&sections),
        role: StructuredChunkRole::Parent,
        parent_id: None,
        breadcrumb: String::new(),
    });

    for section in &sections {
        for body in split_section_atomic(&section.body, policy) {
            next_index = next_index.checked_add(1).ok_or_else(|| {
                EngineError::StorageInvariant("structured chunk count exceeds u32".to_owned())
            })?;
            out.push(StructuredChunk {
                index: next_index,
                chunk_id: stable_chunk_id(document_id, next_index),
                text: body,
                role: StructuredChunkRole::Child,
                parent_id: Some(parent_id.clone()),
                breadcrumb: section.breadcrumb.clone(),
            });
        }
    }
    Ok(out)
}

/// Markdown ATX heading level (1..=6) if `line` is a heading, else `None`.
fn heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&hashes) && trimmed.chars().nth(hashes) == Some(' ')).then_some(hashes)
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.matches('|').count() >= 2
}

/// Segments the document into (breadcrumb, body) sections at headings. Lines
/// inside a fenced code block are never treated as headings.
fn segment_by_headings(text: &str) -> Vec<HeadingSection> {
    let mut sections: Vec<HeadingSection> = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut breadcrumb = String::new();
    let mut body = String::new();
    let mut in_code = false;

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            body.push_str(line);
            body.push('\n');
            continue;
        }
        if !in_code {
            if let Some(level) = heading_level(line) {
                push_section(&mut sections, &breadcrumb, &mut body);
                let title = line.trim_start().trim_start_matches('#').trim().to_owned();
                stack.truncate(level.saturating_sub(1));
                stack.push(title);
                breadcrumb = stack.join(" > ");
                continue;
            }
        }
        body.push_str(line);
        body.push('\n');
    }
    push_section(&mut sections, &breadcrumb, &mut body);
    sections
}

fn push_section(sections: &mut Vec<HeadingSection>, breadcrumb: &str, body: &mut String) {
    let trimmed = body.trim();
    if !trimmed.is_empty() {
        sections.push(HeadingSection {
            breadcrumb: breadcrumb.to_owned(),
            body: trimmed.to_owned(),
        });
    }
    body.clear();
}

/// The parent summary is the document's heading outline (unique breadcrumbs), or
/// — for a heading-less document — the lead of its first section.
fn parent_summary(sections: &[HeadingSection]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut outline = Vec::new();
    for section in sections {
        if !section.breadcrumb.is_empty() && seen.insert(section.breadcrumb.as_str()) {
            outline.push(section.breadcrumb.clone());
        }
    }
    if !outline.is_empty() {
        return outline.join("\n");
    }
    sections
        .first()
        .map(|section| section.body.chars().take(200).collect())
        .unwrap_or_default()
}

/// Splits a section body into `<= max_chars` chunks, treating each fenced code
/// block and each run of table rows as one indivisible unit (kept whole even if
/// it exceeds `max_chars`); regular prose can still be split.
fn split_section_atomic(body: &str, policy: TextChunkPolicy) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for (unit, atomic) in atomic_units(body) {
        if char_len(&unit) > policy.max_chars {
            flush_plain(&mut chunks, &mut current);
            if atomic {
                chunks.push(unit);
            } else {
                split_long_text(&unit, policy, &mut chunks);
            }
            continue;
        }
        if current.is_empty() {
            current = unit;
        } else if char_len(&current) + 2 + char_len(&unit) <= policy.max_chars {
            current.push_str("\n\n");
            current.push_str(&unit);
        } else {
            flush_plain(&mut chunks, &mut current);
            current = unit;
        }
    }
    flush_plain(&mut chunks, &mut current);
    chunks
}

fn flush_plain(chunks: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        chunks.push(trimmed.to_owned());
    }
    current.clear();
}

/// Breaks a section body into units, each flagged `atomic` (a code fence or a
/// table-row run — never split) or not (prose paragraphs, split on blank lines).
fn atomic_units(body: &str) -> Vec<(String, bool)> {
    let lines: Vec<&str> = body.lines().collect();
    let mut units = Vec::new();
    let mut plain = String::new();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        if line.trim_start().starts_with("```") {
            flush_plain_paragraphs(&mut units, &mut plain);
            let mut block = String::new();
            block.push_str(line);
            block.push('\n');
            index += 1;
            while index < lines.len() {
                block.push_str(lines[index]);
                block.push('\n');
                let closing = lines[index].trim_start().starts_with("```");
                index += 1;
                if closing {
                    break;
                }
            }
            units.push((block.trim_end().to_owned(), true));
            continue;
        }
        if is_table_row(line) {
            flush_plain_paragraphs(&mut units, &mut plain);
            let mut block = String::new();
            while index < lines.len() && is_table_row(lines[index]) {
                block.push_str(lines[index]);
                block.push('\n');
                index += 1;
            }
            units.push((block.trim_end().to_owned(), true));
            continue;
        }
        plain.push_str(line);
        plain.push('\n');
        index += 1;
    }
    flush_plain_paragraphs(&mut units, &mut plain);
    units
}

fn flush_plain_paragraphs(units: &mut Vec<(String, bool)>, plain: &mut String) {
    for paragraph in plain.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        units.push((paragraph.to_owned(), false));
    }
    plain.clear();
}

#[cfg(test)]
mod structured_tests {
    use super::*;

    fn structured(text: &str) -> Vec<StructuredChunk> {
        split_text_chunks_structured("doc", text, TextChunkPolicy::default()).unwrap()
    }

    #[test]
    fn parent_summary_is_the_heading_outline_and_children_carry_breadcrumbs() {
        let doc = "# Runbook\n\nintro text\n\n## Recovery\n\nrestore from backup\n\n### Steps\n\ndo the thing";
        let chunks = structured(doc);
        assert_eq!(chunks[0].role, StructuredChunkRole::Parent);
        assert_eq!(chunks[0].parent_id, None);
        assert!(chunks[0].text.contains("Runbook > Recovery > Steps"));
        // Every child points at the parent and carries its heading breadcrumb.
        let steps = chunks
            .iter()
            .find(|c| c.text.contains("do the thing"))
            .expect("steps chunk");
        assert_eq!(steps.role, StructuredChunkRole::Child);
        assert_eq!(steps.parent_id.as_deref(), Some(chunks[0].chunk_id.as_str()));
        assert_eq!(steps.breadcrumb, "Runbook > Recovery > Steps");
    }

    #[test]
    fn a_fenced_code_block_is_never_split_and_hashes_inside_are_not_headings() {
        let doc = "# Code\n\n```\nfn main() {\n    # not a heading\n\n    let x = 1;\n}\n```\n\nafter";
        let chunks = structured(doc);
        let code = chunks
            .iter()
            .find(|c| c.text.contains("fn main()"))
            .expect("code chunk");
        // The whole fence (including its blank line and the '#' line) stays intact.
        assert!(code.text.contains("# not a heading"));
        assert!(code.text.contains("let x = 1;"));
        assert_eq!(code.breadcrumb, "Code");
    }

    #[test]
    fn a_run_of_table_rows_stays_one_chunk() {
        let doc = "## Prices\n\n| item | price |\n| --- | --- |\n| a | 1 |\n| b | 2 |";
        let chunks = structured(doc);
        let table = chunks
            .iter()
            .find(|c| c.text.contains("| item | price |"))
            .expect("table chunk");
        assert!(table.text.contains("| a | 1 |") && table.text.contains("| b | 2 |"));
    }

    #[test]
    fn deterministic_same_bytes_same_chunks() {
        let doc = "# A\n\nalpha\n\n## B\n\nbeta";
        assert_eq!(structured(doc), structured(doc));
    }

    #[test]
    fn heading_less_document_produces_a_lead_summary_and_children() {
        let doc = "just some prose\n\nmore prose";
        let chunks = structured(doc);
        assert_eq!(chunks[0].role, StructuredChunkRole::Parent);
        assert!(chunks[0].text.contains("just some prose"));
        assert!(chunks.iter().skip(1).all(|c| c.role == StructuredChunkRole::Child));
    }
}
