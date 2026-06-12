from __future__ import annotations


def _quote_aql_string(value: str) -> str:
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )
    return f'"{escaped}"'


def _validate_aql_identifier(field: str, value: str) -> None:
    if not value:
        raise ValueError(f"{field} must be a non-empty AQL identifier")
    first = value[0]
    if not (first == "_" or first.isascii() and first.isalpha()):
        raise ValueError(f"{field} must start with '_' or an ASCII letter")
    for character in value[1:]:
        if not (
            character == "_"
            or character == "-"
            or character == ":"
            or character.isascii()
            and character.isalnum()
        ):
            raise ValueError(f"{field} contains an invalid AQL identifier character")


def _validate_decimal(field: str, value: str | None) -> None:
    if value is None:
        return
    left, separator, right = value.partition(".")
    if separator != "." or not left.isdigit() or not right.isdigit():
        raise ValueError(f"{field} must be a decimal literal")


def build_retrieve_context_aql(
    task: str,
    brain: str,
    *,
    mode: str | None = None,
    budget_tokens: int | None = None,
    limit_candidates: int | None = None,
    where_clause: str | None = None,
    require_citations: bool = False,
    min_confidence: str | None = None,
    source_trust: str | None = None,
    freshness_seconds: int | None = None,
    explain: bool = False,
) -> str:
    _validate_aql_identifier("brain", brain)
    if mode is not None and mode not in {"fast", "balanced", "semantic", "audit"}:
        raise ValueError("mode must be fast, balanced, semantic, or audit")
    if where_clause is not None and not where_clause.strip():
        raise ValueError("where_clause must not be empty")
    _validate_decimal("min_confidence", min_confidence)
    _validate_decimal("source_trust", source_trust)

    parts = []
    if explain:
        parts.append("EXPLAIN")
    parts.extend(["RETRIEVE CONTEXT FOR TASK", _quote_aql_string(task), "IN BRAIN", brain])
    if mode is not None:
        parts.extend(["USING MODE", mode])
    if budget_tokens is not None:
        parts.extend(["BUDGET", str(budget_tokens), "TOKENS"])
    if limit_candidates is not None:
        parts.extend(["LIMIT", str(limit_candidates), "CANDIDATES"])
    if where_clause is not None:
        parts.extend(["WHERE", where_clause.strip()])
    if require_citations:
        parts.extend(["REQUIRE", "citations"])
    if min_confidence is not None:
        parts.extend(["REQUIRE", "confidence", ">=", min_confidence])
    if source_trust is not None:
        parts.extend(["REQUIRE", "source_trust", ">=", source_trust])
    if freshness_seconds is not None:
        parts.extend(["REQUIRE", "freshness", "<=", str(freshness_seconds), "SECONDS"])
    return " ".join(parts) + ";"


def build_verify_fact_aql(fact: str, brain: str) -> str:
    _validate_aql_identifier("brain", brain)
    return f"VERIFY FACT {_quote_aql_string(fact)} IN BRAIN {brain};"


def build_remember_aql(
    content: str,
    scope: str,
    memory_type: str,
    *,
    ttl_seconds: int | None = None,
) -> str:
    _validate_aql_identifier("scope", scope)
    _validate_aql_identifier("memory_type", memory_type)
    statement = (
        f"REMEMBER {_quote_aql_string(content)} IN SCOPE {scope} AS TYPE {memory_type}"
    )
    if ttl_seconds is not None:
        statement += f" TTL {ttl_seconds} SECONDS"
    return statement + ";"


