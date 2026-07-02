#!/usr/bin/env python3
"""Frozen ranking-weights gate for CortexDB."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

from ranking_frozen_weights_gate_spec import (
    BANNED_MARKERS,
    CONSUMER_MARKERS,
    FIXTURE_SCHEMA,
    FIXTURE_VERSION,
    MODULE_PATH,
    PROFILE_CONSTS,
    PROFILE_FIELDS,
    ROUTE_CONSTS,
    ROUTE_FIELDS,
    SCHEMA_VERSION,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=".", help="repository root")
    parser.add_argument("--fixture", required=True, help="frozen weights fixture")
    parser.add_argument("--report", required=True, help="output report path")
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def rust_int(value: str) -> int:
    return int(value.strip().replace("_", ""))


def const_values(module: str) -> dict[str, int | str]:
    values: dict[str, int | str] = {}
    for name, raw in re.findall(r"pub const ([A-Z0-9_]+): [^=]+ = ([^;]+);", module):
        raw = raw.strip()
        if raw.startswith('"') and raw.endswith('"'):
            values[name] = raw.strip('"')
        elif re.fullmatch(r"\d[\d_]*", raw):
            values[name] = rust_int(raw)
    return values


def resolve_expr(expr: str, constants: dict[str, int | str]) -> int | str | None:
    expr = expr.strip().rstrip(",")
    if expr == "None":
        return None
    if match := re.fullmatch(r"Some\((\d[\d_]*)\)", expr):
        return rust_int(match.group(1))
    if re.fullmatch(r"\d[\d_]*", expr):
        return rust_int(expr)
    return constants.get(expr)


def struct_body(module: str, const_name: str, struct_name: str) -> str | None:
    pattern = (
        rf"pub const {re.escape(const_name)}: {re.escape(struct_name)} = "
        rf"{re.escape(struct_name)} \{{(?P<body>.*?)\}};"
    )
    match = re.search(pattern, module, re.S)
    return match.group("body") if match else None


def struct_field(body: str, field: str) -> str | None:
    match = re.search(rf"{re.escape(field)}: ([^,\n]+)", body)
    return match.group(1).strip() if match else None


def expected_constants(fixture: dict[str, Any]) -> dict[str, int | str]:
    constants: dict[str, int | str] = {
        "VERSION": fixture["version"],
        "Q16_ONE_U32": fixture["q16_one"],
        "Q16_ONE_U64": fixture["q16_one"],
        "Q16_SCALE_U32": fixture["q16_scale"],
        "Q16_SCALE_I128": fixture["q16_scale"],
        "CONTEXT_REDUNDANCY_PENALTY_WEIGHT": fixture["context_pack"][
            "redundancy_penalty_weight"
        ],
        "RERANK_MIN_CANDIDATE_LIMIT": fixture["rrf"]["min_candidate_limit"],
    }
    add_section_constants(constants, "VALUE_PER_TOKEN", fixture["value_per_token"])
    add_section_constants(constants, "QUERY", fixture["query_terms"])
    add_section_constants(constants, "RRF", fixture["rrf"])
    add_section_constants(constants, "RERANK_DEFAULT", fixture["reranker_default"])
    add_metadata_constants(constants, fixture["metadata_rerank"])
    return constants


def add_section_constants(
    constants: dict[str, int | str], prefix: str, values: dict[str, int]
) -> None:
    reranker_aliases = {
        "requirement_payload_bonus": "RERANK_REQUIREMENT_PAYLOAD_BONUS",
        "term_payload_bonus": "RERANK_TERM_PAYLOAD_BONUS",
        "evidence_overlap_threshold": "EVIDENCE_OVERLAP_THRESHOLD",
        "evidence_anchor_points": "EVIDENCE_ANCHOR_POINTS",
        "evidence_source_points": "EVIDENCE_SOURCE_POINTS",
        "evidence_condition_points": "EVIDENCE_CONDITION_POINTS",
        "evidence_requirement_points": "EVIDENCE_REQUIREMENT_POINTS",
        "evidence_term_points": "EVIDENCE_TERM_POINTS",
    }
    for key, value in values.items():
        if key == "min_candidate_limit":
            continue
        name = reranker_aliases.get(key, f"{prefix}_{key.upper()}")
        if name == "RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_SCORE_Q16":
            name = "RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16"
        constants[name] = value


def add_metadata_constants(constants: dict[str, int | str], values: dict[str, Any]) -> None:
    constants["METADATA_RERANK_SCALE"] = values["scale"]
    for name, key in {
        "CONFLICTING_INFO": "conflicting_info",
        "CONSTRAINED_TEMPORAL": "constrained_temporal",
        "TEMPORAL": "temporal",
        "INFO_NOT_FOUND": "info_not_found",
        "DEFAULT": "default",
    }.items():
        constants[f"METADATA_{name}_TRUST_Q16"] = values[key][0]
        constants[f"METADATA_{name}_FRESHNESS_Q16"] = values[key][1]


def check_fixture(fixture: dict[str, Any], errors: list[str]) -> None:
    if fixture.get("schema_version") != FIXTURE_SCHEMA:
        errors.append("fixture: unexpected schema_version")
    if fixture.get("version") != FIXTURE_VERSION:
        errors.append("fixture: unexpected version")
    if fixture.get("q16_one") != 65_535 or fixture.get("q16_scale") != 65_536:
        errors.append("fixture: q16 constants must be frozen to 65535/65536")


def check_module(root: Path, fixture: dict[str, Any], errors: list[str]) -> None:
    module = (root / MODULE_PATH).read_text(encoding="utf-8")
    values = const_values(module)
    for name, expected in expected_constants(fixture).items():
        if values.get(name) != expected:
            errors.append(f"{MODULE_PATH}: {name} expected {expected!r}, got {values.get(name)!r}")

    check_structs(module, values, fixture["calibration"], PROFILE_CONSTS, PROFILE_FIELDS, errors)
    check_structs(module, values, fixture["route_policies"], ROUTE_CONSTS, ROUTE_FIELDS, errors)


def check_structs(
    module: str,
    constants: dict[str, int | str],
    expected: dict[str, list[Any]],
    names: dict[str, str],
    fields: list[str],
    errors: list[str],
) -> None:
    struct_name = "FrozenRerankProfile" if fields == PROFILE_FIELDS else "FrozenRoutePolicy"
    for key, const_name in names.items():
        body = struct_body(module, const_name, struct_name)
        if body is None:
            errors.append(f"{MODULE_PATH}: missing {const_name}")
            continue
        for field, expected_value in zip(fields, expected[key]):
            actual = resolve_expr(struct_field(body, field) or "", constants)
            if actual != expected_value:
                errors.append(
                    f"{MODULE_PATH}: {const_name}.{field} expected "
                    f"{expected_value!r}, got {actual!r}"
                )


def check_consumers(root: Path, errors: list[str]) -> None:
    for rel_path, markers in CONSUMER_MARKERS.items():
        text = (root / rel_path).read_text(encoding="utf-8")
        for marker in markers:
            if marker not in text:
                errors.append(f"{rel_path}: missing marker {marker!r}")
    for rel_path, markers in BANNED_MARKERS.items():
        text = (root / rel_path).read_text(encoding="utf-8")
        for marker in markers:
            if marker in text:
                errors.append(f"{rel_path}: banned bare ranking marker {marker!r}")


def write_report(path: Path, errors: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": SCHEMA_VERSION,
        "fixture_version": FIXTURE_VERSION,
        "status": "passed" if not errors else "failed",
        "errors": errors,
        "checked_consumers": sorted(CONSUMER_MARKERS),
    }
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    errors: list[str] = []
    fixture = read_json(root / args.fixture)
    check_fixture(fixture, errors)
    check_module(root, fixture, errors)
    check_consumers(root, errors)
    write_report(root / args.report, errors)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"ranking frozen weights check passed: {args.report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
