#!/usr/bin/env python3
"""Guard descriptor-first hot paths against legacy payload metadata parsing."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SERVER_AUTHZ = ROOT / "crates/cortex-server/src/authz.rs"
SERVER_ROUTER = ROOT / "crates/cortex-server/src/router/core_routes.rs"
SERVER_MEMORY_ROUTES = ROOT / "crates/cortex-server/src/router/memory_routes.rs"
SERVER_SEARCH = ROOT / "crates/cortex-server/src/search/rerank.rs"
CONTEXT_DEDUP = ROOT / "crates/cortex-engine/src/context/dedup.rs"
CONTEXT_PACK = ROOT / "crates/cortex-engine/src/context/pack/builder.rs"
SEARCH_DATABASE_FILES = (
    ROOT / "crates/cortex-engine/src/search/database/diversity.rs",
    ROOT / "crates/cortex-engine/src/search/database/ranking.rs",
)
SEARCH_RERANK_FILES = (
    ROOT / "crates/cortex-engine/src/search/rerank/types.rs",
    ROOT / "crates/cortex-engine/src/search/rerank/scoring.rs",
)
SEARCH_SCOPE_MAPPING = ROOT / "crates/cortex-engine/src/search/scope_mapping/scoring.rs"
DATABASE = ROOT / "crates/cortex-engine/src/database.rs"
RETRIEVAL_QUALITY = ROOT / "crates/cortex-engine/src/retrieval_quality.rs"
QUERY_EXPLAIN = ROOT / "crates/cortex-engine/src/query/explain.rs"
SESSION = ROOT / "crates/cortex-engine/src/session.rs"
SESSION_INDEX = ROOT / "crates/cortex-engine/src/session/index.rs"
SESSION_PAYLOAD = ROOT / "crates/cortex-engine/src/session/payload.rs"
INGESTION = ROOT / "crates/cortex-engine/src/ingestion.rs"
INGESTION_REPORT = ROOT / "crates/cortex-engine/src/ingestion/report.rs"
REPLICATION_SNAPSHOT = ROOT / "crates/cortex-engine/src/replication/snapshot.rs"
REPLICATION_INSTALL = ROOT / "crates/cortex-engine/src/replication/install.rs"
TOOL_REGISTRY_FILES = (
    ROOT / "crates/cortex-engine/src/tool_registry.rs",
    ROOT / "crates/cortex-engine/src/tool_registry/index.rs",
)
VERIFICATION = ROOT / "crates/cortex-engine/src/verification/evidence.rs"
VERIFICATION_OPERATOR_FILES = (ROOT / "crates/cortex-engine/src/verification/operator.rs", ROOT / "crates/cortex-engine/src/verification/operator/candidates.rs")
VERIFICATION_GRAPH = ROOT / "crates/cortex-engine/src/verification/graph.rs"
VERIFICATION_CONFLICT_INDEX = ROOT / "crates/cortex-engine/src/verification/conflict_index.rs"; VERIFICATION_CONFLICT_STORE = ROOT / "crates/cortex-engine/src/verification/conflict_index/store.rs"
VERIFICATION_TEMPORAL_INDEX = ROOT / "crates/cortex-engine/src/verification/temporal_index.rs"
VERIFICATION_TEMPORAL_STORE = ROOT / "crates/cortex-engine/src/verification/temporal_index/store.rs"
VERIFICATION_GUARDS = ROOT / "crates/cortex-engine/src/verification/guards.rs"


def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"missing {label}: {needle}")


def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"forbidden {label}: {needle}")


def main() -> None:
    server_authz = SERVER_AUTHZ.read_text()
    server_router = SERVER_ROUTER.read_text()
    server_memory_routes = SERVER_MEMORY_ROUTES.read_text()
    server_search = SERVER_SEARCH.read_text()
    context_dedup = CONTEXT_DEDUP.read_text()
    context_pack = CONTEXT_PACK.read_text()
    search_database = "\n".join(path.read_text() for path in SEARCH_DATABASE_FILES)
    search_rerank = "\n".join(path.read_text() for path in SEARCH_RERANK_FILES)
    search_scope_mapping = SEARCH_SCOPE_MAPPING.read_text()
    database = DATABASE.read_text()
    retrieval_quality = RETRIEVAL_QUALITY.read_text()
    query_explain = QUERY_EXPLAIN.read_text()
    session = SESSION.read_text()
    session_index = SESSION_INDEX.read_text() + "\n" + (ROOT / "crates/cortex-engine/src/session/index/record.rs").read_text()
    session_payload = SESSION_PAYLOAD.read_text()
    ingestion = INGESTION.read_text()
    ingestion_report = INGESTION_REPORT.read_text()
    replication_snapshot = REPLICATION_SNAPSHOT.read_text()
    replication_install = REPLICATION_INSTALL.read_text()
    tool_registry = "\n".join(path.read_text() for path in TOOL_REGISTRY_FILES)
    verification = VERIFICATION.read_text()
    verification_operator = "\n".join(path.read_text() for path in VERIFICATION_OPERATOR_FILES)
    verification_graph = VERIFICATION_GRAPH.read_text()
    verification_conflict_index = VERIFICATION_CONFLICT_INDEX.read_text(); verification_conflict_store = VERIFICATION_CONFLICT_STORE.read_text()
    verification_temporal_index = VERIFICATION_TEMPORAL_INDEX.read_text()
    verification_temporal_store = VERIFICATION_TEMPORAL_STORE.read_text()
    verification_guards = VERIFICATION_GUARDS.read_text()

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
        "authz::require_descriptor_write(authenticated_view, &descriptor)?;",
        "raw write descriptor authorization",
    )
    forbid(server_authz, "require_payload_write", "payload-based write authorization helper")
    forbid(server_authz, "CellMetadata::from_payload", "payload parsing in server authz")
    forbid(server_router, "require_payload_write", "payload-based route authorization")
    require(
        server_router,
        "db.get_latest_cell_descriptor(cell_id)",
        "server cell routes authorize from descriptor-only lookup before payload fetch",
    )
    require(
        server_router,
        "let cell = db.get_latest_cell(cell_id);",
        "server cell route fetches payload only after descriptor authorization",
    )
    require(
        server_router,
        ".or_else(|| db.get_latest_cell_descriptor(cell_id))",
        "server batch tombstone authorization uses descriptor-only lookup",
    )
    forbid(
        server_router,
        "get_latest_cell_with_descriptor",
        "server core routes pre-auth payload+descriptor lookup",
    )
    require(
        server_memory_routes,
        "db.get_latest_cell_descriptor(cell_id)",
        "server forget route uses descriptor-only lookup",
    )
    forbid(
        server_memory_routes,
        "get_latest_cell_with_descriptor",
        "server memory routes pre-auth payload+descriptor lookup",
    )
    require(
        server_search,
        "metadata: Some(&result.metadata)",
        "server weighted rerank receives descriptor-backed metadata",
    )

    require(
        retrieval_quality,
        "pub(crate) fn cell_version_meets_quality_thresholds",
        "descriptor-backed quality threshold helper",
    )
    forbid(
        retrieval_quality,
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
        "payload_q16: payload_jaccard_q16(&candidate.metadata, &existing.metadata)",
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
        search_rerank,
        "scope_mapping_metadata_bonus: u64",
        "rerank scope mapping multiplier is metadata-scoped",
    )
    forbid(
        search_rerank,
        "scope_mapping_payload_bonus",
        "rerank scope mapping payload fallback",
    )
    require(
        search_scope_mapping,
        "pub fn scope_mapping_metadata_bonus(mapping: &QueryScopeMapping, metadata: &CellMetadata)",
        "scope mapping metadata scoring helper",
    )
    for text, needle, label in (
        (session, "let mut cells = self.session_index.retrieve(", "session retrieval delegates to maintained index"),
        (session_index, "SessionMetadata::from_descriptor(descriptor)", "session index descriptor-backed metadata"),
        (session_index, "view.can_read_scope(scope_id(&self.descriptor.scope))", "session index descriptor scope authorization"),
        (session, "self.get_latest_cell_descriptor(cell_id).is_none()", "session id allocation uses descriptor-only existence check"),
        (ingestion, "self.get_latest_cell_descriptor(cell_id).is_none()", "memory id allocation uses descriptor-only existence check"),
    ):
        require(text, needle, label)
    for text, needle, label in (
        (session, "snapshot_versions()", "session retrieval full snapshot scan"),
        (session, "view.can_read_scope(scope_id(&metadata.scope))", "session retrieval payload scope authorization"),
        (session_index, "CellMetadata::from_payload_with_descriptor(", "session index payload metadata parsing"),
        (session_payload, "pub scope: String", "session payload scope permission metadata"),
        (session, "self.get_latest_cell(cell_id).is_none()", "session id allocation payload existence check"),
        (ingestion, "self.get_latest_cell(cell_id).is_none()", "memory id allocation payload existence check"),
    ):
        forbid(text, needle, label)
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
    require(
        tool_registry,
        "let descriptor = ToolDescriptor::from_version(version).ok()?;",
        "tool registry list uses CellVersion descriptor",
    )
    require(
        tool_registry,
        "view.can_read_scope(scope_id(&tool.descriptor.scope))",
        "tool registry permission check uses descriptor scope",
    )
    require(
        verification,
        "let metadata = CellMetadata::from_version(version);",
        "verification evidence uses CellVersion descriptor",
    )
    require(
        verification,
        "if !view.can_read_scope(scope_id(&metadata.scope))",
        "verification permission check uses descriptor metadata",
    )
    require(
        verification_operator,
        "if !view.can_read_scope(scope_id(&metadata.scope))",
        "verification source-support scan checks descriptor scope before payload",
    )
    require(
        verification_operator,
        "let payload = self.payload_for_version(version)?;",
        "verification source-support scan materializes payload after descriptor checks",
    )
    require(
        verification_graph,
        "let metadata = CellMetadata::from_payload_with_descriptor(payload, &version.descriptor);",
        "verification graph relation uses CellVersion descriptor",
    )
    require(
        verification_graph,
        "CellMetadata::from_payload_with_descriptor(&payload, &descriptor)",
        "verification graph persisted edge uses descriptor-backed metadata",
    )
    require(
        verification_graph,
        "let descriptor = db.get_latest_cell_descriptor(edge.relation_cell_id)?;",
        "verification graph persisted edge checks descriptor before payload",
    )
    forbid(
        verification_graph,
        "let (payload, descriptor) = db.get_latest_cell_with_descriptor(edge.relation_cell_id)?;",
        "verification graph persisted edge pre-auth payload+descriptor lookup",
    )
    require(
        verification_conflict_store,
        "let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);",
        "conflict index uses CellVersion descriptor",
    )
    require(
        verification_temporal_index,
        "self.temporal_fact_store.fact_index(view)",
        "temporal index delegates to maintained store",
    )
    require(
        verification_temporal_store,
        "let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);",
        "temporal store uses descriptor-backed metadata",
    )
    forbid(
        verification_temporal_index,
        "snapshot_versions()",
        "temporal index full snapshot scan",
    )
    require(
        verification_guards,
        "let metadata = CellMetadata::from_version(version);",
        "verification guards use CellVersion descriptor",
    )
    require(
        replication_snapshot,
        "pub struct SnapshotCell",
        "replication snapshot descriptor-aware cell model",
    )
    require(
        replication_snapshot,
        "pub descriptor: Option<Vec<u8>>",
        "replication snapshot descriptor bytes",
    )
    require(
        replication_snapshot,
        "const SNAPSHOT_SEGMENT_MAGIC: &[u8; 4] = b\"CSP2\";",
        "replication snapshot descriptor format version",
    )
    require(
        replication_install,
        "SegmentWriter::write_refs",
        "replication snapshot install descriptor-aware segment write",
    )
    require(
        replication_install,
        "EngineAqlIndex::try_from_segment_cell_refs(&cell_refs)",
        "replication snapshot install descriptor-backed AQL index",
    )
    require(
        replication_install,
        "memtable.put_cell_with_descriptor",
        "replication snapshot install descriptor-backed memtable",
    )
    forbid(
        replication_install,
        "SegmentWriter::write(\n            segment_path",
        "replication snapshot install payload-only segment write",
    )
    forbid(
        replication_install,
        "EngineAqlIndex::try_from_segment_cells(&snapshot.cells)",
        "replication snapshot install payload-only AQL index",
    )

    print("descriptor hot path gate passed")


if __name__ == "__main__":
    main()
