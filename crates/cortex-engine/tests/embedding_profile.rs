//! A2.1: embedding-profile provenance. A store records the embedding profile
//! (model/dimension/metric) it was built with, and reopening it with an
//! incompatible profile fails closed. All hermetic (offline
//! `DeterministicTestEmbedder`); the engine performs no network I/O.

use cortex_core::CellId;
use cortex_engine::{
    Database, DatabaseOptions, DeterministicTestEmbedder, EmbeddingProfile, IngestionUpdatePolicy,
    TextChunkPolicy, TextIngestOptions,
};

const DIM: usize = 16;

fn profile(model: &str, dimension: u32, metric: u32) -> EmbeddingProfile {
    EmbeddingProfile {
        model: model.to_owned(),
        dimension,
        metric,
    }
}

fn options_with(profile: Option<EmbeddingProfile>) -> DatabaseOptions {
    DatabaseOptions {
        embedding_profile: profile,
        ..DatabaseOptions::default()
    }
}

fn opts(scope: &str, source: &str) -> TextIngestOptions {
    TextIngestOptions {
        scope: scope.to_owned(),
        source: source.to_owned(),
    }
}

/// Opens a store, embeds one chunk with the configured profile, checkpoints so
/// the manifest is stamped, and closes. Returns the store path's tempdir.
fn build_embedded_store(dir: &std::path::Path, model: &str) {
    let mut db =
        Database::open_with_options(dir, options_with(Some(profile(model, DIM as u32, 0))))
            .unwrap();
    let embedder = DeterministicTestEmbedder::new(DIM);
    db.ingest_text_chunks_with_embedder(
        CellId(1_000),
        "solar plant capital budget approved for 2025",
        opts("project:embed", "doc-1"),
        TextChunkPolicy::default(),
        IngestionUpdatePolicy::AlwaysInsert,
        &embedder,
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.close().unwrap();
}

#[test]
fn checkpoint_stamps_manifest_embedding_profile() {
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");

    let db = Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0))),
    )
    .unwrap();
    let stamped = db
        .manifest()
        .embedding_profile
        .clone()
        .expect("profile stamped");
    assert_eq!(stamped.model, "det-16");
    assert_eq!(stamped.dimension, DIM as u32);
    assert_eq!(stamped.metric, 0);
}

#[test]
fn reopen_with_matching_profile_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");
    assert!(Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0)))
    )
    .is_ok());
}

#[test]
fn reopen_with_mismatched_model_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");
    let error = Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16-v2", DIM as u32, 0))),
    )
    .expect_err("model mismatch must fail closed");
    assert!(
        format!("{error:?}").contains("embedding profile"),
        "unexpected error: {error:?}"
    );
}

#[test]
fn reopen_with_mismatched_dimension_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");
    assert!(
        Database::open_with_options(dir.path(), options_with(Some(profile("det-16", 8, 0))))
            .is_err()
    );
}

#[test]
fn reopen_with_mismatched_metric_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");
    assert!(Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 1)))
    )
    .is_err());
}

#[test]
fn reopen_without_configured_profile_succeeds() {
    // A store with a recorded profile still opens when the caller configures
    // none (permissive posture O2): benches/tooling/restore reopen unaffected.
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");
    assert!(Database::open_with_options(dir.path(), options_with(None)).is_ok());
}

#[test]
fn empty_store_with_configured_profile_opens() {
    // No live segments yet -> nothing to compare -> opens freely.
    let dir = tempfile::tempdir().unwrap();
    assert!(Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0)))
    )
    .is_ok());
}

#[test]
fn unlabelled_store_records_no_profile_and_reopens() {
    // Embeddings written without a configured model: no manifest profile is
    // stamped, so the store carries vectors but stays open to any profile.
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open_with_options(dir.path(), options_with(None)).unwrap();
        let embedder = DeterministicTestEmbedder::new(DIM);
        db.ingest_text_chunks_with_embedder(
            CellId(1_000),
            "wind farm operating budget rejected",
            opts("project:embed", "doc-2"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();
        db.checkpoint().unwrap();
        assert_eq!(db.manifest().embedding_profile, None);
        db.close().unwrap();
    }
    // Reopening with a profile is allowed because the store recorded none.
    assert!(Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0)))
    )
    .is_ok());
}

