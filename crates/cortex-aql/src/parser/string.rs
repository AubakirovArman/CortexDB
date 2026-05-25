use nom::{Err, InputTake};

use crate::ast::{AqlString, Spanned};
use crate::errors::AqlParseErrorKind;

use super::{source_span, PResult, ParseFailure, Span};

pub(super) fn parse_quoted_string(input: Span<'_>) -> PResult<'_, Spanned<AqlString<'_>>> {
    let source: &str = input.fragment();
    if !source.starts_with('"') {
        return Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )));
    }

    let bytes = source.as_bytes();
    let mut index = 1;
    let mut last_plain = 1;
    let mut owned = None::<String>;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let consumed = index + 1;
                let (rest, token) = input.take_split(consumed);
                let fragment: &str = token.fragment();
                let node = if let Some(mut value) = owned {
                    value.push_str(&source[last_plain..index]);
                    AqlString::owned(value)
                } else {
                    AqlString::borrowed(&fragment[1..fragment.len() - 1])
                };
                return Ok((rest, Spanned::new(node, source_span(input))));
            }
            b'\\' => {
                let Some(escaped) = bytes.get(index + 1).copied() else {
                    return invalid_escape(input);
                };
                let value = owned.get_or_insert_with(String::new);
                value.push_str(&source[last_plain..index]);
                match escaped {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    b'r' => value.push('\r'),
                    _ => return invalid_escape(input),
                }
                index += 2;
                last_plain = index;
            }
            _ => index += 1,
        }
    }
    Err(Err::Failure(ParseFailure::new(
        input,
        AqlParseErrorKind::Incomplete,
    )))
}

fn invalid_escape(input: Span<'_>) -> PResult<'_, Spanned<AqlString<'_>>> {
    Err(Err::Failure(ParseFailure::new(
        input,
        AqlParseErrorKind::InvalidStringEscape,
    )))
}
