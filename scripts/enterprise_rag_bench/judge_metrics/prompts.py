from __future__ import annotations

import json
import re
from typing import Any


def build_prompt(question: dict[str, Any], answer: dict[str, Any]) -> str:
    facts = "\n".join(f"- {fact}" for fact in question.get("answer_facts", []))
    return f"""You are judging an EnterpriseRAG-Bench answer.

Use the gold answer and required facts as the source of truth. Ignore whether
the retrieved documents were good or bad; score only the candidate answer.

Return only strict JSON with this shape:
{{"answer_correct": true|false, "completeness_pct": 0-100, "correctness_reasoning": "short reason"}}

Scoring rules:
- answer_correct is true only when the candidate answers the question without a material contradiction.
- completeness_pct estimates how much of the required facts are present.
- If the candidate says insufficient information while the gold answer contains facts, mark it incorrect.
- If the gold answer says the information is unavailable and the candidate also says that, mark it correct.
- Penalize wrong numbers, dates, file paths, headers, names, IDs, regions, and versions.
- Keep correctness_reasoning under 40 words.

Question:
{question.get("question", "")}

Gold answer:
{question.get("gold_answer", "")}

Required facts:
{facts}

Candidate answer:
{answer.get("answer", "")}
"""


def parse_judge_json(text: str) -> dict[str, Any]:
    cleaned = text.strip()
    if not cleaned:
        return {
            "answer_correct": False,
            "completeness_pct": 0.0,
            "correctness_reasoning": "judge returned empty response",
        }

    if cleaned.startswith("```"):
        cleaned = re.sub(r"^```(?:json)?", "", cleaned).strip()
        cleaned = re.sub(r"```$", "", cleaned).strip()

    candidates = re.findall(r"\{.*?\}", cleaned, flags=re.S)
    if not candidates:
        return {
            "answer_correct": False,
            "completeness_pct": 0.0,
            "correctness_reasoning": "judge response had no JSON object",
        }

    last_error: Exception | None = None
    for candidate in candidates:
        try:
            payload = json.loads(candidate)
            return {
                "answer_correct": bool(payload.get("answer_correct")),
                "completeness_pct": max(
                    0.0,
                    min(100.0, float(payload.get("completeness_pct", 0.0))),
                ),
                "correctness_reasoning": str(
                    payload.get("correctness_reasoning", "")
                ).strip()[:500],
            }
        except json.JSONDecodeError as error:
            last_error = error
            continue

    reason = (
        f"judge response parse error: {last_error}; raw={cleaned[:500]!r}"
        if last_error is not None
        else "judge response parse error"
    )
    return {
        "answer_correct": False,
        "completeness_pct": 0.0,
        "correctness_reasoning": reason,
    }
