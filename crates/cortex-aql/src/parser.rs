use std::num::ParseIntError;
use std::str::FromStr;

use nom::bytes::complete::{tag_no_case, take_while, take_while1};
use nom::character::complete::{char, digit1, multispace0};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};
use nom::{Err, IResult};
use nom_locate::LocatedSpan;

use crate::ast::{
    AqlStatement, Identifier, RawRemember, RawRetrieveContext, RawVerifyFact, SourceSpan, Spanned,
    TtlValue,
};
use crate::errors::{AqlParseError, AqlParseErrorKind};
use crate::types::RetrievalMode;

mod condition;
mod string;

use condition::parse_condition;
use string::parse_quoted_string;

pub(super) type Span<'a> = LocatedSpan<&'a str>;
pub(super) type PResult<'a, O> = IResult<Span<'a>, O, ParseFailure<'a>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParseFailure<'a> {
    span: Span<'a>,
    kind: AqlParseErrorKind,
}

pub fn parse_aql(input: &str) -> Result<AqlStatement<'_>, AqlParseError> {
    match all_consuming(delimited(multispace0, parse_statement, multispace0))(Span::new(input)) {
        Ok((_, statement)) => Ok(statement),
        Err(Err::Error(error)) | Err(Err::Failure(error)) => {
            Err(AqlParseError::new(error.kind, source_span(error.span)))
        }
        Err(Err::Incomplete(_)) => Err(AqlParseError::new(
            AqlParseErrorKind::Incomplete,
            SourceSpan::default(),
        )),
    }
}

fn parse_statement(input: Span<'_>) -> PResult<'_, AqlStatement<'_>> {
    if starts_with_keyword(input, "RETRIEVE") {
        map(parse_retrieve_context, |raw| {
            AqlStatement::RetrieveContext(Box::new(raw))
        })(input)
    } else if starts_with_keyword(input, "VERIFY") {
        map(parse_verify_fact, |raw| {
            AqlStatement::VerifyFact(Box::new(raw))
        })(input)
    } else if starts_with_keyword(input, "REMEMBER") {
        map(parse_remember, |raw| AqlStatement::Remember(Box::new(raw)))(input)
    } else {
        Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    }
}

fn parse_retrieve_context(input: Span<'_>) -> PResult<'_, RawRetrieveContext<'_>> {
    let (input, _) = kw("RETRIEVE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("CONTEXT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("FOR")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TASK")(input)?;
    let (input, _) = ws1(input)?;
    let (input, task) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("BRAIN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, brain) = parse_identifier(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("USING")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("MODE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, mode) = parse_mode(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("BUDGET")(input)?;
    let (input, _) = ws1(input)?;
    let (input, budget_tokens) = parse_spanned_integer(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TOKENS")(input)?;
    let (input, where_clause) =
        opt(preceded(tuple((ws1, kw("WHERE"), ws1)), parse_condition))(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;

    Ok((
        input,
        RawRetrieveContext {
            task,
            brain,
            mode,
            budget_tokens,
            candidate_limit: None,
            where_clause,
            requirements: Vec::new(),
            strategy: None,
        },
    ))
}

fn parse_mode(input: Span<'_>) -> PResult<'_, Spanned<RetrievalMode>> {
    let start = input;
    let (input, identifier) = parse_identifier(input)?;
    let mode = RetrievalMode::from_str(identifier.node.value.as_ref())
        .map_err(|_| Err::Failure(ParseFailure::new(start, AqlParseErrorKind::InvalidMode)))?;
    Ok((input, Spanned::new(mode, identifier.span)))
}

fn parse_verify_fact(input: Span<'_>) -> PResult<'_, RawVerifyFact<'_>> {
    let (input, _) = kw("VERIFY")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("FACT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, fact) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("BRAIN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, brain) = parse_identifier(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((input, RawVerifyFact { fact, brain }))
}

fn parse_remember(input: Span<'_>) -> PResult<'_, RawRemember<'_>> {
    let (input, _) = kw("REMEMBER")(input)?;
    let (input, _) = ws1(input)?;
    let (input, content) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("SCOPE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, scope) = parse_identifier(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("AS")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TYPE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, memory_type) = parse_identifier(input)?;
    let (input, ttl) = opt(preceded(
        tuple((ws1, kw("TTL"), ws1)),
        map(
            terminated(parse_spanned_integer, tuple((ws1, kw("SECONDS")))),
            |ttl| Spanned::new(TtlValue::Seconds(ttl.node), ttl.span),
        ),
    ))(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((
        input,
        RawRemember {
            content,
            scope,
            memory_type,
            ttl,
        },
    ))
}

pub(super) fn parse_identifier(input: Span<'_>) -> PResult<'_, Spanned<Identifier<'_>>> {
    spanned(map(
        recognize(pair(take_while1(is_ident_start), take_while(is_ident_rest))),
        |span: Span<'_>| Identifier::borrowed(span.fragment()),
    ))(input)
}

pub(super) fn parse_spanned_integer(input: Span<'_>) -> PResult<'_, Spanned<u64>> {
    spanned(parse_integer)(input)
}

pub(super) fn parse_integer(input: Span<'_>) -> PResult<'_, u64> {
    map_res(digit1, |span: Span<'_>| span.fragment().parse::<u64>())(input)
}

pub(super) fn spanned<'a, O, F>(mut parser: F) -> impl FnMut(Span<'a>) -> PResult<'a, Spanned<O>>
where
    F: FnMut(Span<'a>) -> PResult<'a, O>,
{
    move |input| {
        let start = input;
        let (input, node) = parser(input)?;
        Ok((input, Spanned::new(node, source_span(start))))
    }
}

pub(super) fn kw<'a>(keyword: &'static str) -> impl FnMut(Span<'a>) -> PResult<'a, Span<'a>> {
    tag_no_case(keyword)
}

pub(super) fn ws1(input: Span<'_>) -> PResult<'_, Span<'_>> {
    take_while1(|value: char| value.is_whitespace())(input)
}

fn is_ident_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_ident_rest(value: char) -> bool {
    value == '_' || value == '-' || value.is_ascii_alphanumeric()
}

fn starts_with_keyword(input: Span<'_>, keyword: &str) -> bool {
    input
        .fragment()
        .get(..keyword.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
}

pub(super) fn source_span(input: Span<'_>) -> SourceSpan {
    SourceSpan {
        offset: input.location_offset(),
        line: input.location_line(),
        column: input.get_column(),
    }
}

impl<'a> ParseFailure<'a> {
    pub(super) fn new(span: Span<'a>, kind: AqlParseErrorKind) -> Self {
        Self { span, kind }
    }
}

impl<'a> ParseError<Span<'a>> for ParseFailure<'a> {
    fn from_error_kind(input: Span<'a>, _kind: ErrorKind) -> Self {
        Self::new(input, AqlParseErrorKind::Unexpected)
    }

    fn append(_input: Span<'a>, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a> FromExternalError<Span<'a>, ParseIntError> for ParseFailure<'a> {
    fn from_external_error(input: Span<'a>, _kind: ErrorKind, _error: ParseIntError) -> Self {
        Self::new(input, AqlParseErrorKind::InvalidInteger)
    }
}