#[test]
fn rebuild_without_model_drops_stale_profile() {
    // Regression: a store rebuilt (compacted) without a configured model, after
    // its embedded cells are gone, must NOT keep claiming the old profile —
    // otherwise a later open with the old profile would pass on a store whose
    // vectors no longer match it.
    let dir = tempfile::tempdir().unwrap();

    // 1) Build with model A + one embedded cell AND one plain (un-embedded) cell.
    {
        let mut db = Database::open_with_options(
            dir.path(),
            options_with(Some(profile("det-16", DIM as u32, 0))),
        )
        .unwrap();
        let embedder = DeterministicTestEmbedder::new(DIM);
        db.ingest_text_chunks_with_embedder(
            CellId(1_000),
            "solar plant capital budget approved",
            opts("project:embed", "doc-1"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();
        db.put_cell(
            CellId(2_000),
            b"scope=project:embed\nstatus=ready\n\nplain fact".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        assert_eq!(
            db.manifest()
                .embedding_profile
                .as_ref()
                .map(|profile| profile.model.as_str()),
            Some("det-16")
        );
        db.close().unwrap();
    }

    // 2) Reopen with NO model configured, remove the embedded cell, compact:
    //    no vectors remain, so the stale profile must be cleared.
    {
        let mut db = Database::open_with_options(dir.path(), options_with(None)).unwrap();
        db.tombstone_cell(CellId(1_000)).unwrap();
        db.compact().unwrap();
        assert_eq!(
            db.manifest().embedding_profile,
            None,
            "stale profile must be cleared once its vectors are gone"
        );
        db.close().unwrap();
    }

    // 3) Reopening with the original profile is permissive (store is now
    //    honestly unlabelled) — it can never masquerade as still holding
    //    det-16 vectors.
    assert!(Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0)))
    )
    .is_ok());
}

#[test]
fn inconsistent_recorded_profiles_fail_closed_on_open() {
    // Defense in depth: if a manifest ends up with an embedding profile whose
    // dimension disagrees with the vector profile (e.g. an out-of-band segment
    // install), open fails closed instead of serving a mismatched vector space.
    let dir = tempfile::tempdir().unwrap();
    build_embedded_store(dir.path(), "det-16");

    // Patch the manifest's VECM dimension (16 -> 32) while EMBD stays 16.
    let manifest_path = dir.path().join("manifest.acm");
    let mut bytes = std::fs::read(&manifest_path).unwrap();
    let body_len = bytes.len() - 4;
    let vecm = bytes
        .windows(4)
        .position(|window| window == b"VECM")
        .expect("VECM section present");
    bytes[vecm + 4..vecm + 8].copy_from_slice(&32u32.to_le_bytes());
    bytes.truncate(body_len);
    let checksum = crc32c::crc32c(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    std::fs::write(&manifest_path, bytes).unwrap();

    let error = Database::open_with_options(dir.path(), options_with(None))
        .expect_err("inconsistent recorded profiles must fail closed");
    assert!(
        format!("{error:?}").contains("inconsistent"),
        "unexpected error: {error:?}"
    );
}

#[test]
fn ingest_writes_embedding_ref_header() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(
        dir.path(),
        options_with(Some(profile("det-16", DIM as u32, 0))),
    )
    .unwrap();
    let embedder = DeterministicTestEmbedder::new(DIM);
    let ingested = db
        .ingest_text_chunks_with_embedder(
            CellId(1_000),
            "solar plant capital budget approved for 2025",
            opts("project:embed", "doc-1"),
            TextChunkPolicy::default(),
            IngestionUpdatePolicy::AlwaysInsert,
            &embedder,
        )
        .unwrap();

    let payload = db.get_latest_cell(ingested[0].cell_id).unwrap();
    let text = String::from_utf8_lossy(&payload);
    let ref_line = text
        .lines()
        .find(|line| line.starts_with("embedding_ref="))
        .expect("embedding_ref header missing");
    // emb1:<model>:<dim>:<metric>:<content_hash>
    assert!(ref_line.starts_with("embedding_ref=emb1:det-16:16:dot_product:"));
    assert!(text.lines().any(|line| line == "embedding_model=det-16"));
}
