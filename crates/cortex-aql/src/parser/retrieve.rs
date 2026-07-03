use std::str::FromStr;

use nom::character::complete::{char, multispace0};
use nom::sequence::{delimited, preceded, tuple};
use nom::Err;

use crate::ast::{RawRetrieveContext, Requirement, Spanned};
use crate::errors::AqlParseErrorKind;
use crate::types::RetrievalMode;

use super::condition::parse_condition;
use super::string::parse_quoted_string;
use super::{
    kw, parse_identifier, parse_spanned_integer, parse_spanned_u32, span_between,
    starts_with_keyword, ws1, PResult, ParseFailure, Span,
};

mod requirements;
use requirements::parse_require;

pub(super) fn parse_retrieve_context(input: Span<'_>) -> PResult<'_, RawRetrieveContext<'_>> {
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
    let (mut input, _) = ws1(input)?;
    let (next, brain) = parse_identifier(input)?;
    input = next;

    let mut clauses = RetrieveClauses::default();
    loop {
        let (next, _) = multispace0(input)?;
        if next.fragment().starts_with(';') {
            input = next;
            break;
        }
        input = parse_next_clause(next, &mut clauses)?;
    }
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((
        input,
        RawRetrieveContext {
            task,
            brain,
            mode: clauses.mode,
            budget_tokens: clauses.budget_tokens,
            candidate_limit: clauses.candidate_limit,
            diversity_lambda_q16: clauses.diversity_lambda_q16,
            rerank_weight_q16: clauses.rerank_weight_q16,
            suppress_superseded: clauses.suppress_superseded,
            where_clause: clauses.where_clause,
            requirements: clauses.requirements,
            strategy: None,
        },
    ))
}

#[derive(Default)]
struct RetrieveClauses<'a> {
    mode: Option<Spanned<RetrievalMode>>,
    budget_tokens: Option<Spanned<u64>>,
    candidate_limit: Option<Spanned<u32>>,
    diversity_lambda_q16: Option<Spanned<u64>>,
    rerank_weight_q16: Option<Spanned<u64>>,
    suppress_superseded: bool,
    where_clause: Option<Spanned<crate::ast::Condition<'a>>>,
    requirements: Vec<Spanned<Requirement<'a>>>,
}

fn parse_next_clause<'a>(
    input: Span<'a>,
    clauses: &mut RetrieveClauses<'a>,
) -> Result<Span<'a>, nom::Err<ParseFailure<'a>>> {
    if starts_with_keyword(input, "USING") {
        // USING MODE <mode> | USING DIVERSITY <lambda_q16>
        let (after_using, _) = kw("USING")(input)?;
        let (after_using, _) = ws1(after_using)?;
        if starts_with_keyword(after_using, "DIVERSITY") {
            let (rest, _) = kw("DIVERSITY")(after_using)?;
            let (rest, _) = ws1(rest)?;
            let (rest, lambda) = parse_spanned_integer(rest)?;
            reject_duplicate(clauses.diversity_lambda_q16.replace(lambda), rest)?;
            Ok(rest)
        } else if starts_with_keyword(after_using, "RERANK") {
            let (rest, _) = kw("RERANK")(after_using)?;
            let (rest, _) = ws1(rest)?;
            let (rest, weight) = parse_spanned_integer(rest)?;
            reject_duplicate(clauses.rerank_weight_q16.replace(weight), rest)?;
            Ok(rest)
        } else {
            let (rest, mode) = parse_using_mode(input)?;
            reject_duplicate(clauses.mode.replace(mode), rest)?;
            Ok(rest)
        }
    } else if starts_with_keyword(input, "BUDGET") {
        let (input, budget) = parse_budget(input)?;
        reject_duplicate(clauses.budget_tokens.replace(budget), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "LIMIT") {
        let (input, limit) = parse_limit(input)?;
        reject_duplicate(clauses.candidate_limit.replace(limit), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "WHERE") {
        let (input, where_clause) = preceded(tuple((kw("WHERE"), ws1)), parse_condition)(input)?;
        reject_duplicate(clauses.where_clause.replace(where_clause), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "REQUIRE") {
        let (input, mut requirements) = parse_require(input)?;
        clauses.requirements.append(&mut requirements);
        Ok(input)
    } else if starts_with_keyword(input, "SUPPRESS") {
        let (input, _) = kw("SUPPRESS")(input)?;
        let (input, _) = ws1(input)?;
        let (input, _) = kw("SUPERSEDED")(input)?;
        if clauses.suppress_superseded {
            return Err(Err::Failure(ParseFailure::new(
                input,
                AqlParseErrorKind::Unexpected,
            )));
        }
        clauses.suppress_superseded = true;
        Ok(input)
    } else {
        Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    }
}

fn parse_using_mode(input: Span<'_>) -> PResult<'_, Spanned<RetrievalMode>> {
    let (input, _) = kw("USING")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("MODE")(input)?;
    let (input, _) = ws1(input)?;
    let start = input;
    let (input, identifier) = parse_identifier(input)?;
    let mode = RetrievalMode::from_str(identifier.node.value.as_ref())
        .map_err(|_| Err::Failure(ParseFailure::new(start, AqlParseErrorKind::InvalidMode)))?;
    Ok((input, Spanned::new(mode, span_between(start, input))))
}

fn parse_budget(input: Span<'_>) -> PResult<'_, Spanned<u64>> {
    let (input, _) = kw("BUDGET")(input)?;
    let (input, _) = ws1(input)?;
    let (input, budget) = parse_spanned_integer(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TOKENS")(input)?;
    Ok((input, budget))
}

fn parse_limit(input: Span<'_>) -> PResult<'_, Spanned<u32>> {
    let (input, _) = kw("LIMIT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, limit) = parse_spanned_u32(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("CANDIDATES")(input)?;
    Ok((input, limit))
}

fn reject_duplicate<T>(existing: Option<T>, input: Span<'_>) -> PResult<'_, ()> {
    if existing.is_some() {
        Err(Err::Failure(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    } else {
        Ok((input, ()))
    }
}
