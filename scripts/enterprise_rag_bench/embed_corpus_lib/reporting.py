"""Coverage reporting for resumable corpus embedding outputs."""

from __future__ import annotations

import json
from pathlib import Path

from embed_corpus_lib.state import load_done_ids


def embedding_output_report(
    output: Path,
    expected_ids: list[str],
    *,
    model: str,
    min_coverage_bps: int,
    expected_dimension: int | None,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> dict:
    expected = set(expected_ids)
    seen: set[str] = set()
    duplicate_ids: list[str] = []
    unexpected_ids: list[str] = []
    invalid_rows: list[str] = []
    duplicate_count = 0
    unexpected_count = 0
    invalid_count = 0
    empty_vector_count = 0
    dimension_mismatch_count = 0
    stale_count = 0
    dimension = expected_dimension
    stale_ids: list[str] = []
    if output.exists():
        with output.open("r", encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, 1):
                line = line.strip()
                if not line:
                    continue
                try:
                    row = json.loads(line)
                except json.JSONDecodeError as error:
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: invalid json: {error}")
                    continue
                doc_id = row.get("doc_id")
                vector = row.get("vector")
                if not isinstance(doc_id, str) or not doc_id:
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: missing doc_id")
                    continue
                if not isinstance(vector, list):
                    invalid_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: missing vector for {doc_id}")
                    continue
                if not vector:
                    empty_vector_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(f"line {line_number}: empty vector for {doc_id}")
                    continue
                if dimension is None:
                    dimension = len(vector)
                elif len(vector) != dimension:
                    dimension_mismatch_count += 1
                    if len(invalid_rows) < 25:
                        invalid_rows.append(
                            f"line {line_number}: vector dimension {len(vector)} for {doc_id}, expected {dimension}"
                        )
                    continue
                if doc_id not in expected:
                    unexpected_count += 1
                    if len(unexpected_ids) < 25 and doc_id not in unexpected_ids:
                        unexpected_ids.append(doc_id)
                    continue
                if expected_model is not None and row.get("model") != expected_model:
                    stale_count += 1
                    if len(stale_ids) < 25 and doc_id not in stale_ids:
                        stale_ids.append(doc_id)
                    continue
                expected_hash = expected_text_hashes.get(doc_id) if expected_text_hashes else None
                if expected_hash is not None and row.get("text_hash") != expected_hash:
                    stale_count += 1
                    if len(stale_ids) < 25 and doc_id not in stale_ids:
                        stale_ids.append(doc_id)
                    continue
                if doc_id in seen:
                    duplicate_count += 1
                    if len(duplicate_ids) < 25 and doc_id not in duplicate_ids:
                        duplicate_ids.append(doc_id)
                    continue
                seen.add(doc_id)
    missing = sorted(expected - seen)
    coverage_bps = 10_000 if not expected_ids else int(len(seen) * 10_000 / len(expected_ids))
    production_ready = (
        coverage_bps >= min_coverage_bps
        and duplicate_count == 0
        and unexpected_count == 0
        and invalid_count == 0
        and empty_vector_count == 0
        and dimension_mismatch_count == 0
        and stale_count == 0
    )
    return {
        "schema_version": "cortexdb.embedding_pipeline.coverage.v1",
        "model": model,
        "output": str(output),
        "total_items": len(expected_ids),
        "embedded_items": len(seen),
        "missing_items": len(missing),
        "duplicate_items": duplicate_count,
        "unexpected_items": unexpected_count,
        "invalid_rows": invalid_count,
        "empty_vector_rows": empty_vector_count,
        "dimension_mismatch_rows": dimension_mismatch_count,
        "stale_items": stale_count,
        "dimension": dimension,
        "expected_dimension": expected_dimension,
        "expected_model": expected_model,
        "coverage_basis_points": coverage_bps,
        "coverage_percent": coverage_bps / 100.0,
        "min_coverage_basis_points": min_coverage_bps,
        "production_ready": production_ready,
        "missing_ids_sample": missing[:25],
        "duplicate_ids_sample": duplicate_ids,
        "unexpected_ids_sample": unexpected_ids,
        "stale_ids_sample": stale_ids,
        "invalid_row_samples": invalid_rows,
    }


def write_report_and_retry_ids(
    *,
    report_file: Path | None,
    retry_ids_file: Path | None,
    output: Path,
    expected_ids: list[str],
    model: str,
    min_coverage_bps: int,
    expected_dimension: int | None,
    expected_model: str | None = None,
    expected_text_hashes: dict[str, str] | None = None,
) -> None:
    report = embedding_output_report(
        output,
        expected_ids,
        model=model,
        min_coverage_bps=min_coverage_bps,
        expected_dimension=expected_dimension,
        expected_model=expected_model,
        expected_text_hashes=expected_text_hashes,
    )
    if report_file is not None:
        report_file.parent.mkdir(parents=True, exist_ok=True)
        report_file.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if retry_ids_file is not None:
        missing = sorted(
            set(expected_ids)
            - load_done_ids(
                output,
                expected_dimension,
                expected_model=expected_model,
                expected_text_hashes=expected_text_hashes,
            )
        )
        retry_ids_file.parent.mkdir(parents=True, exist_ok=True)
        retry_ids_file.write_text("".join(f"{doc_id}\n" for doc_id in missing), encoding="utf-8")

