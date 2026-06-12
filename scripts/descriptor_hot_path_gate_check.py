#!/usr/bin/env python3
"""Guard descriptor-first hot paths against legacy payload metadata parsing."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SERVER_AUTHZ = ROOT / "crates/cortex-server/src/authz.rs"
SERVER_ROUTER = ROOT / "crates/cortex-server/src/router.rs"
SERVER_SEARCH = ROOT / "crates/cortex-server/src/search.rs"
CONTEXT_DEDUP = ROOT / "crates/cortex-engine/src/context/dedup.rs"
CONTEXT_PACK = ROOT / "crates/cortex-engine/src/context/pack.rs"
SEARCH_DATABASE = ROOT / "crates/cortex-engine/src/search/database.rs"
SEARCH_RERANK = ROOT / "crates/cortex-engine/src/search/rerank.rs"
SEARCH_SCOPE_MAPPING = ROOT / "crates/cortex-engine/src/search/scope_mapping.rs"
DATABASE = ROOT / "crates/cortex-engine/src/database.rs"
QUERY_EXPLAIN = ROOT / "crates/cortex-engine/src/query/explain.rs"
SESSION = ROOT / "crates/cortex-engine/src/session.rs"
SESSION_PAYLOAD = ROOT / "crates/cortex-engine/src/session/payload.rs"
INGESTION_REPORT = ROOT / "crates/cortex-engine/src/ingestion/report.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    server_authz = SERVER_AUTHZ.read_text()
    server_router = SERVER_ROUTER.read_text()
    server_search = SERVER_SEARCH.read_text()
    context_dedup = CONTEXT_DEDUP.read_text()
    context_pack = CONTEXT_PACK.read_text()
    search_database = SEARCH_DATABASE.read_text()
    search_rerank = SEARCH_RERANK.read_text()
    search_scope_mapping = SEARCH_SCOPE_MAPPING.read_text()
    database = DATABASE.read_text()
    query_explain = QUERY_EXPLAIN.read_text()
    session = SESSION.read_text()
    session_payload = SESSION_PAYLOAD.read_text()
    ingestion_report = INGESTION_REPORT.read_text()

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
        server_search,
        "metadata: Some(&result.metadata)",
        "server weighted rerank receives descriptor-backed metadata",
    )

    require(
        database,
        "pub(crate) fn cell_version_meets_quality_thresholds",
        "descriptor-backed quality threshold helper",
    )
    forbid(
        database,
        "pub(crate) fn cell_meets_quality_thresholds",
        "payload-only quality threshold helper",
    )
    require(
        query_explain,
        "cell_version_meets_quality_thresholds(version, &plan.quality_thresholds)",
        "EXPLAIN quality count from CellVersion descriptor",
    )
    forbid(
        query_explain,
        "cell_meets_quality_thresholds",
        "EXPLAIN payload-only quality threshold helper",
    )

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
    require(
        search_database,
        "metadata: Some(&result.metadata)",
        "production search rerank receives descriptor-backed metadata",
    )
    forbid(
        search_database,
        "CellMetadata::from_payload(payload)",
        "search diversity payload parsing",
    )
    require(
        search_rerank,
        "pub metadata: Option<&'a CellMetadata>",
        "rerank input descriptor-backed metadata",
    )
    require(
        search_rerank,
        "scope_mapping_metadata_bonus(&scope_mapping, metadata)",
        "rerank scope mapping from metadata when available",
    )
    require(
        search_scope_mapping,
        "pub fn scope_mapping_metadata_bonus(mapping: &QueryScopeMapping, metadata: &CellMetadata)",
        "scope mapping metadata scoring helper",
    )
    require(
        session,
        "let descriptor_metadata = CellMetadata::from_version(&version);",
        "session retrieval descriptor metadata",
    )
    require(
        session,
        "view.can_read_scope(scope_id(&descriptor_metadata.scope))",
        "session retrieval descriptor scope authorization",
    )
    forbid(
        session,
        "view.can_read_scope(scope_id(&metadata.scope))",
        "session retrieval payload scope authorization",
    )
    forbid(
        session_payload,
        "pub scope: String",
        "session payload scope permission metadata",
    )
    require(
        ingestion_report,
        "self.get_latest_cell_with_descriptor(cell.cell_id)",
        "ingestion validation descriptor read",
    )
    require(
        ingestion_report,
        "CellMetadata::from_payload_with_descriptor(&payload, &descriptor)",
        "ingestion validation descriptor-backed metadata",
    )
    forbid(
        ingestion_report,
        "CellMetadata::from_payload(&payload)",
        "ingestion validation payload-only metadata",
    )

    print("descriptor hot path gate passed")


if __name__ == "__main__":
    main()
