from __future__ import annotations

from typing import Any

from .models import (
    AnswerGroundingReportResponse,
    AnswerGroundingSpanResponse,
    ContextPackResponse,
    GroundedAnswerResponse,
    VerificationReportResponse,
)


def _tokenize(text: str) -> tuple[str, ...]:
    terms: list[str] = []
    current: list[str] = []
    for character in text.lower():
        if character.isalnum():
            current.append(character)
        elif current:
            term = "".join(current)
            if term not in {"a", "an", "and", "the", "or", "of", "to", "in"}:
                terms.append(term)
            current = []
    if current:
        term = "".join(current)
        if term not in {"a", "an", "and", "the", "or", "of", "to", "in"}:
            terms.append(term)
    return tuple(sorted(set(terms)))


def _split_answer_spans(answer: str) -> tuple[tuple[str, int, int], ...]:
    spans: list[tuple[str, int, int]] = []
    start = 0
    for index, character in enumerate(answer):
        if character in {"!", "?", "\n"} or (
            character == "."
            and not (
                index > 0
                and index + 1 < len(answer)
                and answer[index - 1].isdigit()
                and answer[index + 1].isdigit()
            )
        ):
            _push_answer_span(answer, start, index + 1, spans)
            start = index + 1
    _push_answer_span(answer, start, len(answer), spans)
    return tuple(spans)


def _push_answer_span(
    answer: str,
    start: int,
    end: int,
    spans: list[tuple[str, int, int]],
) -> None:
    raw = answer[start:end]
    text = raw.strip()
    if not text:
        return
    leading = len(raw) - len(raw.lstrip())
    trailing = len(raw) - len(raw.rstrip())
    spans.append((text, start + leading, end - trailing))


def _q16_ratio(numerator: int, denominator: int) -> int:
    if denominator == 0:
        return 65535
    return int(numerator * 65535 / denominator)


def _unique(values: list[str] | list[int]) -> tuple[Any, ...]:
    seen = set()
    out = []
    for value in values:
        if value not in seen:
            seen.add(value)
            out.append(value)
    return tuple(out)


def ground_answer(
    context: ContextPackResponse,
    answer: str,
    *,
    min_span_support_q16: int,
    require_citations: bool,
    reject_unsupported: bool,
) -> AnswerGroundingReportResponse:
    spans: list[AnswerGroundingSpanResponse] = []
    for text, start, end in _split_answer_spans(answer):
        span_terms = _tokenize(text)
        if not span_terms:
            spans.append(
                AnswerGroundingSpanResponse(
                    text=text,
                    start_byte=start,
                    end_byte=end,
                    support_q16=65535,
                    supported=True,
                    covered_terms=(),
                    missing_terms=(),
                    supported_by_cell_ids=(),
                    citations=(),
                )
            )
            continue
        covered: set[str] = set()
        cell_ids: list[int] = []
        citations: list[str] = []
        for cell in context.cells:
            cell_terms = set(_tokenize(cell.payload_text))
            matched = False
            for term in span_terms:
                if term in cell_terms:
                    covered.add(term)
                    matched = True
            if matched:
                cell_ids.append(cell.cell_id)
                if cell.citation:
                    citations.append(cell.citation)
        support = _q16_ratio(len(covered), len(span_terms))
        supported = support >= min_span_support_q16 and (
            not require_citations or bool(citations)
        )
        spans.append(
            AnswerGroundingSpanResponse(
                text=text,
                start_byte=start,
                end_byte=end,
                support_q16=support,
                supported=supported,
                covered_terms=tuple(sorted(covered)),
                missing_terms=tuple(term for term in span_terms if term not in covered),
                supported_by_cell_ids=_unique(cell_ids),
                citations=_unique(citations),
            )
        )
    supported_count = sum(1 for span in spans if span.supported)
    unsupported_count = len(spans) - supported_count
    average = int(sum(span.support_q16 for span in spans) / len(spans)) if spans else 65535
    return AnswerGroundingReportResponse(
        answer_supported=unsupported_count == 0,
        rejected=reject_unsupported and unsupported_count > 0,
        support_q16=average,
        supported_span_count=supported_count,
        unsupported_span_count=unsupported_count,
        spans=tuple(spans),
    )


def _grounded_answer_response(
    *,
    question: str,
    answer: str,
    retrieve_statement: str,
    verify_statement: str | None,
    context: ContextPackResponse,
    verification: "VerificationReportResponse | None",
    require_citations: bool,
    reject_unsupported: bool,
) -> GroundedAnswerResponse:
    grounding = context.ground_answer(
        answer,
        require_citations=require_citations,
        reject_unsupported=reject_unsupported,
    )
    citations = _unique(
        [
            citation
            for span in grounding.spans
            for citation in span.citations
        ]
        + [cell.citation for cell in context.cells if cell.citation]
    )
    used_cell_ids = _unique(
        [
            cell_id
            for span in grounding.spans
            for cell_id in span.supported_by_cell_ids
        ]
        + [cell.cell_id for cell in context.cells]
    )
    return GroundedAnswerResponse(
        question=question,
        answer=answer,
        retrieve_statement=retrieve_statement,
        verify_statement=verify_statement,
        context=context,
        grounding=grounding,
        verification=verification,
        citations=citations,
        used_context_cell_ids=used_cell_ids,
        rejected=grounding.rejected,
    )


