use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::error::{EngineError, EngineResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfExtractedPageText {
    pub page: u32,
    pub text: String,
    pub literal_strings: usize,
    pub hex_strings: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfExtractionStats {
    pub text: String,
    pub literal_strings: usize,
    pub hex_strings: usize,
    pub page_count: u32,
    pub pages: Vec<PdfExtractedPageText>,
}

pub fn extract_pdf_text(bytes: &[u8]) -> EngineResult<PdfExtractionStats> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(EngineError::InvalidOperation);
    }
    let mut pages = extract_stream_pages(bytes)?;
    if pages.is_empty() {
        let expanded = expand_pdf_streams(bytes)?;
        let fallback = extract_text_from_content(&expanded)?;
        if !fallback.text.is_empty() {
            pages.push(PdfExtractedPageText {
                page: 1,
                text: fallback.text,
                literal_strings: fallback.literal_strings,
                hex_strings: fallback.hex_strings,
            });
        }
    }
    let stats = aggregate_pages(pages)?;
    if stats.text.is_empty() {
        return Err(EngineError::InvalidOperation);
    }
    Ok(stats)
}

fn extract_stream_pages(bytes: &[u8]) -> EngineResult<Vec<PdfExtractedPageText>> {
    let mut pages = Vec::new();
    for (dict, stream) in stream_sections(bytes) {
        let content = if contains_bytes(dict, b"/FlateDecode") {
            inflate_zlib(stream)?
        } else {
            stream.to_vec()
        };
        let page_stats = extract_text_from_content(&content)?;
        if page_stats.text.is_empty() {
            continue;
        }
        let page = u32::try_from(pages.len() + 1)
            .map_err(|_| EngineError::StorageInvariant("PDF page count exceeds u32".to_owned()))?;
        pages.push(PdfExtractedPageText {
            page,
            text: page_stats.text,
            literal_strings: page_stats.literal_strings,
            hex_strings: page_stats.hex_strings,
        });
    }
    Ok(pages)
}

fn extract_text_from_content(bytes: &[u8]) -> EngineResult<PdfExtractedPageText> {
    let text = String::from_utf8_lossy(bytes);
    let mut stats = PdfExtractionStats {
        text: String::new(),
        literal_strings: 0,
        hex_strings: 0,
        page_count: 0,
        pages: Vec::new(),
    };
    let mut in_text = false;
    for token in tokenize_pdf(&text) {
        match token.as_str() {
            "BT" => in_text = true,
            "ET" => in_text = false,
            _ if in_text && token.starts_with('(') => {
                push_text(&mut stats.text, &decode_literal(&token));
                stats.literal_strings += 1;
            }
            _ if in_text && token.starts_with('<') && !token.starts_with("<<") => {
                push_text(&mut stats.text, &decode_hex(&token)?);
                stats.hex_strings += 1;
            }
            _ => {}
        }
    }
    Ok(PdfExtractedPageText {
        page: 0,
        text: stats.text,
        literal_strings: stats.literal_strings,
        hex_strings: stats.hex_strings,
    })
}

fn aggregate_pages(pages: Vec<PdfExtractedPageText>) -> EngineResult<PdfExtractionStats> {
    let page_count = u32::try_from(pages.len())
        .map_err(|_| EngineError::StorageInvariant("PDF page count exceeds u32".to_owned()))?;
    let text = pages
        .iter()
        .map(|page| page.text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    Ok(PdfExtractionStats {
        text,
        literal_strings: pages.iter().map(|page| page.literal_strings).sum(),
        hex_strings: pages.iter().map(|page| page.hex_strings).sum(),
        page_count,
        pages,
    })
}

fn expand_pdf_streams(bytes: &[u8]) -> EngineResult<Vec<u8>> {
    let mut out = bytes.to_vec();
    for (dict, stream) in stream_sections(bytes) {
        if contains_bytes(dict, b"/FlateDecode") {
            out.extend_from_slice(b"\n");
            out.extend_from_slice(&inflate_zlib(stream)?);
        }
    }
    Ok(out)
}

fn stream_sections(bytes: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut sections = Vec::new();
    let mut offset = 0;
    while let Some(stream_at) = find_bytes(&bytes[offset..], b"stream") {
        let stream_at = offset + stream_at;
        let before = &bytes[..stream_at];
        let after_marker_start = stream_at + b"stream".len();
        let Some(end_rel) = find_bytes(&bytes[after_marker_start..], b"endstream") else {
            break;
        };
        let end_at = after_marker_start + end_rel;
        let stream = trim_newlines(&bytes[after_marker_start..end_at]);
        let dict_start = rfind_bytes(before, b"<<").unwrap_or(0);
        sections.push((&before[dict_start..], stream));
        offset = end_at + b"endstream".len();
    }
    sections
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|value| value == needle)
}

fn rfind_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|value| value == needle)
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    find_bytes(haystack, needle).is_some()
}

fn trim_newlines(mut value: &[u8]) -> &[u8] {
    while matches!(value.first(), Some(b'\r' | b'\n')) {
        value = &value[1..];
    }
    while matches!(value.last(), Some(b'\r' | b'\n')) {
        value = &value[..value.len() - 1];
    }
    value
}

fn inflate_zlib(bytes: &[u8]) -> EngineResult<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(bytes);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|_| EngineError::InvalidOperation)?;
    Ok(out)
}

fn tokenize_pdf(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '(' => tokens.push(read_literal(&mut chars)),
            '<' if chars.peek() != Some(&'<') => tokens.push(read_hex(&mut chars)),
            value if value.is_whitespace() => {}
            value => tokens.push(read_word(value, &mut chars)),
        }
    }
    tokens
}

fn read_literal(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut out = String::from("(");
    let mut escaped = false;
    for character in chars.by_ref() {
        out.push(character);
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == ')' {
            break;
        }
    }
    out
}

fn read_hex(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut out = String::from("<");
    for character in chars.by_ref() {
        out.push(character);
        if character == '>' {
            break;
        }
    }
    out
}

fn read_word(first: char, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut out = String::from(first);
    while let Some(character) = chars.peek().copied() {
        if character.is_whitespace() || matches!(character, '(' | '<') {
            break;
        }
        out.push(character);
        chars.next();
    }
    out
}

fn decode_literal(token: &str) -> String {
    let inner = token.trim_start_matches('(').trim_end_matches(')');
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('b') => out.push('\u{0008}'),
                Some('f') => out.push('\u{000c}'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(character);
        }
    }
    out
}

fn decode_hex(token: &str) -> EngineResult<String> {
    let mut hex = token
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_owned();
    hex.retain(|character| !character.is_whitespace());
    if hex.len() % 2 == 1 {
        hex.push('0');
    }
    let mut bytes = Vec::new();
    for chunk in hex.as_bytes().chunks(2) {
        let value = std::str::from_utf8(chunk).map_err(|_| EngineError::InvalidOperation)?;
        bytes.push(u8::from_str_radix(value, 16).map_err(|_| EngineError::InvalidOperation)?);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn push_text(dst: &mut String, value: &str) {
    if !dst.is_empty() && !dst.ends_with(char::is_whitespace) {
        dst.push(' ');
    }
    dst.push_str(value.trim());
}
