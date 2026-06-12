"""Oracle-free answer intent detection for EnterpriseRAG-Bench runs."""

from __future__ import annotations

from typing import Any

from question_decomposition import tokens


PROJECT_SIGNALS = {
    "activation",
    "billing",
    "blocked",
    "canary",
    "credit",
    "customer",
    "dedicated",
    "demo",
    "failover",
    "fallback",
    "incident",
    "invoice",
    "latency",
    "migration",
    "onboarding",
    "paging",
    "prospect",
    "retry",
    "rollout",
    "runtime",
    "sdk",
    "slo",
    "spike",
    "streaming",
    "tenant",
    "timeout",
}

ACTION_SIGNALS = {
    "approved",
    "baseline",
    "cause",
    "caused",
    "check",
    "criteria",
    "dashboard",
    "deadline",
    "explain",
    "fix",
    "mitigation",
    "owner",
    "policy",
    "prevent",
    "recalculate",
    "remediate",
    "rollback",
    "standard",
    "status",
    "verify",
}

PHRASE_SIGNALS = (
    "what caused",
    "how should",
    "how do we",
    "do we need",
    "what exact",
    "what should",
    "which code",
    "which dashboards",
    "which evidence",
    "recommended oncall mitigations",
)

COMPLETENESS_SIGNALS = {
    "all",
    "complete",
    "comprehensive",
    "each",
    "every",
    "list",
    "summarize",
}

HIGH_LEVEL_SIGNALS = {
    "business",
    "company",
    "department",
    "departments",
    "mission",
    "overview",
    "pricing",
    "product",
    "products",
    "revenue",
    "strategy",
    "streams",
}

CONFLICT_SIGNALS = {
    "changed",
    "conflict",
    "conflicting",
    "current",
    "difference",
    "discrepancy",
    "latest",
    "newer",
    "older",
    "previous",
}

CONSTRAINED_SIGNALS = {
    "after",
    "before",
    "between",
    "except",
    "exclude",
    "for",
    "only",
    "within",
}

INTENT_BUDGETS: dict[str, dict[str, Any]] = {
    "default": {
        "top_k_context": None,
        "max_chars_per_doc": None,
        "max_tokens": None,
        "context_mode": None,
    },
    "constrained": {
        "top_k_context": 8,
        "max_chars_per_doc": 2600,
        "max_tokens": 700,
        "context_mode": None,
    },
    "conflict": {
        "top_k_context": 10,
        "max_chars_per_doc": 2800,
        "max_tokens": 800,
        "context_mode": None,
    },
    "completeness": {
        "top_k_context": 10,
        "max_chars_per_doc": 3200,
        "max_tokens": 900,
        "context_mode": None,
    },
    "complex_project": {
        "top_k_context": 10,
        "max_chars_per_doc": 3200,
        "max_tokens": 900,
        "context_mode": None,
    },
    "high_level": {
        "top_k_context": 10,
        "max_chars_per_doc": 5000,
        "max_tokens": 900,
        "context_mode": "brain-digest",
    },
}


def answer_intent_profile(question: str) -> dict[str, Any]:
    """Classify answer needs using only the visible question text."""

    lowered = question.lower()
    question_tokens = set(tokens(question))
    project_hits = sorted(PROJECT_SIGNALS & question_tokens)
    action_hits = sorted(ACTION_SIGNALS & question_tokens)
    completeness_hits = sorted(COMPLETENESS_SIGNALS & question_tokens)
    high_level_hits = sorted(HIGH_LEVEL_SIGNALS & question_tokens)
    conflict_hits = sorted(CONFLICT_SIGNALS & question_tokens)
    constrained_hits = sorted(CONSTRAINED_SIGNALS & question_tokens)
    phrase_hits = [phrase for phrase in PHRASE_SIGNALS if phrase in lowered]
    multi_part = lowered.count(" and ") >= 1 or lowered.count(",") >= 2
    long_question = len(tokens(question)) >= 18

    score = len(project_hits) * 2 + len(action_hits) + len(phrase_hits) * 2
    if multi_part:
        score += 1
    if long_question:
        score += 1

    intent = "default"
    if len(high_level_hits) >= 2 and not project_hits:
        intent = "high_level"
    elif conflict_hits and (multi_part or project_hits or action_hits):
        intent = "conflict"
    elif completeness_hits and (multi_part or long_question or lowered.startswith(("what are", "which "))):
        intent = "completeness"
    elif constrained_hits and (project_hits or action_hits or long_question):
        intent = "constrained"
    elif (
        (project_hits and action_hits and (multi_part or long_question))
        or len(project_hits) >= 3
        or (project_hits and phrase_hits)
    ):
        intent = "complex_project"
    return {
        "intent": intent,
        "score": score,
        "project_hits": project_hits,
        "action_hits": action_hits,
        "completeness_hits": completeness_hits,
        "high_level_hits": high_level_hits,
        "conflict_hits": conflict_hits,
        "constrained_hits": constrained_hits,
        "phrase_hits": phrase_hits,
        "budget_profile": INTENT_BUDGETS[intent],
    }
