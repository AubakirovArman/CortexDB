#!/usr/bin/env python3
"""Inventory typed provenance coverage for EPIC-B06."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Check:
    name: str
    status: str
    evidence: list[str]
    gap: str | None = None

    def to_json(self) -> dict[str, object]:
        value: dict[str, object] = {
            "name": self.name,
            "status": self.status,
            "evidence": self.evidence,
        }
        if self.gap:
            value["gap"] = self.gap
        return value


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def line_refs(path: str, needles: list[str]) -> list[str]:
    refs: list[str] = []
    for index, line in enumerate(read(path).splitlines(), start=1):
        if any(needle in line for needle in needles):
            refs.append(f"{path}:{index}")
    return refs


def has_all(text: str, needles: list[str]) -> bool:
    return all(needle in text for needle in needles)


def check_descriptor_fields() -> Check:
    path = "crates/cortex-core/src/cell/descriptor.rs"
    text = read(path)
    fields = ["source_trust_q16", "source:", "citation:", "content_hash:"]
    status = "pass" if has_all(text, fields) else "fail"
    return Check(
        "core_descriptor_typed_provenance_fields",
        status,
        line_refs(path, fields),
        None if status == "pass" else "CellDescriptor lacks typed source/trust/citation/hash fields.",
    )


def check_descriptor_source_ref_boundary() -> Check:
    path = "crates/cortex-core/src/cell/descriptor.rs"
    text = read(path)
    fields = ["source_id", "source_url", "document_id", "cell_range", "json_path", "confidence_q16"]
    present = [field for field in fields if field in text]
    if len(present) == len(fields):
        return Check("core_descriptor_typed_source_ref_boundary", "pass", line_refs(path, fields))
    return Check(
        "core_descriptor_typed_source_ref_boundary",
        "fail",
        line_refs(path, present),
        "SourceRef is still not encoded as first-class descriptor/WAL fields.",
    )


def check_core_metadata() -> Check:
    path = "crates/cortex-core/src/cell/metadata.rs"
    text = read(path)
    required = ["source_trust_q16", "source:"]
    missing = ["citation", "content_hash", "source_id", "document_id"]
    if has_all(text, required) and not any(value in text for value in missing):
        return Check(
            "knowledge_cell_metadata_provenance_boundary",
            "partial",
            line_refs(path, required),
            "KnowledgeCellMetadata writes source/trust but not citation/content_hash/source_ref yet.",
        )
    status = "pass" if has_all(text, required + missing) else "partial"
    return Check("knowledge_cell_metadata_provenance_boundary", status, line_refs(path, required + missing))


def check_engine_source_ref() -> Check:
    paths = [
        "crates/cortex-engine/src/query/metadata/types.rs",
        "crates/cortex-engine/src/query/metadata/parser.rs",
    ]
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        needles = ["SourceRef", "source_id", "document_id", "confidence_q16"]
        ok = ok and has_all(text, needles)
        evidence.extend(line_refs(path, needles))
    return Check(
        "engine_metadata_source_ref_model",
        "pass" if ok else "fail",
        evidence,
        None if ok else "Engine metadata does not expose full SourceRef parsing.",
    )


def check_ingestion_hashes() -> Check:
    paths = [
        "crates/cortex-engine/src/ingestion/cells.rs",
        "crates/cortex-engine/src/ingestion/adapters.rs",
        "crates/cortex-engine/src/ingestion/dedup.rs",
    ]
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        needles = ["content_hash", "source_hash", "stable_ingestion_hash_hex"]
        ok = ok and any(needle in text for needle in needles)
        evidence.extend(line_refs(path, needles))
    return Check(
        "ingestion_sets_content_hash",
        "pass" if ok else "fail",
        evidence,
        None if ok else "Ingestion path does not consistently compute provenance hashes.",
    )


def check_contextpack_citations() -> Check:
    paths = [
        "crates/cortex-engine/src/context/pack/builder.rs",
        "crates/cortex-engine/src/context/export/json_export.rs",
        "crates/cortex-engine/src/context/export/prompt.rs",
        "crates/cortex-engine/src/context/export/markdown.rs",
    ]
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        needles = ["citation", "source_ref"]
        ok = ok and has_all(text, needles)
        evidence.extend(line_refs(path, needles))
    return Check(
        "contextpack_exports_typed_provenance",
        "pass" if ok else "fail",
        evidence,
        None if ok else "ContextPack export path does not consistently expose citation/source_ref.",
    )


def check_content_hash_dedup() -> Check:
    paths = [
        "crates/cortex-engine/src/retrieval_rank.rs",
        "crates/cortex-engine/src/search/database/diversity.rs",
        "crates/cortex-engine/src/ingestion/dedup.rs",
    ]
    evidence: list[str] = []
    ok = True
    for path in paths:
        text = read(path)
        ok = ok and "content_hash" in text
        evidence.extend(line_refs(path, ["content_hash"]))
    return Check(
        "dedup_uses_content_hash",
        "pass" if ok else "fail",
        evidence,
        None if ok else "Dedup/diversity path does not use content_hash.",
    )


def inventory() -> dict[str, object]:
    checks = [
        check_descriptor_fields(),
        check_descriptor_source_ref_boundary(),
        check_core_metadata(),
        check_engine_source_ref(),
        check_ingestion_hashes(),
        check_contextpack_citations(),
        check_content_hash_dedup(),
    ]
    gaps = [check.gap for check in checks if check.gap]
    failing = [check.name for check in checks if check.status == "fail"]
    partial = [check.name for check in checks if check.status == "partial"]
    status = "complete" if not failing and not partial else "partial"
    return {
        "schema_version": "cortexdb.provenance_model_inventory.v1",
        "epic": "EPIC-B06",
        "status": status,
        "summary": {
            "checks": len(checks),
            "pass": sum(1 for check in checks if check.status == "pass"),
            "partial": len(partial),
            "fail": len(failing),
        },
        "checks": [check.to_json() for check in checks],
        "remaining_gaps": gaps,
        "next_patch_order": [
            "Add core SourceRef/ProvenanceDescriptor fields or a nested wire section.",
            "Make write/ingestion paths fill descriptor-backed citation/hash/source_ref values.",
            "Add no-payload-parse ContextPack citation regression tests.",
            "Document descriptor-backed provenance in DATA_MODEL.md.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/provenance-model/inventory.json")
    args = parser.parse_args()
    output = inventory()
    report = Path(args.report)
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = output["summary"]
    print(
        "provenance inventory: "
        f"status={output['status']} pass={summary['pass']} "
        f"partial={summary['partial']} fail={summary['fail']}"
    )
    print(f"report: {report}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
