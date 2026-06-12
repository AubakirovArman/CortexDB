#!/usr/bin/env python3
"""Guard descriptor-first hot paths against legacy payload metadata parsing."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SERVER_AUTHZ = ROOT / "crates/cortex-server/src/authz.rs"
SERVER_ROUTER = ROOT / "crates/cortex-server/src/router.rs"
CONTEXT_DEDUP = ROOT / "crates/cortex-engine/src/context/dedup.rs"
CONTEXT_PACK = ROOT / "crates/cortex-engine/src/context/pack.rs"
SEARCH_DATABASE = ROOT / "crates/cortex-engine/src/search/database.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    server_authz = SERVER_AUTHZ.read_text()
    server_router = SERVER_ROUTER.read_text()
    context_dedup = CONTEXT_DEDUP.read_text()
    context_pack = CONTEXT_PACK.read_text()
    search_database = SEARCH_DATABASE.read_text()

    require(
        server_authz,
        "pub(crate) fn require_descriptor_write",
        "descriptor-based write authorization",
    )
    require(
        server_router,
        "let descriptor = CellDescriptor::from_payload_lossy(body);",
        "raw write boundary descriptor materialization",
    )
    require(
        server_router,
        "authz::require_descriptor_write(authenticated_view.as_ref(), &descriptor)?;",
        "raw write descriptor authorization",
    )
    forbid(server_authz, "require_payload_write", "payload-based write authorization helper")
    forbid(server_authz, "CellMetadata::from_payload", "payload parsing in server authz")
    forbid(server_router, "require_payload_write", "payload-based route authorization")

    require(
        context_dedup,
        "metadata: &CellMetadata",
        "ContextPack redundancy metadata parameter",
    )
    require(
        context_dedup,
        "pub(crate) fn term_set(metadata: &CellMetadata)",
        "ContextPack term extraction from metadata",
    )
    forbid(
        context_dedup,
        "CellMetadata::from_payload(",
        "ContextPack redundancy payload parsing",
    )
    require(
        context_pack,
        "weighted_jaccard_q16(&cell_body_terms, &term_set(&packed.metadata))",
        "ContextPack redundancy penalty from packed metadata",
    )
    forbid(
        context_pack,
        "term_set(&packed.payload)",
        "ContextPack redundancy penalty payload parsing",
    )

    require(
        search_database,
        "payload_jaccard_q16(&candidate.metadata, &existing.metadata)",
        "search diversity payload similarity from result metadata",
    )
    require(
        search_database,
        "fn payload_terms(metadata: &CellMetadata)",
        "search diversity term extraction from metadata",
    )
    forbid(
        search_database,
        "CellMetadata::from_payload(payload)",
        "search diversity payload parsing",
    )

    print("descriptor hot path gate passed")


if __name__ == "__main__":
    main()
