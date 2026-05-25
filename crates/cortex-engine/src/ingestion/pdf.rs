use crate::error::{EngineError, EngineResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfExtractionStats {
    pub text: String,
    pub literal_strings: usize,
    pub hex_strings: usize,
}

pub fn extract_pdf_text(bytes: &[u8]) -> EngineResult<PdfExtractionStats> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(EngineError::InvalidOperation);
    }
    let text = String::from_utf8_lossy(bytes);
    let mut stats = PdfExtractionStats {
        text: String::new(),
        literal_strings: 0,
        hex_strings: 0,
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
    if stats.text.is_empty() {
        return Err(EngineError::InvalidOperation);
    }
    Ok(stats)
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
