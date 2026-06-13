from __future__ import annotations

from typing import Callable, Protocol

from .aql import build_retrieve_context_aql, build_verify_fact_aql
from .grounding import _grounded_answer_response
from .models import ContextPackResponse, GroundedAnswerResponse, VerificationReportResponse


class GroundedAnswerClient(Protocol):
    def context_response(self, scope: str, statement: str) -> ContextPackResponse: ...

    def verify_response(self, scope: str, statement: str) -> VerificationReportResponse: ...


def answer_with_grounded_context(
    client: GroundedAnswerClient,
    scope: str,
    brain: str,
    question: str,
    answerer: Callable[[ContextPackResponse], str],
    *,
    mode: str | None = "balanced",
    budget_tokens: int | None = None,
    limit_candidates: int | None = None,
    where_clause: str | None = None,
    require_citations: bool = True,
    reject_unsupported: bool = False,
    verify_answer: bool = True,
) -> GroundedAnswerResponse:
    retrieve_statement = build_retrieve_context_aql(
        question,
        brain,
        mode=mode,
        budget_tokens=budget_tokens,
        limit_candidates=limit_candidates,
        where_clause=where_clause,
        require_citations=require_citations,
    )
    context = client.context_response(scope, retrieve_statement)
    answer = answerer(context)
    verify_statement = (
        build_verify_fact_aql(answer, brain) if verify_answer and answer.strip() else None
    )
    verification = (
        client.verify_response(scope, verify_statement)
        if verify_statement is not None
        else None
    )
    return _grounded_answer_response(
        question=question,
        answer=answer,
        retrieve_statement=retrieve_statement,
        verify_statement=verify_statement,
        context=context,
        verification=verification,
        require_citations=require_citations,
        reject_unsupported=reject_unsupported,
    )
