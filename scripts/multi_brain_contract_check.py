#!/usr/bin/env python3
"""Guard the B20 single-brain/deprecated-alias contract."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    brain_doc = (ROOT / "docs/BRAIN_SEMANTICS.md").read_text()
    data_model = (ROOT / "docs/DATA_MODEL.md").read_text()
    aql_doc = (ROOT / "docs/AQL_V0_5.md").read_text()
    query_brain = (ROOT / "crates/cortex-engine/src/query/brain.rs").read_text()
    query_catalog = (ROOT / "crates/cortex-engine/src/query/catalog.rs").read_text()
    stats_catalog = (ROOT / "crates/cortex-engine/src/query/statistics.rs").read_text()

    for text, label in (
        (brain_doc, "brain semantics doc"),
        (data_model, "data model"),
        (aql_doc, "AQL v0.5 doc"),
    ):
        require(text, "BrainId(1)", f"{label} names default brain id")
        require(text, "deprecated aliases", f"{label} documents deprecated aliases")

    require(
        query_brain,
        "pub(crate) const DEFAULT_BRAIN: BrainId = BrainId(1);",
        "single default brain constant",
    )
    require(
        query_brain,
        "resolve_single_brain_name",
        "single-brain resolver helper",
    )
    require(
        query_catalog,
        "resolve_single_brain_name(name)",
        "runtime AQL catalog uses single-brain resolver",
    )
    require(
        stats_catalog,
        "resolve_single_brain_name(name)",
        "statistics AQL catalog uses single-brain resolver",
    )
    forbid(
        query_catalog,
        "fn resolve_brain(&self, _name: &str)",
        "anonymous always-default brain resolver",
    )
    print("multi-brain contract gate passed")


if __name__ == "__main__":
    main()
