from __future__ import annotations

from typing import Any


def document_metrics(question: dict[str, Any], answer: dict[str, Any]) -> tuple[float | None, int | None]:
    expected = {str(item) for item in question.get("expected_doc_ids", [])}
    retrieved = [str(item) for item in answer.get("document_ids", [])]
    if not expected:
        return None, None
    recall = len(expected & set(retrieved)) / len(expected) * 100.0
    invalid_extra = sum(1 for doc_id in retrieved if doc_id not in expected)
    return round(recall, 2), invalid_extra


def mean(values: list[float]) -> float:
    return round(sum(values) / len(values), 2) if values else 0.0


def aggregate_stats(rows: list[dict[str, Any]], *, include_total: bool = True) -> dict[str, Any]:
    recall_values = [
        float(row["document_recall_pct"])
        for row in rows
        if row.get("document_recall_pct") is not None
    ]
    invalid_values = [
        float(row["invalid_extra_docs"])
        for row in rows
        if row.get("invalid_extra_docs") is not None
    ]
    correct_pct = (
        sum(1 for row in rows if row.get("answer_correct")) / len(rows) * 100.0
        if rows
        else 0.0
    )
    completeness = mean([float(row.get("completeness_pct") or 0.0) for row in rows])
    combined = mean(
        [
            float(row.get("completeness_pct") or 0.0) if row.get("answer_correct") else 0.0
            for row in rows
        ]
    )
    stats: dict[str, Any] = {
        "average_correctness_pct": round(correct_pct, 2),
        "average_completeness_pct": completeness,
        "combined_correctness_completeness_score": combined,
        "average_recall_pct": mean(recall_values),
        "average_invalid_extra_docs": mean(invalid_values),
    }
    if include_total:
        stats.update(
            {
                "total_questions": len(rows),
                "completed_questions": len(rows),
                "skipped_rows": 0,
                "num_corrected_questions": 0,
            }
        )
    return stats


def question_type_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        grouped.setdefault(str(row.get("question_type") or "unknown"), []).append(row)
    return {
        name: aggregate_stats(items, include_total=False) | {"count": len(items)}
        for name, items in grouped.items()
    }
