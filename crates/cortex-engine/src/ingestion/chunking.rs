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
